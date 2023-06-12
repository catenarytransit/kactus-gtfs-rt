use futures::StreamExt;
use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};

extern crate csv;

extern crate farmhash;

use csv::Reader;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
//this program tests the time to write and read from redis, as well as time to calculate a fast hash

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

    let mut urltouse = url.clone();

    if auth_type == "url" {
        urltouse = urltouse.replace("PASSWORD", &auth_password);
    }

    let requesttoreturn = reqwest_client.get(urltouse);

    if auth_type == "header" {
        return requesttoreturn.header(auth_header, auth_password);
    }

    requesttoreturn
}

#[tokio::main]
async fn main() {
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

    let connect_time = Instant::now();

    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();

    let mut con = redisclient.get_connection().unwrap();

    let redis_connected = Instant::now();

    let timetoconnect = redis_connected.duration_since(connect_time);

    println!("Time to connect to redis: {}µs", timetoconnect.as_micros());

    for agency_info in agency_infos {
        /*
        let a = Instant::now();
        let b = Instant::now();
        println!("Test time, {}ns", b.duration_since(a).as_nanos());
        */

        let resp = make_reqwest_to_agency(
            &agency_info.realtime_vehicle_positions,
            agency_info.has_auth,
            &agency_info.auth_type,
            &agency_info.auth_header,
            &agency_info.auth_password,
        )
        .send()
        .await
        .unwrap();

        match resp.bytes().await {
            Ok(bytes_pre) => {
                let bytes = bytes_pre.to_vec();

                println!("Onetrip: {}", &agency_info.onetrip);

                println!("Bytes: {}", bytes.len());

                let timetostorestart = Instant::now();
                let _: () = con.set(format!("gtfstest|{}", &agency_info.onetrip), &bytes).unwrap();
                let timetostoreend = Instant::now();
                let timetostore = timetostoreend.duration_since(timetostorestart);
                println!("Time to store: {}µs", timetostore.as_micros());

                let timetocomputehash_start = Instant::now();
                let hash: u64 = farmhash::hash64(&bytes);
                let timetocomputehash_end = Instant::now();
                let timetocomputehash = timetocomputehash_end.duration_since(timetocomputehash_start);
                println!("Time to compute hash: {}µs", timetocomputehash.as_micros());
                println!("hash: {}", hash);
                //store the hash into redis

                let timetostorehash_start = Instant::now();
                let _: () = con.set(format!("gtfstesthash|{}", &agency_info.onetrip), hash).unwrap();
                let timetostorehash_end = Instant::now();
                let timetostorehash = timetostorehash_end.duration_since(timetostorehash_start);
                println!("Time to store hash: {}µs", timetostorehash.as_micros());

                let timetoread_start = Instant::now();
                let _: u64 = con.get(format!("gtfstesthash|{}", &agency_info.onetrip)).unwrap();
                let timetoread_end = Instant::now();
                let timetoread = timetoread_end.duration_since(timetoread_start);
                println!("Time to read: {}µs", timetoread.as_micros());

            }
            Err(e) => {
                println!("Error {}: {}", &agency_info.onetrip ,e);
            }
        }
    }
}
