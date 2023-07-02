use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;
use std::time::{Instant, SystemTime, Duration, UNIX_EPOCH};
use termion::{color, style};
use protobuf::Message;

use kactus::gtfs_realtime;
use kactus::gtfs_realtime::FeedMessage;

use std::fs;

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn parse_protobuf_message(bytes: &[u8]) -> Result<FeedMessage, protobuf::Error> {
    return gtfs_realtime::FeedMessage::parse_from_bytes(bytes);
}

#[tokio::main]
async fn main() {

    let metrolink_key = fs::read_to_string("./metrolink-key.txt").expect("Unable to read file metrolink-key.txt").trim().to_string();

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
            runcategory(&client, &metrolink_key, "vehicles", &mut last_veh_attempt, &mut last_veh_protobuf_timestamp).await;
        }

        if trip_run {
            runcategory(&client, &metrolink_key, "trips", &mut last_trip_attempt, &mut last_trip_protobuf_timestamp).await;
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
                        if get_epoch_ms() - (last_protobuf_timestamp * 1000) > 30_000 {
                            true
                        } else {
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

async fn runcategory(client: &ReqwestClient, metrolink_key: &String, category: &str, last_veh_attempt: &mut Option<Instant>, last_protobuf_timestamp: &mut Option<u128>) -> () {
    let url = match category {
        "trips" => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-trips",
        "vehicles" => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-vehicles",
        _ => "https://metrolink-gtfsrt.gbsdigital.us/feed/gtfsrt-vehicles"
    };
    
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let response = client.get(url)
            .header("X-Api-Key", metrolink_key)
            .timeout(Duration::from_millis(15000))
            .send().await;

            //set the new attempt time
            *last_veh_attempt = Some(Instant::now());

            match response {
                Ok(response) => {
                    //if 429 response, freeze the program for 20 seconds

                    if response.status().is_client_error() {
                    println!("{}Recieved 429, freezing{}",color::Fg(color::Red), style::Reset);

                    std::thread::sleep(Duration::from_millis(20_000));
                    }

                    if response.status().is_success() {
                        match response.bytes().await {
                            Ok(bytes) => {
                                let bytes = bytes.to_vec();

                                println!("Success, byte length {}", bytes.len());

                                let protobuf_message = parse_protobuf_message(&bytes);

                                match protobuf_message {
                                    Ok(protobuf_message) => {
                                        match protobuf_message.header.timestamp {
                                            Some(timestamp) => {
                                                *last_protobuf_timestamp = Some(timestamp as u128);

                                                let _: () = con.set(format!(
                                                    "gtfsrttime|{}|{}",
                                                    "f-metrolinktrains~rt", &category
                                                ), SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_millis()
                                                .to_string())
                                                .unwrap();

                                                let _: () = con.set(format!(
                                                    "gtfsrt|{}|{}",
                                                    "f-metrolinktrains~rt", &category
                                                ), bytes).unwrap();

                                            },
                                            None => {
                                                println!("Protobuf missing timestamp");
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        println!("Cannot interpret protobuf");
                                    }
                                }
                                
                            }
                            Err(err) => {
                                println!("Failed to convert bytes")
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("{:?}", e)
                }
            };
}