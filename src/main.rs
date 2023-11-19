use actix::{Actor, StreamHandler, AsyncContext, ActorContext};
use actix_web::{
    middleware::DefaultHeaders, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Error
};
use actix_web_actors::ws;
use rand::Rng;
use redis::Commands;
extern crate qstring;

use kactus::parse_protobuf_message;
use qstring::QString;
use std::time::Instant;
use serde::Serialize;

pub struct GtfsWs {
    feed: String,
    category: String,
    hashofbodyclient: Option<String>,
    suicidebutton: bool,
    skipfailure: Option<String>,
    timeofclientcache: Option<String>,
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
        let timeofcache = doesexist.unwrap();
        let data = con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", &feed, &category));
        if data.is_err() {
            println!("Error: {:?}", data);
            ctx.text(format!("Error: {:?}\n", data));
            return ctx.close(None);
        }
        let data = data.unwrap();
        if self.suicidebutton { return ctx.binary(data)}
        let proto = parse_protobuf_message(&data);
        let hashofresult = match proto {
                Ok(_) => fasthash::metro::hash64(
                data.as_slice(),
            ),
                Err(_) => {let mut rng = rand::thread_rng();
                    rng.gen::<u64>()
            }
        };
        let timeofclientcache = &self.timeofclientcache;
        if timeofclientcache.is_some() {
            let timeofclientcache = timeofclientcache.clone().unwrap();
            let timeofclientcache = (*timeofclientcache).parse::<u64>();
            if timeofclientcache.is_ok() {
                let timeofclientcache = timeofclientcache.unwrap();
                if timeofclientcache >= timeofcache {return ctx.text("")}
                match &proto {
                    Ok(proto) => {
                        let headertimestamp = proto.header.timestamp;
                        if headertimestamp.is_some() {
                            if timeofclientcache
                                >= headertimestamp.unwrap()
                            {
                                return ctx.text("")
                            }
                        }
                    }
                    Err(bruh) => {
                        println!("{:#?}", bruh);
                        let skipfailure = &self.skipfailure;
                        let mut allowcrash = true;
                        if skipfailure.is_some() {
                            if skipfailure.clone().unwrap() == "true" {
                                allowcrash = false;
                            }
                        }
                        if allowcrash {
                            ctx.text("protobuf failed to parse");
                            return ctx.close(None);
                        }
                    }
                }
            }
            let hashofbodyclient = &self.hashofbodyclient;
            if hashofbodyclient.is_some() {
                let hashofbodyclient = hashofbodyclient.clone().unwrap();
                if (&proto).is_ok() {
                    let clienthash = hashofbodyclient.parse::<u64>();
                    if clienthash.is_ok() {
                        if clienthash.unwrap() == hashofresult {
                            return ctx.text("")
                        }
                    }
                }
            }
        }
        ctx.binary(data)
    }
}
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GtfsWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
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
                                    let suicidebutton = qs.get("suicidebutton");

                                    if suicidebutton.is_some() {
                                        let suicidebutton = suicidebutton.unwrap();

                                        if suicidebutton == "true" {
                                            return HttpResponse::Ok()
                                                .insert_header((
                                                    "Content-Type",
                                                    "application/x-google-protobuf",
                                                ))
                                                .body(data);
                                        }
                                    }

                                    let timeofclientcache = qs.get("timeofcache");

                                    let proto = parse_protobuf_message(&data);

                                    let hashofresult = match proto {
                                        Ok(_) => fasthash::metro::hash64(
                                        data.as_slice(),
                                    ),
                                        Err(_) => {let mut rng = rand::thread_rng();

                                            rng.gen::<u64>()
                                    }
                                };

                                    if timeofclientcache.is_some() {
                                        let timeofclientcache = timeofclientcache.unwrap();

                                        let timeofclientcache = (*timeofclientcache).parse::<u64>();

                                        if timeofclientcache.is_ok() {
                                            let timeofclientcache = timeofclientcache.unwrap();

                                            if timeofclientcache >= timeofcache {
                                                return HttpResponse::NoContent().body("");
                                            }
                                            match &proto {
                                                Ok(proto) => {
                                                    let headertimestamp = proto.header.timestamp;

                                                    if headertimestamp.is_some() {
                                                        if timeofclientcache
                                                            >= headertimestamp.unwrap()
                                                        {
                                                            return HttpResponse::NoContent()
                                                                .body("");
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
                                        .insert_header((
                                            "Content-Type",
                                            "application/x-google-protobuf",
                                        ))
                                        .insert_header(("hash", hashofresult))
                                        .body(data)
                                }
                                Err(e) => {
                                    println!("Error: {:?}", e);
                                    HttpResponse::InternalServerError()
                                        .insert_header(("Content-Type", "text/plain"))
                                        .body(format!("Error: {}\n", e))
                                }
                            }
                        }
                        Err(_e) => {
                            return HttpResponse::InternalServerError()
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

    let query_str = req.query_string(); // "name=ferret"
    let qs = QString::from(query_str);
    let feed = qs.get("feed");

    let raw = qs.get("raw");

    let usejson = match raw {
        Some(raw) => {
            if raw == "true" {
                false
            } else {
                true
            }
        }
        None => true,
    };

    match feed {
        Some(feed) => {
            let category = qs.get("category");

            //HttpResponse::Ok().body(format!("Requested {}/{}", feed, category))

            match category {
                Some(category) => {
                    let doesexist =
                        con.get::<String, u64>(format!("gtfsrttime|{}|{}", feed, category));

                    match doesexist {
                        Ok(_timeofcache) => {
                            let data =
                                con.get::<String, Vec<u8>>(format!("gtfsrt|{}|{}", feed, category));

                            match data {
                                Ok(data) => {
                                    let proto = parse_protobuf_message(&data);

                                    match proto {
                                        Ok(proto) => {
                                            if usejson {
                                                let protojson =
                                                    serde_json::to_string(&proto).unwrap();

                                                HttpResponse::Ok()
                                                    .insert_header((
                                                        "Content-Type",
                                                        "application/json",
                                                    ))
                                                    .body(protojson)
                                            } else {
                                                let protojson = format!("{:#?}", proto);

                                                HttpResponse::Ok().body(protojson)
                                            }
                                        }
                                        Err(proto) => {
                                            println!("Error parsing protobuf");

                                            println!("{:#?}", proto);

                                            HttpResponse::NotFound().body(format!("{:#?}", proto))
                                        }
                                    }
                                }
                                Err(e) => HttpResponse::InternalServerError()
                                    .insert_header(("Content-Type", "text/plain"))
                                    .body(format!("Error: {}\n", e)),
                            }
                        }
                        Err(_e) => {
                            return HttpResponse::InternalServerError()
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

async fn gtfsrtws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, actix_web::Error>  {
    let qs = QString::from(req.query_string());
    let feed = match qs.get("feed") {
        Some(feed) => feed.to_string(),
        None => return Ok(HttpResponse::NotFound().insert_header(("Content-Type", "text/plain")).body("Error: No feed specified\n")),
    };
    let category = match qs.get("category") {
        Some(category) => category.to_string(),
        None => return Ok(HttpResponse::NotFound().insert_header(("Content-Type", "text/plain")).body("Error: No category specified\n")),
    };
    let suicidebutton = match qs.get("suicidebutton") {
        Some(_) => true,
        None => false,
    };
    let timeofclientcache = match qs.get("timeofcache") {
        Some(timeofcache) => Some(timeofcache.to_string()),
        None => None,
    };
    let hashofbodyclient = match qs.get("bodyhash") {
        Some(hashofbodyclient) => Some(hashofbodyclient.to_string()),
        None => None,
    };
    let skipfailure = match qs.get("skipfailure") {
        Some(skipfailure) => Some(skipfailure.to_string()),
        None => None,
    };
    let resp = ws::start(GtfsWs {feed, category, suicidebutton, skipfailure, timeofclientcache, hashofbodyclient}, &req, stream);
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
