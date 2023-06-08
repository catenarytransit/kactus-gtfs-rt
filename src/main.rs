use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant};

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
    fetch_interval: f32,
}

#[tokio::main]
async fn main() {
    let mut lastloop = Instant::now();

    let url = "https://api.octa.net/GTFSRealTime/protoBuf/VehiclePositions.aspx";

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
            fetch_interval: record[7].parse().unwrap(),
        };

        agency_infos.push(agency_info);
    }

    let reqwest_client = ReqwestClient::new();

    loop {
        lastloop = Instant::now();

        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();

        for agency_info in &agency_infos {
            let start_gtfs_pull = Instant::now();
            let resp_vehicles = reqwest_client
                .get(&agency_info.realtime_vehicle_positions)
                .send()
                .await;
            let duration_gtfs_pull = start_gtfs_pull.elapsed();

            println!("pull gtfs time is: {:?}", duration_gtfs_pull);
            let bytes: Vec<u8> = resp_vehicles.unwrap().bytes().await.unwrap().to_vec();

            println!("{} bytes: {}", agency_info.onetrip, bytes.len());

            let _ = con
                .set::<String, Vec<u8>, ()>(format!("{}|vehicles", agency_info.onetrip), bytes)
                .unwrap();

            let resp_trip_updates = reqwest_client
                .get(&agency_info.realtime_trip_updates)
                .send()
                .await;

            let bytes_trip_updates: Vec<u8> =
                resp_trip_updates.unwrap().bytes().await.unwrap().to_vec();

            let _ = con.set::<String, Vec<u8>, ()>(
                format!("{}|trip_updates", agency_info.onetrip),
                bytes_trip_updates,
            );

            let resp_alerts = reqwest_client
                .get(&agency_info.realtime_alerts)
                .send()
                .await;

            let bytes_alerts: Vec<u8> = resp_alerts.unwrap().bytes().await.unwrap().to_vec();

            let _ = con.set::<String, Vec<u8>, ()>(
                format!("{}|alerts", agency_info.onetrip),
                bytes_alerts,
            );
        }

        let duration = lastloop.elapsed();

        println!("loop time is: {:?}", duration);

        if (duration.as_millis() as i32) < 1000 {
            let sleep_duration = Duration::from_millis(1000) - duration;
            println!("sleeping for {:?}", sleep_duration);
            std::thread::sleep(sleep_duration);
        }
    }

    /*
    loop {
        let start = Instant::now();
        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();

        let duration = start.elapsed();

        println!("connect to redis time is: {:?}", duration);

        let _: () = con.set("my_key", 42).unwrap();

        let reqwest_client = ReqwestClient::new();
        let start_gtfs_pull = Instant::now();
        let resp = reqwest_client.get(url).send().await;
        let duration_gtfs_pull = start_gtfs_pull.elapsed();

        println!("pull gtfs time is: {:?}", duration_gtfs_pull);
        let bytes: Vec<u8> = resp.unwrap().bytes().await.unwrap().to_vec();

        println!("bytes: {}", bytes.len());

        con.set::<String, Vec<u8>, ()>("octa-vehicle".to_string(), bytes)
            .unwrap();

        println!("set octa-vehicle in the redis db");

        std::thread::sleep(std::time::Duration::from_millis(200));
    }*/
}
