extern crate amtrak_gtfs_rt;

use prost::Message;
use redis::Client as RedisClient;

use kactus::insert::insert_gtfs_rt;
use kactus::insert::insert_gtfs_rt_bytes;

use kactus::aspen::send_to_aspen;

#[tokio::main]
async fn main() {
    let redisclient = RedisClient::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let client = reqwest::Client::new();
    loop {
        let amtrak_raw_data = amtrak_gtfs_rt::fetch_amtrak_gtfs_rt(&client).await.unwrap();

        let vehicle_data = amtrak_raw_data.vehicle_positions.encode_to_vec();

        insert_gtfs_rt_bytes(
            &mut con,
            &vehicle_data,
            &"f-amtrak~rt".to_string(),
            &"vehicles".to_string(),
        );

        send_to_aspen(
            "f-amtrak~rt",
            &Some(vehicle_data),
            &None,
            &None,
            true,
            false,
            false,
            false,
        )
        .await;

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
