use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct from_octa {
    #[serde(rename(deserialize = "Vehicle_ID"))]
    vehicle_id: String,
    #[serde(rename(deserialize = "DRIVER_ID"))]
    driver_id: String,
    #[serde(rename(deserialize = "RunID"))]
    run_id: String,
    #[serde(rename(deserialize = "BlockID"))]
    block_id: String
}

fn main() {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let client = ReqwestClient::new();

    loop {
        
    }
}`