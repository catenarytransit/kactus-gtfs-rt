use actix_web::{
    middleware::DefaultHeaders, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use redis::Commands;
extern crate qstring;
use csv::ReaderBuilder;
use qstring::QString;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;

use protobuf::{CodedInputStream, Message};

use kactus::gtfs_realtime;
use kactus::gtfs_realtime::FeedMessage;

use protobuf_json_mapping::print_to_string;

use serde::Serialize;

#[derive(Serialize)]
pub struct feedtimes {
    feed: String,
    vehicles: Option<u64>,
    trips: Option<u64>,
    alerts: Option<u64>,
    /*
    has_vehicles: bool,
    has_trips: bool,
    has_alerts: bool,*/
}

async fn index(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/plain"))
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
                        con.get::<String, u64>(format!("gtfsrttime|{}|{}", &feed, &category));

                    match doesexist {
                        Ok(timeofcache) => {
                            let data = con
                                .get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", &feed, &category));

                            match data {
                                Ok(data) => {
                                    let timeofclientcache = qs.get("timeofcache");

                                    if timeofclientcache.is_some() {
                                        let timeofclientcache = timeofclientcache.unwrap();

                                        let timeofclientcache = (*timeofclientcache).parse::<u64>();

                                        if timeofclientcache.is_ok() {
                                            let timeofclientcache = timeofclientcache.unwrap();

                                            if timeofclientcache >= timeofcache {
                                                return HttpResponse::NoContent().body("");
                                            }

                                            let proto = parse_protobuf_message(&data);

                                            match proto {
                                                Ok(proto) => {
                                                    let headertimestamp = proto.header.timestamp;

                                                    if (headertimestamp.is_some()) {
                                                        if timeofclientcache
                                                            >= headertimestamp.unwrap()
                                                        {
                                                            return HttpResponse::NoContent()
                                                                .body("");
                                                        }
                                                    }
                                                }
                                                Err(bruh) => {

                                                    println!("{:#?}",bruh);

                                                    return HttpResponse::InternalServerError()
                                                        .body("protobuf failed to parse");
                                                }
                                            }
                                        }
                                    }

                                    HttpResponse::Ok()
                                        .insert_header((
                                            "Content-Type",
                                            "application/x-google-protobuf",
                                        ))
                                        .body(data)
                                }
                                Err(e) => {
                                    println!("Error: {:?}", e);
                                    HttpResponse::NotFound()
                                        .insert_header(("Content-Type", "text/plain"))
                                        .body(format!("Error: {}\n", e))
                                }
                            }
                        }
                        Err(e) => {
                            return HttpResponse::NotFound()
                                .insert_header(("Content-Type", "text/plain"))
                                .body(format!("Error in connecting to redis\n"));
                        }
                    }
                }
                None => {
                    return HttpResponse::NotFound()
                        .insert_header(("Content-Type", "text/plain"))
                        .body("Error: No category specified\n")
                }
            }
        }
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
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

    let mut vecoftimes: Vec<feedtimes> = Vec::new();

    let startiterator = Instant::now();

    let keys = con.keys::<String, Vec<String>>(String::from("gtfsrtexists|*"));

    match keys {
        Ok(data) => {
            let mut keys: Vec<String> = data;

            keys.sort();

            for key in keys.iter_mut() {
                let feed = key.replace("gtfsrtexists|", "");

                // Print the first field of the record

                let vehicles = con.get::<String, u64>(format!("gtfsrttime|{}|vehicles", feed));
                let trips = con.get::<String, u64>(format!("gtfsrttime|{}|trips", feed));
                let alerts = con.get::<String, u64>(format!("gtfsrttime|{}|alerts", feed));

                let vehicles = match vehicles {
                    Ok(data) => Some(data),
                    Err(e) => None,
                };

                let trips = match trips {
                    Ok(data) => Some(data),
                    Err(e) => None,
                };

                let alerts = match alerts {
                    Ok(data) => Some(data),
                    Err(e) => None,
                };

                let feedtime = feedtimes {
                    feed: feed.clone(),
                    vehicles: vehicles,
                    trips: trips,
                    alerts: alerts,
                };

                vecoftimes.push(feedtime);
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
            return HttpResponse::InternalServerError()
                .insert_header(("Content-Type", "text/plain"))
                .body(format!("Error: {}\n", e));
        }
    };

    let finishiterator = startiterator.elapsed();

    println!("reading file took {:#?}", finishiterator);

    let json = serde_json::to_string(&vecoftimes).unwrap();

    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
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
                        con.get::<String, u64>(format!("gtfsrttime|{}|{}", feed, category));

                    match doesexist {
                        Ok(timeofcache) => {
                            let data =
                                con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", feed, category));

                            match data {
                                Ok(data) => {
                                    let proto = parse_protobuf_message(&data);

                                    match proto {
                                        Ok(proto) => {
                                            let protojson = print_to_string(&proto).unwrap();

                                            HttpResponse::Ok()
                                                .insert_header(("Content-Type", "application/json"))
                                                .body(protojson)
                                        }
                                        Err(proto) => {
                                            println!("Error parsing protobuf");

                                            HttpResponse::NotFound().body("Parse protobuf failed")
                                        }
                                    }
                                }
                                Err(e) => HttpResponse::NotFound()
                                    .insert_header(("Content-Type", "text/plain"))
                                    .body(format!("Error: {}\n", e)),
                            }
                        }
                        Err(e) => {
                            return HttpResponse::NotFound()
                                .insert_header(("Content-Type", "text/plain"))
                                .body(format!("Error in connecting to redis\n"))
                        }
                    }
                }
                None => {
                    return HttpResponse::NotFound()
                        .insert_header(("Content-Type", "text/plain"))
                        .body("Error: No category specified\n")
                }
            }
        }
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No feed specified\n")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a new HTTP server.
    let builder = HttpServer::new(|| {
        App::new()
            .wrap(
                DefaultHeaders::new()
                    .add(("Server", "Kactus"))
                    .add(("Access-Control-Allow-Origin", "*")),
            )
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
