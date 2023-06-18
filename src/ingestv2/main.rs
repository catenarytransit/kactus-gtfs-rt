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

#[derive(Debug, Clone)]
struct Reqquery {
    url: String,
    has_auth: bool,
    auth_type: String,
    auth_header: String,
    auth_password: String,
    category: String,
    multiauth: Option<Vec<String>>,
    onetrip: String,
    fetch_interval: f32,
}

#[tokio::main]
async fn main() {
    let mut lastloop = Instant::now();

    let file = File::open("urls.csv").unwrap();
    let mut reader = csv::Reader::from_reader(BufReader::new(file));

    let mut agencies: Vec<AgencyInfo> = Vec::new();

    for record in reader.records() {
        match record {
            Ok(record) => {
                let agency = AgencyInfo {
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

                agencies.push(agency);
            }
            Err(e) => {
                println!("error reading csv");
                println!("{:?}", e);
            }
        }
    }

    let mut lastloop: std::time::Instant;

    let mut reqquery_vec: Vec<Reqquery> = Vec::new();

    //this converts all the agencies into a vec of the requests
    for agency in agencies {
        let mut canrun = true;

        if agency.auth_password == "EXAMPLEKEY" {
            canrun = false;
        }

        if canrun == true {
            if !agency.realtime_vehicle_positions.is_empty() {
                let reqquery = Reqquery {
                    url: agency.realtime_vehicle_positions.clone(),
                    has_auth: agency.has_auth,
                    auth_type: agency.auth_type.clone(),
                    auth_header: agency.auth_header.clone(),
                    auth_password: agency.auth_password.clone(),
                    category: "vehicles".to_string(),
                    multiauth: agency.multiauth.clone(),
                    onetrip: agency.onetrip.clone(),
                    fetch_interval: agency.fetch_interval,
                };

                reqquery_vec.push(reqquery);
            }

            if !agency.realtime_trip_updates.is_empty() {
                let reqquery = Reqquery {
                    url: agency.realtime_trip_updates.clone(),
                    has_auth: agency.has_auth,
                    auth_type: agency.auth_type.clone(),
                    auth_header: agency.auth_header.clone(),
                    auth_password: agency.auth_password.clone(),
                    category: "trips".to_string(),
                    multiauth: agency.multiauth.clone(),
                    onetrip: agency.onetrip.clone(),
                    fetch_interval: agency.fetch_interval,
                };

                reqquery_vec.push(reqquery);
            }

            if !agency.realtime_alerts.is_empty() {
                let reqquery = Reqquery {
                    url: agency.realtime_alerts.clone(),
                    has_auth: agency.has_auth,
                    auth_type: agency.auth_type.clone(),
                    auth_header: agency.auth_header.clone(),
                    auth_password: agency.auth_password.clone(),
                    category: "alerts".to_string(),
                    multiauth: agency.multiauth.clone(),
                    onetrip: agency.onetrip.clone(),
                    fetch_interval: agency.fetch_interval,
                };

                reqquery_vec.push(reqquery);
            }
        }
    }

    let mut lastloop = Instant::now();

    loop {
        lastloop = Instant::now();

        let reqquery_vec_cloned = reqquery_vec.clone();

        let fetches = futures::stream::iter(reqquery_vec_cloned.into_iter().map(|reqquery| {
            async move {
                //println!("Fetching {}", reqquery_vec.url);

                let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
                let mut con = redisclient.get_connection().unwrap();

                let mut allowrun = true;

                let last_updated_time = con.get::<String, u64>(format!(
                    "gtfsrttime|{}|{}",
                    &reqquery.onetrip, &reqquery.category
                ));

                match last_updated_time {
                    Ok(last_updated_time) => {
                        let time_since_last_run = (SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("System time not working!")
                            .as_millis() as u64)
                            - last_updated_time;

                        if time_since_last_run < (reqquery.fetch_interval * 1000.0) as u64 {
                            allowrun = false;
                        }
                    }
                    Err(last_updated_time) => {
                        println!("Error getting last updated time, probably its the first time running this agency {}", reqquery.onetrip);
                        println!("{:?}", last_updated_time);
                    }
                }

                if allowrun {
                    let client = ReqwestClient::new();

                    let mut urltouse = reqquery.url.clone();

                    let passwordtouse = match reqquery.multiauth {
                        Some(multiauth) => {
                            let mut rng = rand::thread_rng();
                            let random_auth = multiauth.choose(&mut rng).unwrap();

                            random_auth.to_string()
                        }
                        None => reqquery.auth_password.clone(),
                    };

                    if !passwordtouse.is_empty() && reqquery.auth_type == "url" {
                        urltouse = urltouse.replace("PASSWORD", &passwordtouse);
                    }

                    let mut req = client.get(urltouse);

                    if reqquery.auth_type == "header" {
                        req = req.header(&reqquery.auth_header, &passwordtouse);
                    }

                    let resp = req.timeout(Duration::from_secs(10)).send().await;

                    match resp {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                match resp.bytes().await {
                                    Ok(bytes_pre) => {
                                        let bytes = bytes_pre.to_vec();

                                        println!(
                                            "{} {} bytes: {}",
                                            &reqquery.onetrip,
                                            &reqquery.category,
                                            bytes.len()
                                        );

                                        let _: () = con
                                            .set(
                                                format!(
                                                    "gtfsrt|{}|{}",
                                                    &reqquery.onetrip, &reqquery.category
                                                ),
                                                bytes.to_vec(),
                                            )
                                            .unwrap();

                                        let _: () = con
                                            .set(
                                                format!(
                                                    "gtfsrttime|{}|{}",
                                                    &reqquery.onetrip, &reqquery.category
                                                ),
                                                SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_millis()
                                                    .to_string(),
                                            )
                                            .unwrap();
                                    }
                                    Err(e) => {
                                        println!("error parsing bytes: {}", &reqquery.url);
                                    }
                                }
                            } else {
                                println!(
                                    "{} {} HTTP error: {}",
                                    &reqquery.onetrip,
                                    &reqquery.category,
                                    resp.status()
                                );
                            }
                        }
                        Err(e) => {
                            println!("{}error fetching url: {} {:?}{}", color::Fg(color::Red), &reqquery.url, e.source().unwrap(), style::Reset);
                        }
                    }
                }
            }
        }))
        .buffer_unordered(500)
        .collect::<Vec<()>>();
        println!("Starting loop: {} fetches", &reqquery_vec.len());
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
