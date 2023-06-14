use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use redis::Commands;
extern crate qstring;
use qstring::QString;

async fn index(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Server", "Kactus"))
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
                        con.exists::<String, bool>(format!("gtfsrtvalid|{}|{}", feed, category));

                    match doesexist {
                        Ok(data) => {
                            if data == false {
                                return HttpResponse::NotFound()
                                    .insert_header(("Content-Type", "text/plain"))
                                    .insert_header(("Server", "Kactus"))
                                    .body("Error: Data Not Found\n");
                            } else {
                                let data = con.get::<String, Vec<u8>>(format!(
                                    "gtfsrt|{}|{}",
                                    feed, category
                                ));

                                match data {
                                    Ok(data) => HttpResponse::Ok()
                                        .insert_header((
                                            "Content-Type",
                                            "application/x-google-protobuf",
                                        ))
                                        .insert_header(("Server", "Kactus"))
                                        .body(data),
                                    Err(e) => HttpResponse::NotFound()
                                        .insert_header(("Content-Type", "text/plain"))
                                        .insert_header(("Server", "Kactus"))
                                        .body(format!("Error: {}\n", e)),
                                }
                            }
                        }
                        Err(e) => {
                            return HttpResponse::NotFound()
                                .insert_header(("Content-Type", "text/plain"))
                                .insert_header(("Server", "Kactus"))
                                .body(format!("Error in connecting to redis\n"))
                        }
                    }
                }
                None => {
                    return HttpResponse::NotFound()
                        .insert_header(("Content-Type", "text/plain"))
                        .insert_header(("Server", "Kactus"))
                        .body("Error: No category specified\n")
                }
            }
        }
        None => {
            return HttpResponse::NotFound()
                .insert_header(("Content-Type", "text/plain"))
                .insert_header(("Server", "Kactus"))
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
    })
    .workers(4);

    // Bind the server to port 8080.
    let _ = builder.bind("127.0.0.1:8080").unwrap().run().await;

    Ok(())
}
