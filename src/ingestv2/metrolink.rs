use redis::Commands;
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};
use prost::Message;
use chrono::prelude::*;
use chrono_tz::US::Pacific;
use gtfs_rt::FeedMessage;
use kactus::insert::insert_gtfs_rt_bytes;
use regex::Regex;
use kactus::parse_protobuf_message;
use std::fs;

use kactus::aspen::send_to_aspen;

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[tokio::main]
async fn main() {
    let metrolink_key = fs::read_to_string("./metrolink-keys.txt")
        .expect("Unable to read file metrolink-keys.txt")
        .trim()
        .to_string();

    let arguments = std::env::args();
    let _arguments = arguments::parse(arguments).unwrap();

    let client = ReqwestClient::new();
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let _con = redisclient.get_connection().unwrap();

    let mut last_veh_attempt: Option<Instant> = None;
    let mut last_trip_attempt: Option<Instant> = None;

    let mut last_veh_protobuf_timestamp: Option<u128> = None;
    let mut last_trip_protobuf_timestamp: Option<u128> = None;

    loop {
        //should the vehicle section run?
        let veh_run: bool =
            determine_if_category_should_run(&last_veh_attempt, &last_veh_protobuf_timestamp);
        let trip_run: bool =
            determine_if_category_should_run(&last_trip_attempt, &last_trip_protobuf_timestamp);

        if veh_run || trip_run {
            let metrolink_results = futures::join!(
                runcategory(
                    &client,
                    &metrolink_key,
                    "vehicles",
                    &mut last_veh_attempt,
                    &mut last_veh_protobuf_timestamp,
                ),
                runcategory(
                    &client,
                    &metrolink_key,
                    "trips",
                    &mut last_trip_attempt,
                    &mut last_trip_protobuf_timestamp,
                ),
                get_metrolink_alerts(&client)
            );

            let mut alerts_con = redisclient.get_connection().unwrap();

            if metrolink_results.2.is_some() {
                println!("Alerts {} bytes", metrolink_results.2.as_ref().unwrap().len());
            insert_gtfs_rt_bytes(
                &mut alerts_con,
                &metrolink_results.2.as_ref().unwrap(),
                &"f-metrolinktrains~rt",
                &("alerts".to_string()),
            );
            } else {
                println!("Alerts crashed, skipping");
            }


            send_to_aspen(
                "f-metrolinktrains~rt",
                &metrolink_results.0,
                &metrolink_results.1,
                &metrolink_results.2,
                true,
                true,
                true,
                false,
            )
            .await;
        }

        //added this section because thread looping apparently consumes the whole core
        //20% cpu usage on the crappy Intel NUC this program executes on

        let instant_comp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Back to 1969?!?!!!");
        if last_veh_protobuf_timestamp.is_some() && last_trip_protobuf_timestamp.is_some() {
            if instant_comp.as_millis() - (last_veh_protobuf_timestamp.unwrap() * 1000) < 57_000
                && instant_comp.as_millis() - (last_trip_protobuf_timestamp.unwrap() * 1000)
                    < 57_000
            {
                let sleep_for = (std::cmp::min(
                    last_veh_protobuf_timestamp.unwrap(),
                    last_trip_protobuf_timestamp.unwrap(),
                ) + 57)
                    - instant_comp.as_secs() as u128;

                if sleep_for > 1 {
                    println!("Sleeping for {}s", sleep_for);

                    std::thread::sleep(Duration::from_secs(sleep_for.try_into().unwrap()));
                }
            }
        }
    }
}
#[derive(serde::Deserialize, Debug, Clone)]
struct MetrolinkAlertsResults {
    #[serde(rename="ServiceLines")]
    service_lines: Vec<String>,
    #[serde(rename="Advisories")]
    advisories: Vec<MetrolinkAlertsAdvisories>
}

#[derive(serde::Deserialize, Debug, Clone)]
struct MetrolinkAlertsAdvisories {
    #[serde(rename="Line")]
    line: String,
    #[serde(rename="LineAbbreviation")]
    line_abbreviation: String,
    #[serde(rename="ServiceAdvisories")]
    service_advisories: Vec<MetrolinkAlertsServiceAdvisories>
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct MetrolinkAlertsServiceAdvisories {
    id: i32,
    message: String,
    line: String,
    platform: String,
    playtime: Option<String>,
    create_date: String,
    start_date_time: Option<String>,
    short_start_date_time: Option<String>,
    end_date_time: Option<String>,
    short_end_date_time: Option<String>,
    //"Timestamp":"11/5/2023 4:16 PM",
    timestamp: String,
    #[serde(rename = "Type")]
    alert_type: String,
    details_page: String,
    alert_details_page: Option<String>,
    date_range_output: String
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct MetrolinkAlertsRaw {
    results: MetrolinkAlertsResults,
    time_limit: i32,
    minified: bool,
    is_homepage: bool
}

fn metrolink_alert_line_abbrv_to_route_id(x:&str) -> &str {
    match x {
        "AV" => "Antelope Valley Line",
        "IEOC" => "Inland Emp.-Orange Co. Line",
        "OC" => "Orange County Line",
        "SB" => "San Bernardino Line",
        "VC" => "Ventura County Line",
        "RIV" => "Riverside Line",
        "91/PV" => "91 Line",
        _ => ""
    }
}

async fn get_metrolink_alerts(client: &ReqwestClient) -> Option<Vec<u8>> {
    let alert_html = client
        .get("https://metrolinktrains.com/train-status/")
        .send()
        .await;

    match alert_html {
        Ok(alert_html) => {
            let re = Regex::new(r"<script\b[^>]*>\s+window.ml([\s\S]*?)<\/script>").unwrap();

            let text = alert_html.text().await.unwrap();

            let alertsscript = re.captures(text.as_str()).unwrap().get(0).unwrap().as_str();
            let alertsscript = Regex::new("<script type=\\\"text/javascript\\\">\\s+window.ml = window.ml\\s+\\|\\|\\s+\\{\\};(\\s)+window.ml =").unwrap().replace(&alertsscript,"");
            let alertsscript = Regex::new("</script>").unwrap().replace(&alertsscript,"");

            let alertsscript = alertsscript.trim().replace("results", "\"results\"");
            let alertsscript = alertsscript.replace("timeLimit", "\"timeLimit\"");
            let alertsscript = alertsscript.replace("minified", "\"minified\"");
            let alertsscript = alertsscript.replace("isHomepage", "\"isHomepage\"");
            /*
            let alertsscript = snailquote::unescape(alertsscript);

            let alertsscript = alertsscript.unwrap();

            let alertsscript = alertsscript.as_str();*/

            //println!("Alerts script: {:}", alertsscript);

            let alerts: Result<MetrolinkAlertsRaw, _> = serde_json::from_str(&alertsscript);

            match alerts {
                Ok(alerts) => {
                    //println!("Alerts: {:#?}", alerts);

                    let alerts_list = alerts.results.advisories.iter().map(|a| a.service_advisories.clone()).flatten().collect::<Vec<MetrolinkAlertsServiceAdvisories>>();


                    let alerts_gtfs_list = alerts_list.iter().map(|x| {
                        
                    let mut metrolink_link = String::from("https://metrolinktrains.com");

                    metrolink_link.push_str((x.details_page).as_str());

                    let url_alert = match x.alert_details_page {
                        Some(_) => {
                            Some(gtfs_rt::TranslatedString {
                                translation: vec![gtfs_rt::translated_string::Translation {
                                    text: metrolink_link,
                                    language: Some("en".to_string())
                                }]
                            })
                        }
                        None => None
                    };

                        gtfs_rt::FeedEntity {
                            id: x.id.to_string(),
                            is_deleted: Some(false),
                            trip_update: None,
                            vehicle: None,
                            alert: Some(gtfs_rt::Alert {
                                tts_description_text: None,
                                tts_header_text: None,
                                cause: None,
                                effect: None,
                                header_text: Some(gtfs_rt::TranslatedString {
                                    translation: vec![gtfs_rt::translated_string::Translation {
                                        text: x.message.clone(),
                                        language: Some("en".to_string())
                                    }]
                                }),
                                description_text: None,
                                informed_entity: vec![gtfs_rt::EntitySelector {
                                    agency_id: None,
                                    route_id: Some(metrolink_alert_line_abbrv_to_route_id(&x.line).to_string()),
                                    direction_id: None,
                                    route_type: None,
                                    trip: None,
                                    stop_id: None
                                }],
                                active_period: vec![gtfs_rt::TimeRange {
                                    start: None,
                                    end: None
                                }],
                                cause_detail: None,
                                effect_detail: None,
                                image: None,
                                image_alternative_text: None,
                                severity_level: None,
                                url: url_alert
                                }),
                            shape: None
                        }
                    }).collect::<Vec<gtfs_rt::FeedEntity>>();

                    let feed_message = FeedMessage {
                        header: gtfs_rt::FeedHeader {
                            gtfs_realtime_version: "2.0".to_string(),
                            incrementality: Some(gtfs_rt::feed_header::Incrementality::FullDataset as i32),
                            timestamp: Some(get_epoch_ms() as u64),
                        },
                        entity: alerts_gtfs_list
                    };

                    println!("Alerts: {:#?}", feed_message);

                    let bytes:Vec<u8> = feed_message.encode_to_vec();

                    Some(bytes)
                }
                Err(error) => {
                    println!("Error parsing alerts: {:#?}", error);
                    None
                }
            }
        }
        Err(error) => {
            println!("Error getting alerts: {:?}", error);
            None
        },
    }
}

fn determine_if_category_should_run(
    last_attempt: &Option<Instant>,
    last_protobuf_timestamp: &Option<u128>,
) -> bool {
    match last_attempt {
        None => true,
        Some(last_attempt) => {
            if last_attempt.elapsed().as_millis() as i64 > 500 {
                match last_protobuf_timestamp {
                    Some(last_protobuf_timestamp) => {
                        if get_epoch_ms() - (last_protobuf_timestamp * 1000) > 60_500 {
                            true
                        } else {
                            //println!("Skip! it was only {} ms ago", get_epoch_ms() - last_protobuf_timestamp * 1000);
                            false
                        }
                    }
                    None => true,
                }
            } else {
                false
            }
        }
    }
}

async fn runcategory(
    client: &ReqwestClient,
    metrolink_key: &String,
    category: &str,
    last_veh_attempt: &mut Option<Instant>,
    last_protobuf_timestamp: &mut Option<u128>,
) -> Option<Vec<u8>> {
    let url = match category {
        "trips" => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-trips",
        "vehicles" => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-vehicles",
        _ => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-vehicles",
    };

    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let response = client
        .get(url)
        .header("X-Api-Key", metrolink_key)
        .timeout(Duration::from_millis(15000))
        .send()
        .await;

    //set the new attempt time
    *last_veh_attempt = Some(Instant::now());

    match response {
        Ok(response) => {
            //if 429 response, freeze the program for 30 seconds

            if response.status().is_client_error() {
                println!(
                    "{}Recieved 429, freezing{}",
                    color::Fg(color::Red),
                    style::Reset
                );

                std::thread::sleep(Duration::from_millis(30_000));
            }

            if response.status().is_success() {
                match response.bytes().await {
                    Ok(bytes) => {
                        let bytes = bytes.to_vec();

                        println!("{} Success, byte length {}", &category, bytes.len());

                        let protobuf_message = parse_protobuf_message(&bytes);

                        match protobuf_message {
                            Ok(protobuf_message) => match protobuf_message.header.timestamp {
                                Some(timestamp) => {
                                    *last_protobuf_timestamp = Some(timestamp as u128);

                                    println!(
                                        "{} timestamp is {} aka {} ms ago",
                                        category,
                                        timestamp,
                                        get_epoch_ms() - (timestamp as u128 * 1000)
                                    );

                                    let feed_id = "f-metrolinktrains~rt";

                                    insert_gtfs_rt_bytes(
                                        &mut con,
                                        &bytes,
                                        &feed_id,
                                        &category.to_string(),
                                    );

                                    return Some(bytes);
                                }
                                None => {
                                    println!("{} Protobuf missing timestamp", &category);
                                }
                            },
                            Err(_e) => {
                                println!("{} Cannot interpret protobuf", category);
                            }
                        }
                    }
                    Err(_err) => {
                        println!("{} Failed to convert bytes", category)
                    }
                }
            }
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrolink_alerts() {
        let client = ReqwestClient::new();

        let alerts_result = get_metrolink_alerts(&client).await;

        assert_eq!(alerts_result.is_some(), true);
    }
}