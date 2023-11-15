use serde::{Deserialize,Serialize};

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

async fn fetch_swiftly(swiftly_id:&String) -> () {
    
}

fn get_routes_url(swiftly_id: &String) -> String {
    format!("https://transitime-api.goswift.ly/api/v1/key/81YENWXv/agency/{}/command/routesDetails", swiftly_id)
}

#[tokio::main]
async fn main() {
    loop {
        fetch_swiftly(&"octa".to_string()).await;
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
}