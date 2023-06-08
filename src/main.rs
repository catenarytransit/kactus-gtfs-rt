use redis::Commands;
use redis::RedisError;
use redis::{Client as RedisClient, RedisResult};
use reqwest::Client as ReqwestClient;

#[tokio::main]
async fn main() {
    /* do something here */

    let url = "https://api.octa.net/GTFSRealTime/protoBuf/VehiclePositions.aspx";

    loop {
        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();

        let _: () = con.set("my_key", 42).unwrap();

        let reqwest_client = ReqwestClient::new();
        let resp = reqwest_client.get(url).send().await;
        let bytes: Vec<u8> = resp.unwrap().bytes().await.unwrap().to_vec();

        println!("bytes: {}", bytes.len());

        con.set::<String, Vec<u8>, ()>("octa-vehicle".to_string(), bytes)
            .unwrap();

        println!("set octa-vehicle in the redis db");

        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
