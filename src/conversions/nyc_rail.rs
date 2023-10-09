use gtfs_rt::EntitySelector;
use gtfs_rt::TimeRange;
use prost::Message;
use protobuf::SpecialFields;
use protobuf::{CodedInputStream, Message as ProtobufMessage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;
use std::time::UNIX_EPOCH;

use kactus::insert::insert_gtfs_rt;

use serde_json;

use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};

use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainStatus {
    otp: Option<i32>,
    otp_location: Option<String>,
    held: bool,
    canceled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainCar {
    #[serde(rename = "type")]
    traintype: String,
    number: Option<i32>,
    loading: String,
    restroom: Option<bool>,
    revenue: Option<bool>,
    bikes: Option<i32>,
    locomotive: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainConsist {
    cars: Vec<TrainCar>,
    fleet: Option<String>,
    actual_len: Option<i32>,
    sched_len: Option<i32>,
    occupancy: Option<String>,
    occupancy_timestamp: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainLocation {
    longitude: f32,
    latitude: f32,
    //recieved in miles per hour, needs conversion to meters per second
    speed: Option<f32>,
    heading: Option<f32>,
    source: String,
    timestamp: i32,
    extra_info: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainTurf {
    length: f32,
    location_mp: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainStop {
    code: String,
    sched_time: i32,
    sign_track: Option<String>,
    avps_track_id: Option<String>,
    posted: bool,
    t2s_track: String,
    stop_status: String,
    stop_type: String,
    track_change: Option<bool>,
    local_cancel: Option<bool>,
    bus: bool,
    occupancy: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainDetails {
    headsign: String,
    summary: String,
    peak_code: String,
    branch: Option<String>,
    stops: Vec<TrainStop>,
    direction: String,
    turf: Option<TrainTurf>,
    //"PERMITTED" or "PROHIBITED"
    bike_rule: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MtaTrain {
    train_id: String,
    //MNR or LIRR
    railroad: String,
    run_date: String,
    train_num: String,
    realtime: bool,
    details: TrainDetails,
    consist: TrainConsist,
    location: TrainLocation,
    status: TrainStatus,
}

fn get_lirr_train_id(entity: &gtfs_rt::FeedEntity) -> String {
    let mut train_id = String::from("");

    if entity.vehicle.is_some() {
        let vehicle = entity.vehicle.as_ref().unwrap();

        if vehicle.trip.is_some() {
            let trip = vehicle.trip.as_ref().unwrap();

            if trip.trip_id.is_some() {
                let pre_train_id = trip.trip_id.as_ref().unwrap().clone();

                //split by underscore

                let split: Vec<&str> = pre_train_id.split("_").collect();

                //get last element

                train_id = String::from(split[split.len() - 1]);
            }
        }
    }

    train_id
}

fn convert(
    mta: &Vec<MtaTrain>,
    railroad: &str,
    input_gtfs_trips: &gtfs_rt::FeedMessage,
) -> Vec<gtfs_rt::FeedEntity> {
    mta.iter()
        .filter(|mta| mta.railroad.as_str() == railroad)
        .map(|mta| {
            let mut supporting_gtfs: Option<gtfs_rt::FeedEntity> = None;

            let candidates_for_id: Vec<gtfs_rt::FeedEntity> = input_gtfs_trips
                .entity
                .clone()
                .into_iter()
                .filter(|mta_entity| {
                    let status: bool = match mta.railroad.as_str() {
                        "MNR" => mta.train_num == mta_entity.id,
                        "LIRR" => mta.train_num == get_lirr_train_id(&mta_entity),
                        _ => false,
                    };

                    status
                })
                .collect::<Vec<gtfs_rt::FeedEntity>>();

            if candidates_for_id.len() >= 1 {
                supporting_gtfs = Some(candidates_for_id[0].clone());
            }

            if mta.railroad == "LIRR" {
                //filter for vehicle only
                let candidates_for_id = candidates_for_id
                    .into_iter()
                    .filter(|mta_entity| mta_entity.vehicle.is_some())
                    .collect::<Vec<gtfs_rt::FeedEntity>>();

                if candidates_for_id.len() >= 1 {
                    supporting_gtfs = Some(candidates_for_id[0].clone());
                }
            }

            (mta, supporting_gtfs)
        })
        .map(|(mta, supporting_gtfs)| gtfs_rt::FeedEntity {
            id: mta.train_id.clone(),
            is_deleted: None,
            trip_update: None,
            alert: None,
            shape: None,
            vehicle: Some(gtfs_rt::VehiclePosition {
                vehicle: match &supporting_gtfs {
                    Some(supporting_gtfs) => {
                        supporting_gtfs.clone().vehicle.unwrap().vehicle.clone()
                    }
                    None => None,
                },
                trip: match mta.railroad.as_str() {
                    "MNR" => {
                        /*
                        let mut trip = gtfs_rt::TripDescriptor {
                        trip_id: supporting_gtfs.clone().unwrap().vehicle.unwrap().trip.clone().unwrap().trip_id,
                        route_id: supporting_gtfs.clone().unwrap().trip_update.unwrap().trip.route_id,
                        start_time: supporting_gtfs.clone().unwrap().vehicle.unwrap().trip.clone().unwrap().start_time,
                        start_date: supporting_gtfs.clone().unwrap().vehicle.unwrap().trip.clone().unwrap().start_date,
                        schedule_relationship: supporting_gtfs.clone().unwrap().vehicle.unwrap().trip.clone().unwrap().schedule_relationship,
                        direction_id: supporting_gtfs.clone().unwrap().vehicle.unwrap().trip.clone().unwrap().direction_id,
                    } */

                    
                    match &supporting_gtfs {
                        Some(supporting_gtfs) => {
                            
                                let mut trip = supporting_gtfs.clone().vehicle.unwrap().trip.clone();

                                //insert route id

                                //trip.route_id = supporting_gtfs.clone().trip_update.unwrap().trip.route_id.clone();

                                if trip.is_some() {
                                    let _ = match supporting_gtfs.trip_update.is_some() {
                                        true => {
                                            trip.as_mut().unwrap().route_id = supporting_gtfs.clone().trip_update.unwrap().trip.route_id.clone();
                                        },
                                        false => {
                                            trip.as_mut().unwrap().route_id = None;
                                        }
                                    };
                                }

                                trip
                            
                        },
                        None => None,

                    }
                }
                    ,
                    _ => match &supporting_gtfs {
                    Some(supporting_gtfs) => supporting_gtfs.clone().vehicle.unwrap().trip.clone(),
                    None => None,
                }},
                position: Some(gtfs_rt::Position {
                    latitude: mta.location.latitude,
                    longitude: mta.location.longitude,
                    bearing: mta.location.heading,
                    odometer: None,
                    speed: Some(mta.location.speed.unwrap_or(0.0) as f32 * 0.44704),
                }),
                current_stop_sequence: match &supporting_gtfs {
                    Some(supporting_gtfs) => supporting_gtfs
                        .clone()
                        .vehicle
                        .unwrap()
                        .current_stop_sequence
                        .clone(),
                    None => None,
                },
                stop_id: match &supporting_gtfs {
                    Some(supporting_gtfs) => {
                        supporting_gtfs.clone().vehicle.unwrap().stop_id.clone()
                    }
                    None => None,
                },
                current_status: match &supporting_gtfs {
                    Some(supporting_gtfs) => supporting_gtfs
                        .clone()
                        .vehicle
                        .unwrap()
                        .current_status
                        .clone(),
                    None => None,
                },
                timestamp: Some(mta.location.timestamp as u64),
                congestion_level: None,
                occupancy_status: None,
                multi_carriage_details: vec![],
                occupancy_percentage: None,
            }),
        })
        .collect::<Vec<gtfs_rt::FeedEntity>>()
}

async fn get_mta_trips(client: &reqwest::Client, url: &str, api_key: &str) -> gtfs_rt::FeedMessage {
    let bytes = client
        .get(url)
        .header("x-api-key", api_key)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap()
        .to_vec();

    let decoded: gtfs_rt::FeedMessage = gtfs_rt::FeedMessage::decode(bytes.as_slice()).unwrap();

    decoded
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    // curl https://transloc-api-1-2.p.rapidapi.com/vehicles.json?agencies=1039
    //-H "X-Mashape-Key: b0ebd9e8a5msh5aca234d74ce282p1737bbjsnddd18d7b9365"

    let redisclient = RedisClient::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let client = reqwest::Client::new();

    //exposed on purpose. I don't care.
    let key = "hvThsOlHmP2XzvYWlKKC17YPcq07meIg2V2RPLbC";

    const LIRR_TRIPS_FEED: &str =
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/lirr%2Fgtfs-lirr";

    const MNR_TRIPS_FEED: &str =
        "https://api-endpoint.mta.info/Dataservice/mtagtfsfeeds/mnr%2Fgtfs-mnr";

    loop {
        //every 4 seconds

        let beginning = Instant::now();

        // println!("Inserted into Redis!");

        let request = client
            .get("https://backend-unified.mylirr.org/locations?geometry=TRACK_TURF&railroad=BOTH")
            .header("Accept-Version", "3.0")
            .send()
            .await
            .unwrap();

        println!("Downloaded!");

        //deserialise json into struct

        let body = request.text().await.unwrap();

        let import_data: Vec<MtaTrain> = serde_json::from_str(body.as_str()).unwrap();

        let input_gtfs_lirr = get_mta_trips(&client, LIRR_TRIPS_FEED, &key).await;
        let input_gtfs_mnr = get_mta_trips(&client, MNR_TRIPS_FEED, &key).await;

        let vehiclepositions_lirr: Vec<gtfs_rt::FeedEntity> =
            convert(&import_data, "LIRR", &input_gtfs_lirr);
        let vehiclepositions_mnr: Vec<gtfs_rt::FeedEntity> =
            convert(&import_data, "MNR", &input_gtfs_mnr);

        insert_gtfs_rt(
            &mut con,
            &gtfs_rt::FeedMessage {
                header: gtfs_rt::FeedHeader {
                    gtfs_realtime_version: "2.0".to_string(),
                    incrementality: Some(gtfs_rt::feed_header::Incrementality::FullDataset as i32),
                    timestamp: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    ),
                },
                entity: vehiclepositions_lirr,
            },
            &"f-mta~nyc~rt~lirr".to_string(),
            &"vehicles".to_string(),
        );

        insert_gtfs_rt(
            &mut con,
            &gtfs_rt::FeedMessage {
                header: gtfs_rt::FeedHeader {
                    gtfs_realtime_version: "2.0".to_string(),
                    incrementality: Some(gtfs_rt::feed_header::Incrementality::FullDataset as i32),
                    timestamp: Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    ),
                },
                entity: vehiclepositions_mnr,
            },
            &"f-mta~nyc~rt~mnr".to_string(),
            &"vehicles".to_string(),
        );

        //println!("{:?}", import_data);

        let time_left = 500 as f64 - (beginning.elapsed().as_millis() as f64);

        if time_left > 0.0 {
            println!("Sleeping for {} milliseconds", time_left);
            std::thread::sleep(std::time::Duration::from_millis(time_left as u64));
        }
    }
}
