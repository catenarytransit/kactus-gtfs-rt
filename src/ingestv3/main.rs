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
struct ReqForAgency {
}

#[derive(Debug, Clone)]
struct OctaBit {
    position: gtfs_rt::Position,
    vehicle: gtfs_rt::VehicleDescriptor,
    trip: gtfs_rt::TripDescriptor,
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

    let mut lastloop;

    loop {
        lastloop = Instant::now();

        let reqquery_vec_cloned = reqquery_vec.clone();

        let fetches = futures::stream::iter(reqquery_vec_cloned.into_iter().map(|reqquery| {
            let client = &client;
            
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
                        println!("{:#?}", last_updated_time);
                    }
                }

                if allowrun {

                    let mut urltouse = reqquery.url.clone();

                    let passwordtouse = match reqquery.multiauth {
                        Some(multiauth) => {
                            let mut rng = rand::thread_rng();
                            let random_auth = multiauth.choose(&mut rng).unwrap();

                            random_auth.to_string()
                        }
                        None => reqquery.auth_password.clone(),
                    };

                    if !passwordtouse.is_empty() && reqquery.auth_type == "query_param" {
                        urltouse = urltouse.replace("PASSWORD", &passwordtouse);
                    }

                    let mut req = client.get(urltouse);

                    if reqquery.auth_type == "header" {
                        req = req.header(&reqquery.auth_header, &passwordtouse);
                    }

                    let resp = req.timeout(Duration::from_millis(timeoutforfetch)).send().await;

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
                                        
                                        let mut continue_run = true;

                                        //if feed is OCTA
                                        if reqquery.onetrip == "f-octa~rt" {
                                            //if category is vehicles
                                            if reqquery.category == "vehicles" {
                                                //convert existing to structs and see if the feed has even changed

                                                let new_proto = parse_protobuf_message(&bytes);

                                                let old_data = con.get::<String, Vec<u8>>(format!(
                                                    "gtfsrt|{}|{}",
                                                    &reqquery.onetrip, &reqquery.category
                                                ));

                                                if old_data.is_ok() {
                                                let old_proto =  parse_protobuf_message(&old_data.unwrap());

                                                if new_proto.is_ok() && old_proto.is_ok() {
                                                    println!("Comparing OCTA feeds");
                                                    let new_proto = new_proto.unwrap();
                                                    let old_proto = old_proto.unwrap();

                                                    let newhash = octa_compute_into_hash(&new_proto);
                                                    let oldhash = octa_compute_into_hash(&old_proto);

                                                    if newhash == oldhash {
                                                        continue_run = false;
                                                        println!("Cancelled OCTA for having same hash");
                                                    }
                                                }
                                                }
                                                }
                                            }
                                        

                                        if true == continue_run {
                                            
                                        insert_gtfs_rt_bytes(&mut con, &bytes, &reqquery.onetrip, &reqquery.category);
                                        }
                                    }
                                    Err(_) => {
                                        println!("error parsing bytes: {}", &reqquery.url);
                                    }
                                }
                            }
                            else if resp.status().is_redirection() {
                                println!(
                                    "{}{} {} HTTP redirect: {}{}",
                                    color::Fg(color::Yellow),
                                    &reqquery.onetrip,
                                    &reqquery.category,
                                    resp.status(),
                                    style::Reset
                                );
                            }
                            else {
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
        .buffer_unordered(threadcount)
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