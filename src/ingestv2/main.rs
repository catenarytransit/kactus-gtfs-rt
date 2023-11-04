use futures::StreamExt;
use redis::Commands;
use reqwest::Client as ReqwestClient;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use termion::{color, style};
extern crate color_eyre;
use fasthash::metro;
use kactus::parse_protobuf_message;
extern crate rand;
use crate::rand::prelude::SliceRandom;
use kactus::insert::insert_gtfs_rt_bytes;
extern crate csv;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use futures::future::join_all;

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

#[derive(Debug, Clone)]
struct OctaBit {
    position: gtfs_rt::Position,
    vehicle: gtfs_rt::VehicleDescriptor,
    trip: gtfs_rt::TripDescriptor,
}

#[derive(Debug)]
struct Agencyurls {
    vehicles: Option<String>,
    trips: Option<String>,
    alerts: Option<String>,
}

fn octa_compute_into_hash(feed: &gtfs_rt::FeedMessage) -> u64 {
    let arrayofelements = feed
        .entity
        .iter()
        .filter(|x| x.vehicle.is_some())
        .map(|x| {
            return OctaBit {
                position: x.vehicle.clone().unwrap().position.unwrap(),
                vehicle: x.vehicle.clone().unwrap().vehicle.unwrap(),
                trip: x.vehicle.clone().unwrap().trip.unwrap(),
            };
        })
        .collect::<Vec<OctaBit>>();

    let value = format!("{:?}", arrayofelements);
    return metro::hash64(value);
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let arguments = std::env::args();
    let arguments = arguments::parse(arguments).unwrap();

    let filenametouse = match arguments.get::<String>("urls") {
        Some(filename) => filename,
        None => String::from("urls.csv"),
    };

    let timeoutforfetch = match arguments.get::<u64>("timeout") {
        Some(filename) => filename,
        None => 15_000,
    };

    let threadcount = match arguments.get::<usize>("threads") {
        Some(threadcount) => threadcount,
        None => 50,
    };

    let file = File::open(filenametouse).unwrap();
    let mut reader = csv::Reader::from_reader(BufReader::new(file));

    let mut agencies: Vec<AgencyInfo> = Vec::new();

    let client = ReqwestClient::new();

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

    let mut reqquery_vec: Vec<Reqquery> = Vec::new();

    let mut lastloop;

    loop {
        let client = ReqwestClient::new();

        lastloop = Instant::now();

        let reqquery_vec_cloned = agencies.clone();

        let fetches = futures::stream::iter(reqquery_vec_cloned.into_iter().map(|agency| {
            let client = &client;

          async move {

            let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
            let mut con = redisclient.get_connection().unwrap();
             
             //println!("{:#?}", agency);

             let fetch = Agencyurls {
                 vehicles: make_url(&agency.realtime_vehicle_positions,&agency.auth_type,&agency.auth_header,&agency.auth_password),
                 trips: make_url(&agency.realtime_trip_updates,&agency.auth_type,&agency.auth_header,&agency.auth_password),
                 alerts: make_url(&agency.realtime_alerts,&agency.auth_type,&agency.auth_header,&agency.auth_password),
             };

             let passwordtouse = match &agency.multiauth {
                Some(multiauth) => {
                    let mut rng = rand::thread_rng();
                    let random_auth = multiauth.choose(&mut rng).unwrap();

                    random_auth.to_string()
                }
                None => agency.auth_password.clone(),
            };

            let vehicles_result = fetchurl(fetch.vehicles, &agency.auth_header, &agency.auth_type, &passwordtouse, client.clone(), timeoutforfetch).await;

            let trips_result = fetchurl(fetch.trips, &agency.auth_header, &agency.auth_type, &passwordtouse, client.clone(), timeoutforfetch).await;

            let alerts_result = fetchurl(fetch.alerts, &agency.auth_header, &agency.auth_type, &passwordtouse, client.clone(), timeoutforfetch).await;

            if vehicles_result.is_some() {

                let bytes = vehicles_result.as_ref().unwrap().to_vec();
                
                println!(
                    "{} vehicles bytes: {}",
                    &agency.onetrip,
                    bytes.len()
                );

                insert_gtfs_rt_bytes(&mut con, &bytes, &agency.onetrip, &("vehicles".to_string()));
            }

            if trips_result.is_some() {
                let bytes = trips_result.as_ref().unwrap().to_vec();

                println!(
                    "{} trips bytes: {}",
                    &agency.onetrip,
                    bytes.len()
                );

                insert_gtfs_rt_bytes(&mut con, &bytes, &agency.onetrip, &("trips".to_string()));
            }

            if alerts_result.is_some() {
                let bytes = alerts_result.as_ref().unwrap().to_vec();

                println!(
                    "{} alerts bytes: {}",
                    &agency.onetrip,
                    bytes.len()
                );

                insert_gtfs_rt_bytes(&mut con, &bytes, &agency.onetrip, &("alerts".to_string()));
            }

            send_to_aspen(&agency.onetrip, &vehicles_result, &trips_result, &alerts_result).await;
          } 
        }))
        .buffer_unordered(threadcount)
        .collect::<Vec<()>>();
        println!("Starting loop: {} fetches", &agencies.len());
        fetches.await;

        let duration = lastloop.elapsed();

        println!(
            "{}loop time is: {:?}{}",
            color::Bg(color::Green),
            duration,
            style::Reset
        );

        //if the iteration of the loop took <0.5 second, sleep for the remainder of the second
        if (duration.as_millis() as i32) < 500 {
            let sleep_duration = Duration::from_millis(500) - duration;
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

async fn send_to_aspen(agency: &String, vehicles_result: &Option<Vec<u8>>, trips_result: &Option<Vec<u8>>, alerts_result: &Option<Vec<u8>>) {
    
}

async fn fetchurl(url: Option<String>, auth_header: &String, auth_type: &String, auth_password: &String, client: ReqwestClient, timeoutforfetch: u64) -> Option<Vec<u8>> {
    match url {
        Some(url) => {
            let mut req = client.get(url);

            if auth_type == "header" {
                req = req.header(auth_header, auth_password);
            }

            let resp = req.timeout(Duration::from_millis(timeoutforfetch)).send().await;

            match resp {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(bytes_pre) => {
                                let bytes = bytes_pre.to_vec();
                                Some(bytes)
                            }
                            _ => {
                                None
                            }
                        }
                    } else {
                        None
                    }
                },
                Err(e) => {
                    println!("error fetching url: {:?}", e);
                    None
                }
            }

        },
    _ => None,
    }
}

fn make_url(url: &String, auth_type: &String, auth_header: &String, auth_password: &String) -> Option<String> {
    if url.is_empty() == false {
        let mut outputurl = url.clone();

        if !auth_password.is_empty() && auth_type == "query_param" {
            outputurl = outputurl.replace("PASSWORD", &auth_password);
        }

        Some(outputurl)
    } else {
        None
    }
}