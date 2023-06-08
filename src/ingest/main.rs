use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

extern crate csv;

use csv::Reader;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug)]
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
}

fn make_reqwest_to_agency(
    url: &String,
    has_auth: bool,
    auth_type: &String,
    auth_header: &String,
    auth_password: &String,
) -> reqwest::RequestBuilder {
    let reqwest_client = ReqwestClient::new();

    let urltouse = url.clone();

    if auth_type == "url" {
        let _ = urltouse.replace("PASSWORD", &auth_password);
    }

    let requesttoreturn = reqwest_client.get(urltouse);

    if auth_type == "header" {
        return requesttoreturn.header(auth_header, auth_password);
    }

    requesttoreturn
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
        };

        agency_infos.push(agency_info);
    }

    let reqwest_client = ReqwestClient::new();

    loop {
        lastloop = Instant::now();

        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();

        'eachagencyloop: for agency_info in &agency_infos {
            let last_updated_time =
                con.get::<String, u64>(format!("{}|last_updated", agency_info.onetrip));

            match last_updated_time {
                Ok(last_updated_time) => {
                    let time_since_last_run = (SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("System time not working!")
                        .as_millis() as u64)
                        - last_updated_time;

                    if time_since_last_run < (agency_info.fetch_interval * 1000.0) as u64 {
                        println!(
                            "skipping {} because it was last updated {} seconds ago",
                            agency_info.onetrip,
                            time_since_last_run / 1000
                        );
                        continue;
                    }
                }
                Err(_) => {}
            }

            if agency_info.realtime_vehicle_positions.is_empty() == false {
                let start_gtfs_pull = Instant::now();
                let resp_vehicles = make_reqwest_to_agency(
                    &agency_info.realtime_vehicle_positions,
                    agency_info.has_auth,
                    &agency_info.auth_type,
                    &agency_info.auth_header,
                    &agency_info.auth_password,
                )
                .send()
                .await;
                let duration_gtfs_pull = start_gtfs_pull.elapsed();

                println!("pull gtfs time is: {:?}", duration_gtfs_pull);
                let bytes: Vec<u8> = resp_vehicles.unwrap().bytes().await.unwrap().to_vec();

                println!("{} bytes: {}", agency_info.onetrip, bytes.len());

                let _ = con
                    .set::<String, Vec<u8>, ()>(format!("{}|vehicles", agency_info.onetrip), bytes)
                    .unwrap();
            }

            if agency_info.realtime_trip_updates.is_empty() == false {
                let resp_trip_updates = make_reqwest_to_agency(
                    &agency_info.realtime_trip_updates,
                    agency_info.has_auth,
                    &agency_info.auth_type,
                    &agency_info.auth_header,
                    &agency_info.auth_password,
                )
                .send()
                .await;

                let bytes_trip_updates: Vec<u8> =
                    resp_trip_updates.unwrap().bytes().await.unwrap().to_vec();

                let _ = con.set::<String, Vec<u8>, ()>(
                    format!("{}|trip_updates", agency_info.onetrip),
                    bytes_trip_updates,
                );
            }

            if agency_info.realtime_alerts.is_empty() == false {
                let resp_alerts = make_reqwest_to_agency(
                    &agency_info.realtime_alerts,
                    agency_info.has_auth,
                    &agency_info.auth_type,
                    &agency_info.auth_header,
                    &agency_info.auth_password,
                )
                .send()
                .await;

                let bytes_alerts: Vec<u8> = resp_alerts.unwrap().bytes().await.unwrap().to_vec();

                let _ = con.set::<String, Vec<u8>, ()>(
                    format!("{}|alerts", agency_info.onetrip),
                    bytes_alerts,
                );
            }

            //set the last updated time for this agency
            let _ = con
                .set::<String, u64, ()>(
                    format!("{}|last_updated", agency_info.onetrip),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("System time not working!")
                        .as_millis() as u64,
                )
                .unwrap();
        }

        let duration = lastloop.elapsed();

        println!("loop time is: {:?}", duration);

        if (duration.as_millis() as i32) < 1000 {
            let sleep_duration = Duration::from_millis(1000) - duration;
            println!("sleeping for {:?}", sleep_duration);
            std::thread::sleep(sleep_duration);
        }
    }
}
