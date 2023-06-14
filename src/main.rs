use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
extern crate qstring;
use qstring::QString;

async fn index(req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn gtfsrt(req: HttpRequest) -> impl Responder {
    let redisclient = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let mut con = redisclient.get_connection().unwrap();

    let query_str = req.query_string(); // "name=ferret"
    let qs = QString::from(query_str);
    let feed = qs.get("feed").unwrap();

    let category = qs.get("category").unwrap();

    //HttpResponse::Ok().body(format!("Requested {}/{}", feed, category))

    let data = con.get(format!("gtfsrt|{}|{}", feed, category)).await;

    match data {
        Ok(data) => HttpResponse::Ok().body(data),
        Err(e) => HttpResponse::NotFound().body(format!("Error: {}", e)),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a new HTTP server.
    let builder = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/gtfsrt/", web::get().to(gtfsrt))
    })
    .workers(4);

    // Bind the server to port 8080.
    let _ = builder.bind("127.0.0.1:8080").unwrap().run().await;

    Ok(())
}
