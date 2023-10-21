use protobuf::Message;
use redis::Commands;
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};


use kactus::parse_protobuf_message;
use std::fs;

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
    let arguments = arguments::parse(arguments).unwrap();

    let client = ReqwestClient::new();
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

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

        if veh_run {
            runcategory(
                &client,
                &metrolink_key,
                "vehicles",
                &mut last_veh_attempt,
                &mut last_veh_protobuf_timestamp,
            )
            .await;
        }

        if trip_run {
            runcategory(
                &client,
                &metrolink_key,
                "trips",
                &mut last_trip_attempt,
                &mut last_trip_protobuf_timestamp,
            )
            .await;
        }

        //added this section because thread looping apparently consumes the whole core
        //20% cpu usage on the crappy Intel NUC this program executes on

        let instant_comp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Back to 1969?!?!!!");
        if (last_veh_protobuf_timestamp.is_some() && last_trip_protobuf_timestamp.is_some()) {
            if (instant_comp.as_millis() - (last_veh_protobuf_timestamp.unwrap() * 1000) < 57_000
                && instant_comp.as_millis() - (last_trip_protobuf_timestamp.unwrap() * 1000)
                    < 57_000)
            {
                let sleep_for = (std::cmp::min(
                    last_veh_protobuf_timestamp.unwrap(),
                    last_trip_protobuf_timestamp.unwrap(),
                ) + 57)
                    - instant_comp.as_secs() as u128;

                if (sleep_for > 1) {
                    println!("Sleeping for {}s", sleep_for);

                    std::thread::sleep(Duration::from_secs(sleep_for.try_into().unwrap()));
                }
            }
        }
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
) -> () {
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

                                    let _: () = con
                                        .set(
                                            format!(
                                                "gtfsrttime|{}|{}",
                                                "f-metrolinktrains~rt", &category
                                            ),
                                            SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis()
                                                .to_string(),
                                        )
                                        .unwrap();

                                    let _: () = con
                                        .set(
                                            format!(
                                                "gtfsrt|{}|{}",
                                                "f-metrolinktrains~rt", &category
                                            ),
                                            bytes,
                                        )
                                        .unwrap();

                                    let _: () = con
                                        .set(
                                            format!("gtfsrtexists|{}", "f-metrolinktrains~rt"),
                                            SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis()
                                                .to_string(),
                                        )
                                        .unwrap();
                                }
                                None => {
                                    println!("{} Protobuf missing timestamp", &category);
                                }
                            },
                            Err(e) => {
                                println!("{} Cannot interpret protobuf", category);
                            }
                        }
                    }
                    Err(err) => {
                        println!("{} Failed to convert bytes", category)
                    }
                }
            }
        }
        Err(e) => {
            println!("{:?}", e)
        }
    };
}
