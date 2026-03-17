use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use header_html::{ApiRegistry, ferivonus_config, register_api};
use serde::Deserialize;

#[derive(Deserialize)]
struct MathQuery {
    a: i32,
    b: i32,
}

#[register_api(summary = "System welcome message")]
#[get("/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "message": "Welcome to the Ferivonus engine" }))
}

#[register_api(summary = "System health check")]
#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "Operational" }))
}

#[register_api(summary = "Addition", params = "a:integer, b:integer")]
#[get("/add")]
async fn add(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a + query.b }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8080";

    // Kutuphane varsayilan olarak arayuzu acik sekilde baslatir.
    // Eger arayuzu kapatmak istersen sunu kullanabilirsin:
    // let registry = ApiRegistry::new().deactivate_ui();
    let registry = ApiRegistry::new();

    // Loglari yazdirmasi icin kutuphaneyi gorevlendiriyoruz
    registry.print_startup_info(addr);

    let registry_data = web::Data::new(registry);

    HttpServer::new(move || {
        App::new()
            .app_data(registry_data.clone())
            .configure(ferivonus_config)
            .service(hello)
            .service(status)
            .service(add)
    })
    .bind(addr)?
    .run()
    .await
}
