use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use redis::Commands;
extern crate qstring;
use qstring::QString;
use std::io::BufReader;
 use std::fs::File;
 use csv::ReaderBuilder;
 use std::time::Instant;

use protobuf::{CodedInputStream, Message};

use kactus::gtfs_realtime;
use kactus::gtfs_realtime::FeedMessage;

use protobuf_json_mapping::print_to_string;

use serde::{Serialize};

#[derive(Serialize)]
pub struct feedtimes {
    feed: String,
    vehicles: Option<u64>,
    trips: Option<u64>,
    alerts: Option<u64>,
    has_vehicles: bool,
    has_trips: bool,
    has_alerts: bool,
  }

async fn index(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Server", "Kactus"))
        .insert_header(("Content-Type", "text/plain"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .body("Hello world!")
}

async fn gtfsrt(req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let query_str = req.query_string(); // "name=ferret"
    let qs = QString::from(query_str);
    let feed = qs.get("feed");

    match feed {
        Some(feed) => {
            let category = qs.get("category");

            //HttpResponse::Ok().body(format!("Requested {}/{}", feed, category))

            match category {
                Some(category) => {
                    let doesexist =
                        con.exists::<String, bool>(format!("gtfsrtvalid|{}|{}", feed, category));

                    match doesexist {
                        Ok(data) => {
                            if data == false {
                                return HttpResponse::NotFound()
                                    .insert_header(("Content-Type", "text/plain"))
                                    .insert_header(("Server", "Kactus"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
                                    .body("Error: Data Not Found\n");
                            } else {
                                let data = con.get::<String, Vec<u8>>(format!(
                                    "gtfsrt|{}|{}",
                                    feed, category
                                ));

                                match data {
                                    Ok(data) => 
                                        HttpResponse::Ok()
                                        .insert_header((
                                            "Content-Type",
                                            "application/x-google-protobuf",
                                        ))
                                        .insert_header(("Server", "Kactus"))
                                        
        .insert_header(("Access-Control-Allow-Origin", "*"))
                                        .body(data),
                                    Err(e) => HttpResponse::NotFound()
                                        .insert_header(("Content-Type", "text/plain"))
                                        .insert_header(("Server", "Kactus"))
                                        
        .insert_header(("Access-Control-Allow-Origin", "*"))
                                        .body(format!("Error: {}\n", e)),
                                }
                            }
                        }
                        Err(e) => {
                            return HttpResponse::NotFound()
                                .insert_header(("Content-Type", "text/plain"))
                                .insert_header(("Server", "Kactus"))
                                .insert_header(("Access-Control-Allow-Origin", "*"))
                                .body(format!("Error in connecting to redis\n"))
                        }
                    }
                }
                None => {
                    return HttpResponse::NotFound()
                        .insert_header(("Content-Type", "text/plain"))
                        .insert_header(("Server", "Kactus"))
                        .insert_header(("Access-Control-Allow-Origin", "*"))
                        .body("Error: No category specified\n")
                }
            }
        }
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .insert_header(("Access-Control-Allow-Origin", "*"))
                .insert_header(("Server", "Kactus"))
                .body("Error: No feed specified\n")
        }
    }
}

fn parse_protobuf_message(bytes: &[u8]) -> Result<FeedMessage, protobuf::Error> {
    return gtfs_realtime::FeedMessage::parse_from_bytes(bytes);
}

async fn gtfsrttimes(req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

      // Open the CSV file
      let file = File::open("urls.csv").unwrap();
      let mut reader = csv::Reader::from_reader(BufReader::new(file));

      let mut vecoftimes: Vec<feedtimes> = Vec::new();

        let startiterator = Instant::now();

      // Iterate over each record (line) in the CSV file
      for result in reader.records() {
          // Unwrap the record
          let record = result.unwrap();
  
          // Check the number of fields in the record
          if record.len() > 0 {
              // Print the first field of the record

                let feed = record.get(0).unwrap();  

                let vehicles = con.get::<String, u64>(format!("gtfsrttime|{}|vehicles", feed));
                let trips = con.get::<String, u64>(format!("gtfsrttime|{}|trips", feed));
                let alerts = con.get::<String, u64>(format!("gtfsrttime|{}|alerts", feed));

                let vehicles = match vehicles {
                    Ok(data) => Some(data),
                    Err(e) => None
                };

                let trips = match trips {
                    Ok(data) => Some(data),
                    Err(e) => None
                };

                let alerts = match alerts {
                    Ok(data) => Some(data),
                    Err(e) => None
                };

                let has_vehicles = !(record.get(1).unwrap().is_empty());
                let has_trips = !(record.get(2).unwrap().is_empty());
                let has_alerts = !(record.get(3).unwrap().is_empty());

                let feedtime = feedtimes {
                    feed: feed.to_string(),
                    vehicles: vehicles,
                    trips: trips,
                    alerts: alerts,
                    has_vehicles: has_vehicles,
                    has_trips: has_trips,
                    has_alerts: has_alerts
                };

                vecoftimes.push(feedtime);
          }
      }

      let finishiterator = startiterator.elapsed();

      println!("reading file took {:#?}", finishiterator);

        let json = serde_json::to_string(&vecoftimes).unwrap();

        HttpResponse::Ok()
            .insert_header(("Content-Type", "application/json"))
            .insert_header(("Server", "Kactus"))
            .insert_header(("Access-Control-Allow-Origin", "*"))
            .body(format!("{}\n", json))
}

async fn gtfsrttojson(req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let query_str = req.query_string(); // "name=ferret"
    let qs = QString::from(query_str);
    let feed = qs.get("feed");

    match feed {
        Some(feed) => {
            let category = qs.get("category");

            //HttpResponse::Ok().body(format!("Requested {}/{}", feed, category))

            match category {
                Some(category) => {
                    let doesexist =
                        con.exists::<String, bool>(format!("gtfsrtvalid|{}|{}", feed, category));

                    match doesexist {
                        Ok(data) => {
                            if data == false {
                                return HttpResponse::NotFound()
                                    .insert_header(("Content-Type", "text/plain"))
                                    .insert_header(("Access-Control-Allow-Origin", "*"))
                                    .insert_header(("Server", "Kactus"))
                                    .body("Error: Data Not Found\n");
                            } else {
                                let data = con.get::<String, Vec<u8>>(format!(
                                    "gtfsrt|{}|{}",
                                    feed, category
                                ));

                                match data {
                                    Ok(data) => {
                                        let proto  = parse_protobuf_message(&data);

                                        match proto {
                                            Ok(proto) => {
                                                let protojson = print_to_string(&proto).unwrap();

                                                HttpResponse::Ok()
                                            .insert_header((
                                                "Content-Type",
                                                "application/json",
                                            ))
                                            .insert_header(("Server", "Kactus"))
                                            .insert_header(("Access-Control-Allow-Origin", "*"))
                                            .body(protojson)
                                            },
                                            Err(proto) => {
                                                println!("Error parsing protobuf");

                                                HttpResponse::NotFound()
                                                    .body("Parse protobuf failed")
                                            }
                                        }
                                    },
                                    Err(e) => HttpResponse::NotFound()
                                        .insert_header(("Content-Type", "text/plain"))
                                        .insert_header(("Server", "Kactus"))
                                        .insert_header(("Access-Control-Allow-Origin", "*"))
                                        .body(format!("Error: {}\n", e)),
                                }
                            }
                        }
                        Err(e) => {
                            return HttpResponse::NotFound()
                                .insert_header(("Content-Type", "text/plain"))
                                .insert_header(("Server", "Kactus"))
                                .insert_header(("Access-Control-Allow-Origin", "*"))
                                .body(format!("Error in connecting to redis\n"))
                        }
                    }
                }
                None => {
                    return HttpResponse::NotFound()
                        .insert_header(("Content-Type", "text/plain"))
                        .insert_header(("Server", "Kactus"))
                        .insert_header(("Access-Control-Allow-Origin", "*"))
                        .body("Error: No category specified\n")
                }
            }
        }
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .insert_header(("Server", "Kactus"))
                
                .insert_header(("Access-Control-Allow-Origin", "*"))
                .body("Error: No feed specified\n")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a new HTTP server.
    let builder = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/gtfsrt/", web::get().to(gtfsrt))
            .route("/gtfsrt", web::get().to(gtfsrt))
            .route("/gtfsrtasjson/", web::get().to(gtfsrttojson))
            .route("/gtfsrtasjson", web::get().to(gtfsrttojson))
            .route("/gtfsrttimes", web::get().to(gtfsrttimes))
            .route("/gtfsrttimes/", web::get().to(gtfsrttimes))
    })
    .workers(4);

    // Bind the server to port 8080.
    let _ = builder.bind("127.0.0.1:8080").unwrap().run().await;

    Ok(())
}
