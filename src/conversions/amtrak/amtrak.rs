extern crate amtrak_gtfs_rt;

use prost::Message;
use redis::Client as RedisClient;

use kactus::insert::insert_gtfs_rt;
use kactus::insert::insert_gtfs_rt_bytes;

use kactus::aspen::send_to_aspen;

use gtfs_structures::Gtfs;

pub fn filter_capital_corridor(input: gtfs_rt::FeedMessage) -> gtfs_rt::FeedMessage {
    let cc_route_id = "84";

    gtfs_rt::FeedMessage {
        entity: input
            .entity
            .into_iter()
            .filter(|item| {
                if item.vehicle.is_some() {
                    let vehicle = item.vehicle.as_ref().unwrap();
                    if vehicle.trip.is_some() {
                        let trip = vehicle.trip.as_ref().unwrap();
                        if trip.route_id.is_some() {
                            if trip.route_id.as_ref().unwrap().as_str() == cc_route_id {
                                return false;
                            }
                        }
                    }
                }

                if item.trip_update.is_some() {
                    let trip_update = item.trip_update.as_ref().unwrap();
                    let trip = &trip_update.trip;

                    if trip.route_id.is_some() {
                        let route_id = trip.route_id.as_ref().unwrap();

                        if route_id == cc_route_id {
                            return false;
                        }
                    }
                }

                true
            })
            .collect::<Vec<gtfs_rt::FeedEntity>>(),
        header: input.header,
    }
}

#[tokio::main]
async fn main() {
    let redisclient = RedisClient::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    println!("Downloading GTFS static");
    let gtfs = Gtfs::from_url_async("https://content.amtrak.com/content/gtfs/GTFS.zip")
        .await
        .unwrap();
    println!("Done downloading GTFS static");

    let client = reqwest::ClientBuilder::new()
        .deflate(true)
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    loop {
        let amtrak_gtfs_rt = amtrak_gtfs_rt::fetch_amtrak_gtfs_rt(&gtfs, &client)
            .await
            .unwrap();

        let vehicle_data =
            filter_capital_corridor(amtrak_gtfs_rt.vehicle_positions).encode_to_vec();
        let trip_data = filter_capital_corridor(amtrak_gtfs_rt.trip_updates).encode_to_vec();

        insert_gtfs_rt_bytes(
            &mut con,
            &vehicle_data,
            &"f-amtrak~rt".to_string(),
            &"vehicles".to_string(),
        );

        insert_gtfs_rt_bytes(
            &mut con,
            &trip_data,
            &"f-amtrak~rt".to_string(),
            &"trips".to_string(),
        );

        send_to_aspen(
            "f-amtrak~rt",
            &Some(vehicle_data),
            &Some(trip_data),
            &None,
            true,
            true,
            false,
            false,
        )
        .await;

        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
