use futures::StreamExt;
use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};

extern crate rand;
use crate::rand::prelude::SliceRandom;

extern crate csv;

use csv::Reader;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

//stores the config for each agency
#[derive(Debug, Clone)]
struct AgencyInfo {
    onetrip: String,
    realtime_vehicle_positions: String,
    realtime_trip_updates: String,
    realtime_alerts: String,
    has_auth: bool,
    auth_type: String,
    auth_header: String,
    auth_password: String,
    fetch_interval: f32,
    multiauth: Option<Vec<String>>,
}

fn make_reqwest_to_agency(
    url: &String,
    has_auth: bool,
    auth_type: &String,
    auth_header: &String,
    auth_password: &String,
    multiauth: &Option<Vec<String>>,
) -> reqwest::RequestBuilder {
    let reqwest_client = ReqwestClient::new();

    let mut urltouse = url.clone();

    if auth_type == "url" {
        let mut passwordtouse = auth_password.clone();

        if let Some(multiauth) = multiauth {
            //randomly select one of the auths
            let mut rng = rand::thread_rng();
            let random_auth = multiauth.choose(&mut rng).unwrap();
            passwordtouse = random_auth.clone();
        }

        urltouse = urltouse.replace("PASSWORD", &passwordtouse);
    }

    let requesttoreturn = reqwest_client.get(urltouse);

    let mut passwordtouse = auth_password.clone();

    if let Some(multiauth) = multiauth {
        //randomly select one of the auths
        let mut rng = rand::thread_rng();
        let random_auth = multiauth.choose(&mut rng).unwrap();
        passwordtouse = random_auth.clone();
    }

    if auth_type == "header" {
        return requesttoreturn.header(auth_header, passwordtouse);
    }

    requesttoreturn
}

async fn insertIntoUrl(category: &String, agency_info: &AgencyInfo) -> Result<(), Box<dyn Error>> {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();
    let start_gtfs_pull = Instant::now();
    let url: String;

    match category.as_str() {
        "vehicles" => {
            url = agency_info.realtime_vehicle_positions.clone();
        }
        "trip_updates" => {
            url = agency_info.realtime_trip_updates.clone();
        }
        "alerts" => {
            url = agency_info.realtime_alerts.clone();
        }
        _ => {
            println!("Invalid category");
            return Err("Invalid category".into());
        }
    }

    let resp = make_reqwest_to_agency(
        &url,
        agency_info.has_auth,
        &agency_info.auth_type,
        &agency_info.auth_header,
        &agency_info.auth_password,
        &agency_info.multiauth,
    )
    .timeout(Duration::from_secs(5))
    .send()
    .await;

    match resp {
        Ok(resp) => {
            println!("got response {}", &agency_info.onetrip);

            if (reqwest::Response::status(&resp) == reqwest::StatusCode::OK) {
                let duration_gtfs_pull = start_gtfs_pull.elapsed();
        
                println!(
                    "pull {} {} gtfs time is: {:?}",
                    &agency_info.onetrip, category, duration_gtfs_pull
                );
        
                //let bytes: Vec<u8> = resp.bytes().await.unwrap().to_vec();
        
                match resp.bytes().await {
                    Ok(bytes_pre) => {
                        let bytes = bytes_pre.to_vec();
        
                        println!(
                            "{} {} bytes: {}",
                            agency_info.onetrip,
                            category,
                            bytes.len()
                        );
        
                        let _: () = con
                            .set(
                                format!("gtfsrt|{}|{}", agency_info.onetrip, category),
                                bytes.to_vec(),
                            )
                            .unwrap();
        
                        let _: () = con
                            .set(
                                format!("gtfsrtvalid|{}|{}", agency_info.onetrip, category),
                                true,
                            )
                            .unwrap();
                    }
                    Err(e) => {
                        println!("error getting bytes");
                        return Err(e.into());
                    }
                }
        
                Ok(())
            } else {
                println!(
                    "{}{}Not 200 response{}",
                    color::Bg(color::Black),
                    color::Fg(color::Red),
                    style::Reset
                );
                println!(
                    "{}{:?}{}",
                    color::Fg(color::Red),
                    resp.text().await.unwrap(),
                    style::Reset
                );
                Err("Not 200 response".into())
            }
        },
        Err(e) => {
            println!("error getting response");
            println!("{:?}", e);
            return Err("Server Timed Out".into())
        }
    }


}

#[tokio::main]
async fn main() {
    let mut lastloop = Instant::now();

    let file = File::open("urls.csv").unwrap();
    let mut reader = csv::Reader::from_reader(BufReader::new(file));

    let mut agency_infos: Vec<AgencyInfo> = Vec::new();

    for record in reader.records() {
        let record = record.unwrap();

        let agency_info = AgencyInfo {
            onetrip: record[0].to_string(),
            realtime_vehicle_positions: record[1].to_string(),
            realtime_trip_updates: record[2].to_string(),
            realtime_alerts: record[3].to_string(),
            has_auth: record[4].parse().unwrap(),
            auth_type: record[5].to_string(),
            auth_header: record[6].to_string(),
            auth_password: record[7].to_string(),
            fetch_interval: record[8].parse().unwrap(),
            multiauth: convert_multiauth_to_vec(&(record[9].to_string())),
        };

        agency_infos.push(agency_info);
    }

    let mut lastloop: std::time::Instant;

    loop {
        lastloop = Instant::now();

        let agency_infos_cloned = agency_infos.clone();

        let fetches = futures::stream::iter(agency_infos_cloned.into_iter().map(|agency_info| {
            async move {
                let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
                let mut con = redisclient.get_connection().unwrap();

                let mut canrun = true;

                let last_updated_time = con
                    .get::<String, u64>(format!("metagtfsrt|{}|last_updated", agency_info.onetrip));

                match last_updated_time {
                    Ok(last_updated_time) => {
                        let time_since_last_run = (SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("System time not working!")
                            .as_millis() as u64)
                            - last_updated_time;
                        /*
                                println!(
                                    "{}{}{} {}{} {}{} {}{}{}",
                                    color::Bg(color::Blue),
                                    agency_info.onetrip,
                                    style::Reset,
                                    color::Fg(color::White),
                                    last_updated_time,
                                    color::Fg(color::Cyan),
                                    SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("System time not working!")
                                    .as_millis() as u64,
                                    color::Fg(color::Blue),
                                    time_since_last_run,
                                    style::Reset
                                );
                        */

                        if time_since_last_run < (agency_info.fetch_interval * 1000.0) as u64 {
                            println!(
                                "{}skipping {} because it was last updated {} ms ago{}",
                                color::Bg(color::Blue),
                                agency_info.onetrip,
                                time_since_last_run,
                                style::Reset
                            );
                            canrun = false;
                        }
                    }
                    Err(last_updated_time) => {
                        println!(
                            "{}Error! Redis didn't fetch last_updated_time for {}{}",
                            color::Bg(color::Black),
                            agency_info.onetrip,
                            style::Reset
                        );
                        println!(
                            "{}{:?}{}",
                            color::Bg(color::Black),
                            last_updated_time,
                            style::Reset
                        );
                    }
                }

                if &agency_info.auth_password == "EXAMPLEKEY" {
                    canrun = false;
                }

                if canrun == true {
                    if !agency_info.realtime_vehicle_positions.is_empty() {
                        let _ = insertIntoUrl(&("vehicles".to_string()), &agency_info).await;
                    }

                    if !agency_info.realtime_trip_updates.is_empty() {
                        let _ = insertIntoUrl(&("trip_updates".to_string()), &agency_info).await;
                    }

                    if !agency_info.realtime_alerts.is_empty() {
                        let _ = insertIntoUrl(&("alerts".to_string()), &agency_info).await;
                    }
                    //set the last updated time for this agency
                    let _ = con
                        .set::<String, u64, ()>(
                            format!("metagtfsrt|{}|last_updated", agency_info.onetrip),
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("System time not working!")
                                .as_millis() as u64,
                        )
                        .unwrap();
                }
            }
        }))
        .buffer_unordered(50)
        .collect::<Vec<()>>();
        println!("Waiting...");
        fetches.await;

        let duration = lastloop.elapsed();

        println!(
            "{}loop time is: {:?}{}",
            color::Bg(color::Green),
            duration,
            style::Reset
        );

        //if the iteration of the loop took <1 second, sleep for the remainder of the second
        if (duration.as_millis() as i32) < 1000 {
            let sleep_duration = Duration::from_millis(1000) - duration;
            println!("sleeping for {:?}", sleep_duration);
            std::thread::sleep(sleep_duration);
        }
    }
}

fn convert_multiauth_to_vec(inputstring: &String) -> Option<Vec<String>> {
    if inputstring.is_empty() == false {
        let mut outputvec: Vec<String> = Vec::new();

        let split = inputstring.split(",");

        for s in split {
            outputvec.push(s.to_string());
        }

        Some(outputvec)
    } else {
        None
    }
}
