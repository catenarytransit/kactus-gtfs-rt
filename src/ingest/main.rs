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

async fn insertIntoUrl(category: &String, agency_info: &AgencyInfo) -> Result<(), Box<dyn Error>> {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let start_gtfs_pull = Instant::now();

    let mut url: String;

    if (category == "vehicles") {
        url = agency_info.realtime_vehicle_positions.clone();
    } else if (category == "trip_updates") {
        url = agency_info.realtime_trip_updates.clone();
    } else if (category == "alerts") {
        url = agency_info.realtime_alerts.clone();
    } else {
        return Err("Invalid category".into());
    }

    let resp = make_reqwest_to_agency(
        &url,
        agency_info.has_auth,
        &agency_info.auth_type,
        &agency_info.auth_header,
        &agency_info.auth_password,
    )
    .send()
    .await
    .unwrap();

    if (reqwest::Response::status(&resp) == reqwest::StatusCode::OK) {
        let duration_gtfs_pull = start_gtfs_pull.elapsed();

        println!(
            "pull {} {} gtfs time is: {:?}",
            &agency_info.onetrip, category, duration_gtfs_pull
        );

        let bytes: Vec<u8> = resp.bytes().await.unwrap().to_vec();

        println!(
            "{} {} bytes: {}",
            agency_info.onetrip,
            category,
            bytes.len()
        );

        let _: () = con.set(
            format!("gtfsrt|{}|{}", agency_info.onetrip, category),
            bytes,
        )?;

        Ok(())
    } else {
        Err("Not 200 response".into())
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
        };

        agency_infos.push(agency_info);
    }

    loop {
        lastloop = Instant::now();

        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();

        //if the agency timeout has not expired, skip it
        'eachagencyloop: for agency_info in &agency_infos {
            let last_updated_time =
                con.get::<String, u64>(format!("metagtfsrt|{}|last_updated", agency_info.onetrip));

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
                        continue 'eachagencyloop;
                    }
                }
                Err(_) => {}
            }

            if agency_info.realtime_vehicle_positions.is_empty() == false {
                let _ = insertIntoUrl(&"vehicles".to_string(), &agency_info);
            }

            if agency_info.realtime_trip_updates.is_empty() == false {
                let _ = insertIntoUrl(&"trip_updates".to_string(), &agency_info);
            }

            if agency_info.realtime_alerts.is_empty() == false {
                let _ = insertIntoUrl(&"alerts".to_string(), &agency_info);
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

        let duration = lastloop.elapsed();

        println!("loop time is: {:?}", duration);

        //if the iteration of the loop took <1 second, sleep for the remainder of the second
        if (duration.as_millis() as i32) < 1000 {
            let sleep_duration = Duration::from_millis(1000) - duration;
            println!("sleeping for {:?}", sleep_duration);
            std::thread::sleep(sleep_duration);
        }
    }
}
