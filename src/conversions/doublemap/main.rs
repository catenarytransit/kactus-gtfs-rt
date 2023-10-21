
use std::thread;
use std::time::Duration;
use reqwest::Client as ReqwestClient;

#[derive(serde::Deserialize, Debug)]
struct DoubleMapBus {
    id: i32,
    name: String,
    code: Option<String>,
    ivr_code: Option<String>,
    description: Option<String>,
    lat: f64,
    lon: f64
}

#[tokio::main]
async fn main() {

    let client = ReqwestClient::new();

    loop {
        //sleep for 1 sec

        let stops = client.get("https://roamtransit.doublemap.com/map/v2/stops").send().await;
        let routes = client.get("https://roamtransit.doublemap.com/map/v2/routes").send().await;
        let vehicles = client.get("https://roamtransit.doublemap.com/map/v2/buses").send().await;

        if stops.is_ok() {
            //println!("Stops: {:#?}", stops.unwrap().text().await.unwrap());
        }

        if vehicles.is_ok() {
            let vehicles_parsed: Vec<DoubleMapBus> = serde_json::from_str(&vehicles.unwrap().text().await.unwrap()).unwrap();

            println!("Vehicles: {:#?}", vehicles_parsed);
        }

                // Sleep for 1 second
                thread::sleep(Duration::from_millis(500));
            
    }

}