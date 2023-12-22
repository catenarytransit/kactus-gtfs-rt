use actix::{Actor, StreamHandler};
use actix_web::{
    middleware::DefaultHeaders, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use gtfs_rt::{FeedMessage, FeedEntity, EntitySelector, FeedHeader};
use rand::Rng;
use redis::Commands;
extern crate qstring;

use kactus::parse_protobuf_message;
use qstring::QString;
use serde::Serialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub struct GtfsWs {
    feed: String,
    category: String,
    suicidebutton: bool,
    //skipfailure: Option<String>,
}

impl Actor for GtfsWs {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
        let mut con = redisclient.get_connection().unwrap();
        let feed = &self.feed;
        let category = &self.category;
        let doesexist = con.get::<String, u64>(format!("gtfsrttime|{}|{}", &feed, &category));
        if doesexist.is_err() {
            ctx.text(format!("Error in connecting to redis\n"));
            return ctx.close(None);
        }
        let data = con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", &feed, &category));
        if data.is_err() {
            println!("Error: {:?}", data);
            ctx.text(format!("Error: {:?}\n", data));
            return ctx.close(None);
        }
        let data = data.unwrap();
        if self.suicidebutton {
            return ctx.binary(data);
        }
        ctx.binary(data)
    }
}
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GtfsWs {
    fn handle(&mut self, _msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let _ = ctx;
    }
}

#[derive(Serialize)]
pub struct FeedTimes {
    feed: String,
    vehicles: Option<u64>,
    trips: Option<u64>,
    alerts: Option<u64>,
    /*
    has_vehicles: bool,
    has_trips: bool,
    has_alerts: bool,*/
}

async fn index(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/plain"))
        .body("Hello world!")
}

async fn gtfsrt(req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let qs = QString::from(req.query_string());
    let feed = match qs.get("feed") {
        Some(feed) => feed,
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No feed specified\n")
        }
    };
    let category = match qs.get("category") {
        Some(category) => category,
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No category specified\n")
        }
    };
    let doesexist = con.get::<String, u64>(format!("gtfsrttime|{}|{}", feed, category));
    if doesexist.is_err() {
        return HttpResponse::InternalServerError()
            .insert_header(("Content-Type", "text/plain"))
            .body(format!("Error in connecting to redis\n"));
    }
    let data = con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", &feed, &category));
    if data.is_err() {
        return HttpResponse::InternalServerError()
            .insert_header(("Content-Type", "text/plain"))
            .body(format!("Error: {:?}\n", data.unwrap()));
    }
    let data = data.unwrap();
    let suicidebutton = qs.get("suicidebutton");
    if suicidebutton.is_some() {
        let suicidebutton = suicidebutton.unwrap();
        if suicidebutton == "true" {
            return HttpResponse::Ok()
                .insert_header(("Content-Type", "application/x-google-protobuf"))
                .body(data);
        }
    }
    let timeofclientcache = qs.get("timeofcache");
    let proto = parse_protobuf_message(&data);
    let hashofresult = match proto {
        Ok(_) => fasthash::metro::hash64(data.as_slice()),
        Err(_) => {
            let mut rng = rand::thread_rng();
            rng.gen::<u64>()
        }
    };
    if timeofclientcache.is_some() {
        let timeofclientcache = timeofclientcache.unwrap();
        let timeofclientcache = (*timeofclientcache).parse::<u64>();
        if timeofclientcache.is_ok() {
            let timeofclientcache = timeofclientcache.unwrap();
            if timeofclientcache >= doesexist.unwrap() {
                return HttpResponse::NoContent().body("");
            }
            match &proto {
                Ok(proto) => {
                    let headertimestamp = proto.header.timestamp;
                    if headertimestamp.is_some() {
                        if timeofclientcache >= headertimestamp.unwrap() {
                            return HttpResponse::NoContent().body("");
                        }
                    }
                }
                Err(bruh) => {
                    println!("{:#?}", bruh);
                    let skipfailure = qs.get("skipfailure");
                    let mut allowcrash = true;
                    if skipfailure.is_some() {
                        if skipfailure.unwrap() == "true" {
                            allowcrash = false;
                        }
                    }
                    if allowcrash {
                        return HttpResponse::InternalServerError()
                            .body("protobuf failed to parse");
                    }
                }
            }
        }
        let hashofbodyclient = qs.get("bodyhash");
        if hashofbodyclient.is_some() {
            let hashofbodyclient = hashofbodyclient.unwrap();
            if (&proto).is_ok() {
                let clienthash = hashofbodyclient.parse::<u64>();
                if clienthash.is_ok() {
                    let clienthash = clienthash.unwrap();
                    if clienthash == hashofresult {
                        return HttpResponse::NoContent().body("");
                    }
                }
            }
        }
    }
    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/x-google-protobuf"))
        .insert_header(("hash", hashofresult))
        .body(data)
}

//NOT PROTOBUF this is the list of available agencies
async fn gtfsrttimes(_req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let mut vecoftimes: Vec<FeedTimes> = Vec::new();

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
                    Err(_e) => None,
                };

                let trips = match trips {
                    Ok(data) => Some(data),
                    Err(_e) => None,
                };

                let alerts = match alerts {
                    Ok(data) => Some(data),
                    Err(_e) => None,
                };

                let feedtime = FeedTimes {
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
    let qs = QString::from(req.query_string());
    let feed = match qs.get("feed") {
        Some(feed) => feed.to_string(),
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No feed specified\n")
        }
    };
    let category = match qs.get("category") {
        Some(category) => category.to_string(),
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No category specified\n")
        }
    };
    let usejson = match qs.get("raw") {
        Some(raw) => {
            if raw == "true" {
                false
            } else {
                true
            }
        }
        None => true,
    };
    let doesexist = con.get::<String, u64>(format!("gtfsrttime|{}|{}", feed, category));
    if doesexist.is_err() {
        return HttpResponse::InternalServerError()
            .insert_header(("Content-Type", "text/plain"))
            .body(format!("Error in connecting to redis\n"));
    }
    let data = con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", feed, category));
    if data.is_err() {
        return HttpResponse::InternalServerError()
            .insert_header(("Content-Type", "text/plain"))
            .body(format!("Error: {:?}\n", data.unwrap()));
    }
    let proto = parse_protobuf_message(&data.unwrap());
    if proto.is_err() {
        println!("Error parsing protobuf");
        println!("{:#?}", proto);
        return HttpResponse::InternalServerError().body(format!("{:#?}", proto));
    }
    let proto = match qs.get("route") {
        Some(route) => {
            let mut filtered_message = FeedMessage::default();
            filtered_message.header = FeedHeader {
                gtfs_realtime_version: "2.0".to_string(),
                incrementality: Some(0),
                timestamp: proto.unwrap().header.timestamp.clone(),
            };

            for entity in proto.unwrap().entity {
                let mut filtered_entity = FeedEntity::default();
                filtered_entity.id = entity.id.clone();
                if entity.trip_update.is_some() && entity.trip_update.as_ref().unwrap().trip.route_id() == route {
                    filtered_entity.trip_update = entity.trip_update.clone();
                }
                if entity.vehicle.is_some() && entity.vehicle.as_ref().unwrap().trip.is_some() && entity.vehicle.as_ref().unwrap().trip.as_ref().unwrap().route_id() == route {
                    filtered_entity.vehicle = entity.vehicle.clone();
                }
                if entity.alert.is_some() {
                    let mut informed_entities: Vec<EntitySelector> = Vec::new();
                    for informed_entity in &entity.alert.as_ref().unwrap().informed_entity {
                        if informed_entity.route_id() == route {
                            informed_entities.push(informed_entity.clone());
                        }
                    }
                    if filtered_entity.alert.is_none() {
                        filtered_entity.alert = entity.alert.clone();
                    }
                    filtered_entity.alert.as_mut().unwrap().informed_entity = informed_entities;
                }
                println!("{:?}", filtered_entity);
                if filtered_entity.trip_update.is_some() || filtered_entity.vehicle.is_some() || filtered_entity.alert.is_some() || filtered_entity.shape.is_some() {
                    filtered_message.entity.push(filtered_entity);
                }
            }
            println!("{:?}", filtered_message);
            filtered_message
        }
        None => proto.unwrap(),
    };
    
    if usejson {
        let protojson = serde_json::to_string(&proto).unwrap();
        HttpResponse::Ok()
            .insert_header(("Content-Type", "application/json"))
            .body(protojson)
    } else {
        let protojson = format!("{:#?}", proto);
        HttpResponse::Ok().body(protojson)
    }
}

async fn gtfsrtws(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let qs = QString::from(req.query_string());
    let feed = match qs.get("feed") {
        Some(feed) => feed.to_string(),
        None => {
            return Ok(HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No feed specified\n"))
        }
    };
    let category = match qs.get("category") {
        Some(category) => category.to_string(),
        None => {
            return Ok(HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .body("Error: No category specified\n"))
        }
    };
    let suicidebutton = match qs.get("suicidebutton") {
        Some(_) => true,
        None => false,
    };
    let resp = ws::start(
        GtfsWs {
            feed,
            category,
            suicidebutton,
        },
        &req,
        stream,
    );
    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let builder = HttpServer::new(|| {
        App::new()
            .wrap(
                DefaultHeaders::new()
                    .add(("Server", "Kactus"))
                    .add(("Access-Control-Allow-Origin", "*"))
                    .add((
                        "Access-Control-Expose-Headers",
                        "Server, hash, server, Hash",
                    )),
            )
            .wrap(actix_block_ai_crawling::BlockAi)
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index))
            .route("/gtfsrt/", web::get().to(gtfsrt))
            .route("/gtfsrt", web::get().to(gtfsrt))
            .route("/gtfsrtasjson/", web::get().to(gtfsrttojson))
            .route("/gtfsrtasjson", web::get().to(gtfsrttojson))
            .route("/gtfsrttimes", web::get().to(gtfsrttimes))
            .route("/gtfsrttimes/", web::get().to(gtfsrttimes))
            .route("/gtfsrtws/", web::get().to(gtfsrtws))
            .route("/gtfsrtws", web::get().to(gtfsrtws))
    })
    .workers(4);

    let _ = builder.bind("127.0.0.1:54105").unwrap().run().await;

    Ok(())
}
