use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use ferivonus_swagger_gen::{ApiRegistry, ferivonus_config, register_api};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct MathQuery {
    a: i32,
    b: i32,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: u32,
}

#[derive(Serialize)]
struct User {
    id: u32,
    username: String,
    role: String,
}

#[register_api(summary = "System welcome message")]
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "message": "Ferivonus API Engine is running" }))
}

#[register_api(summary = "System health and uptime check")]
#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "Operational", "uptime": "99.9%" }))
}

#[register_api(
    summary = "Mathematical addition operation",
    params = "a:integer, b:integer"
)]
#[get("/math/add")]
async fn add(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a + query.b }))
}

#[register_api(
    summary = "Mathematical multiplication operation",
    params = "a:integer, b:integer"
)]
#[get("/math/multiply")]
async fn multiply(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a * query.b }))
}

#[register_api(
    summary = "Search across the database",
    params = "q:string, limit:integer"
)]
#[get("/search")]
async fn search(query: web::Query<SearchQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "query": query.q,
        "returned_items": query.limit,
        "data": []
    }))
}

#[register_api(summary = "Retrieve administrator profile")]
#[get("/users/admin")]
async fn get_admin_profile() -> impl Responder {
    let user = User {
        id: 777,
        username: "ferivonus_root".to_string(),
        role: "superuser".to_string(),
    };
    HttpResponse::Ok().json(user)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8080";
    let registry = ApiRegistry::new();

    registry.print_startup_info(addr);

    let registry_data = web::Data::new(registry);

    HttpServer::new(move || {
        App::new()
            .app_data(registry_data.clone())
            .configure(ferivonus_config)
            .service(index)
            .service(status)
            .service(add)
            .service(multiply)
            .service(search)
            .service(get_admin_profile)
    })
    .bind(addr)?
    .run()
    .await
}
