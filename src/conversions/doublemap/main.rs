use gtfs_rt::FeedEntity;
use gtfs_rt::VehiclePosition;
use reqwest::Client as ReqwestClient;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

#[derive(serde::Deserialize, Debug)]
struct DoubleMapStop {
    id: i32,
    name: String,
    code: Option<String>,
    ivr_code: Option<String>,
    description: Option<String>,
    lat: f64,
    lon: f64,
    buddy: Option<i32>,
}

#[derive(serde::Deserialize, Debug)]
struct DoubleMapBus {
    id: i32,
    name: String,
    lat: f32,
    lon: f32,
    heading: f32,
    route: Option<i32>,
    #[serde(rename = "lastStop")]
    last_stop: Option<i32>,
    bus_type: String,
    #[serde(rename = "lastUpdate")]
    last_update: i64,
}

#[derive(serde::Deserialize)]
struct RouteOutPostgres {
    onestop_feed_id: String,
    route_id: String,
    short_name: String,
    long_name: String,
    desc: String,
    route_type: i16,
    url: Option<String>,
    agency_id: Option<String>,
    gtfs_order: Option<i32>,
    color: String,
    text_color: String,
    continuous_pickup: i16,
    continuous_drop_off: i16,
    shapes_list: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
struct DoubleMapRoute {
    id: i32,
    name: String,
    short_name: Option<String>,
    description: String,
    color: String,
    path: Vec<f32>,
    start_time: Option<String>,
    end_time: Option<String>,
    schedule_url: Option<String>,
    active: bool,
}

#[tokio::main]
async fn main() {
    let client = ReqwestClient::new();

    let routes_static = client
        .get("https://backend.catenarymaps.org/getroutesperagency?feed_id=f-c3j-757")
        .send()
        .await;

    let routes_static_parsed: Vec<RouteOutPostgres> =
        serde_json::from_str(&routes_static.unwrap().text().await.unwrap()).unwrap();

    loop {
        //sleep for 1 sec

        let stops = client
            .get("https://roamtransit.doublemap.com/map/v2/stops")
            .send()
            .await;
        let routes = client
            .get("https://roamtransit.doublemap.com/map/v2/routes")
            .send()
            .await;
        let vehicles = client
            .get("https://roamtransit.doublemap.com/map/v2/buses")
            .send()
            .await;

        if stops.is_ok() {
            //println!("Stops: {:#?}", stops.unwrap().text().await.unwrap());
        }

        if vehicles.is_ok() && stops.is_ok() && routes.is_ok() {
            let stops_parsed: Vec<DoubleMapStop> =
                serde_json::from_str(&stops.unwrap().text().await.unwrap()).unwrap();

            let routes_parsed: Vec<DoubleMapRoute> =
                serde_json::from_str(&routes.unwrap().text().await.unwrap()).unwrap();

            let vehicles_parsed: Vec<DoubleMapBus> =
                serde_json::from_str(&vehicles.unwrap().text().await.unwrap()).unwrap();

            println!("Vehicles: {:#?}", vehicles_parsed);

            let doublemap_routes_to_static: HashMap<i32, Option<String>> =
                HashMap::from_iter(routes_parsed.into_iter().map(|route_original| {
                    let route = routes_static_parsed
                        .iter()
                        .find(|route_static| route_static.short_name == *(route_original.short_name.as_ref().unwrap()));

                    match route {
                        Some(route) => (route_original.id, Some(route.route_id.clone())),
                        None => {(route_original.id, None)}
                    }
                }));

            let mut feed_entities: Vec<FeedEntity> = Vec::from_iter(vehicles_parsed.into_iter().map(|doublemap_vehicle| 
                FeedEntity {
                    is_deleted: Some(false),
                    shape: None,
                    id: doublemap_vehicle.id.to_string(),
                    vehicle: Some(VehiclePosition {
                        occupancy_percentage: None,
                        multi_carriage_details: vec![],
                        trip: Some(
                            gtfs_rt::TripDescriptor {
                                route_id: doublemap_routes_to_static.get(&doublemap_vehicle.route.unwrap()).unwrap().clone(),
                                start_time: None,
                                start_date: None,
                                schedule_relationship: None,
                                trip_id: Some(doublemap_vehicle.id.to_string()),
                                direction_id: None,
                            }
                        ),
                        vehicle: Some(gtfs_rt::VehicleDescriptor {
                            id: Some(doublemap_vehicle.name.to_string()),
                            label: Some(doublemap_vehicle.name),
                            license_plate: None,
                            wheelchair_accessible: None,
                        }),
                        position: Some(gtfs_rt::Position {
                            latitude: doublemap_vehicle.lat,
                            longitude: doublemap_vehicle.lon,
                            bearing: Some(doublemap_vehicle.heading),
                            speed: None,
                            odometer: None,
                        }),
                        current_stop_sequence: None,
                        stop_id: None,
                        current_status: None,
                        timestamp: Some(doublemap_vehicle.last_update.try_into().unwrap()),
                        congestion_level: None,
                        occupancy_status: None,
                    }),
                    trip_update: None,
                    alert: None,
                }
            ));
        };
        
    // Sleep for 0.5 seconds
    thread::sleep(Duration::from_millis(500));
    }

}
