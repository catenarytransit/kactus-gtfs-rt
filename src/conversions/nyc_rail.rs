
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;
use protobuf::{CodedInputStream, Message as ProtobufMessage};
use prost::Message;
use std::time::UNIX_EPOCH;
use gtfs_rt::EntitySelector;
use gtfs_rt::TimeRange;
use serde_json;

use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};

use std::time::{Duration, SystemTime};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainStatus {
    otp: i32,
    otp_location: String,
    held: bool,
    canceled: bool
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
    locomotive: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainConsist {
    cars: Vec<TrainCar>,
    fleet: Option<String>,
    actual_len: Option<i32>,
    sched_len: Option<i32>,
    occupancy: Option<String>,
    occupancy_timestamp: Option<i32>
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
    extra_info: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainTurf {
    length: f32,
    location_mp: f32
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
    bike_rule: String
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
   // curl https://transloc-api-1-2.p.rapidapi.com/vehicles.json?agencies=1039 
   //-H "X-Mashape-Key: b0ebd9e8a5msh5aca234d74ce282p1737bbjsnddd18d7b9365"

   let redisclient = RedisClient::open("redis://127.0.0.1:6379/").unwrap();
   let mut con = redisclient.get_connection().unwrap();

    let client = reqwest::Client::new();

    loop {
        //every 4 seconds

        let beginning = Instant::now();

       // println!("Inserted into Redis!");

        let request = client.get("https://backend-unified.mylirr.org/locations?geometry=TRACK_TURF&railroad=BOTH")
            .header("Accept-Version", "3.0")
            .send()
            .await
            .unwrap();

        println!("Downloaded!");

        //deserialise json into struct

        let body = request.text().await.unwrap();

        let import_data: Vec<MtaTrain> = serde_json::from_str(body.as_str()).unwrap();

        //println!("{:?}", import_data);  

        let time_left = 500 as f64 - (beginning.elapsed().as_millis() as f64);

        if time_left > 0.0 {
            println!("Sleeping for {} milliseconds", time_left);
            std::thread::sleep(std::time::Duration::from_millis(time_left as u64));
        }
    }
}