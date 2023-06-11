use futures::StreamExt;
use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};

extern crate csv;

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

fn main() {
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
}
