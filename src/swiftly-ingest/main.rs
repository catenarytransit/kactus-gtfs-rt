use serde::{Deserialize,Serialize};
use std::error::Error;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutesDetailsFile {
    agency: String,
    routes: Vec<RoutesDetailsEntity>
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteDirections {
    id: String,
    title: String,
    stops: Vec<RouteStops>,
    headsigns: Vec<String>
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteStops {
    id: String,
    lat: f32,
    lon: f32,
    name: String,
    code: i32
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoutesDetailsEntity {
    id: String,
    name: String,
    short_name: String,
    long_name: String,
    color: Option<String>,
    text_color: Option<String>,
    #[serde(rename = "type")]
    route_type: String,
    directions: Vec<RouteDirections>,
    shapes: Vec<SwiftlyShape>
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwiftlyShape {
    trip_pattern_id: String,
    shape_id: String,
    direction_id: String,
    headsign: String,
    minor: Option<bool>,
    locs: Vec<SwiftlyShapePoint>
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwiftlyShapePoint {
    lat: f32,
    lon: f32
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwiftlyVehicleInfo {
    id: String,
    route_id: String,
    route_short_name: String,
    route_name: String,
    headsign: String,
    direction: String,
    vehicle_type: String,
    sch_adh_secs: f32,
    sch_adh: f32,
    headway_secs: f32,
    scheduled_headway_secs: f32,
    previous_vehicle_id: String,
    //omit useless stuff
    driver: String,
    loc: SwiftlyVehicleLoc,
    in_yard: bool
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwiftlyVehicleLoc {
    lat: f32,
    lon: f32,
    time: u64,
    speed: f32,
    heading: f32,
    source: String
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwiftlyPerRouteFile {
    vehicles: Vec<SwiftlyVehicleInfo>
}

async fn fetch_per_route_details(client: &reqwest::Client, swiftly_id:&String, route_id: &String) -> Result<SwiftlyPerRouteFile, Box<dyn Error>> {
    let url = format!("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/agency/{}/command/vehiclesDetails?r={}", &swiftly_id, &route_id);

    let request = client.get(url).send().await;

    match request {
        Ok(resp) => {
            match resp.text().await {
                Ok(text) => {
                    match serde_json::from_str::<SwiftlyPerRouteFile>(text.as_str()) {
                        Ok(routes_data) => {
                            println!("Vehicles per route file fetched and parsed successfully");
                            Ok(routes_data)
                        },
                        Err(e) => {
                            println!("Error parsing vehicles per route file: {}",e);
                            Err(Box::new(e))
                        }
                    }
                },
                Err(e) => {
                    println!("Error getting text of vehicles per route fetch: {}",e);
                    return Err(Box::new(e));
                }
            }
            
        },
        Err(e) => {
            println!("Error fetching routes file: {}",e);
            Err(Box::new(e))
        }
    }
}

async fn fetch_routes_file(client: &reqwest::Client,swiftly_id:&String) -> Result<RoutesDetailsFile,Box<dyn std::error::Error>> {
    let routes_url = get_routes_url(swiftly_id);

    println!("{}",&routes_url);

    match client.get(&routes_url).send().await {
        Ok(resp) => {
            match resp.text().await {
                Ok(text) => {
                    match serde_json::from_str::<RoutesDetailsFile>(text.as_str()) {
                        Ok(routes_data) => {
                            println!("Routes file fetched and parsed successfully");
                            Ok(routes_data)
                        },
                        Err(e) => {
                            println!("Error parsing routes file: {}",e);
                            Err(Box::new(e))
                        }
                    }
                },
                Err(e) => {
                    println!("Error getting text of routes fetch: {}",e);
                    return Err(Box::new(e));
                }
            }
            
        },
        Err(e) => {
            println!("Error fetching routes file: {}",e);
            Err(Box::new(e))
        }
    }
}

async fn fetch_swiftly(client: &reqwest::Client,swiftly_id:&String) -> () {

    let swiftly_routes_file = fetch_routes_file(&client,&swiftly_id).await;

    match swiftly_routes_file {
        Ok(swiftly_routes_file) => {

        },
        _ => {

        }
    }
}

fn get_routes_url(swiftly_id: &String) -> String {
    format!("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/agency/{}/command/routesDetails", swiftly_id)
}

#[tokio::main]
async fn main() {
    let client =reqwest::Client::new();
    loop {
        fetch_swiftly(&client, &"octa".to_string()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_routes_url_test() {
        let swiftly_id = "octa".to_string();
        let routes_url = get_routes_url(&swiftly_id);
        assert_eq!(routes_url, String::from("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/agency/octa/command/routesDetails"));
    }

    #[tokio::test]
    async fn get_routes_and_parse() {
        let client = reqwest::Client::new();
        let routes_file_octa = fetch_routes_file(&client, &String::from("octa")).await;
        assert!(routes_file_octa.is_ok());

        let routes_file_la_bus = fetch_routes_file(&client, &String::from("lametro")).await;
        assert!(routes_file_la_bus.is_ok());

        let routes_file_la_rail = fetch_routes_file(&client, &String::from("lametro-rail")).await;
        assert!(routes_file_la_rail.is_ok());
    }

    #[tokio::test]
    async fn get_veh_per_route_and_parse() {
        let client = reqwest::Client::new();
        let routes_file_octa_bus_79 = fetch_per_route_details(&client, &String::from("octa"),  &String::from("79")).await;
        let routes_file_la_bus_910 = fetch_per_route_details(&client, &String::from("lametro"),  &String::from("910-13168")).await;
        assert!(routes_file_octa_bus_79.is_ok());
        assert!(routes_file_la_bus_910.is_ok());
    }
}