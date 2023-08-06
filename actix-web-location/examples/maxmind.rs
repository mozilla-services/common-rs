//! Run server with:
//!
//! ```console
//! $ cargo run --example=maxmind --features=actix-web-v4,maxmind
//! ```
//!
//! Test with:
//!
//! ```console
//! curl http://localhost:8080/ -H 'x-forwarded-for: 216.160.83.56'
//! ```

extern crate actix_web_4 as actix_web;

use std::path::Path;

use actix_web::{get, web::Data, App, HttpRequest, HttpServer, Responder};
use actix_web_location::providers::{MaxMindProvider, Provider as _};

const MMDB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/GeoLite2-City-Test.mmdb");

#[get("/")]
async fn index(req: HttpRequest, location_provider: Data<MaxMindProvider>) -> impl Responder {
    format!("{:#?}", location_provider.get_location(&req).await)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("loading MaxMind DB from {MMDB_PATH}");
    let location_provider =
        MaxMindProvider::from_path(&Path::new(MMDB_PATH)).expect("could not make maxmind client");
    let location_provider = Data::new(location_provider);

    println!("starting HTTP server at http://localhost:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(location_provider.clone())
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
