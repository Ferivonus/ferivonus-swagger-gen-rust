//! This example demonstrates a complete Actix-web server integrated with the Ferivonus engine.
//! It covers basic routes, query parameters, and structured JSON responses.

use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use ferivonus_swagger_gen::{ApiRegistry, ferivonus_config, register_api};
use serde::{Deserialize, Serialize};

/// Request schema for mathematical operations.
#[derive(Deserialize)]
struct MathQuery {
    a: i32,
    b: i32,
}

/// Request schema for database search queries.
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: u32,
}

/// Response schema for user data.
#[derive(Serialize)]
struct User {
    id: u32,
    username: String,
    role: String,
}

#[register_api(summary = "Root index providing basic engine information")]
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "message": "Ferivonus API Engine is running" }))
}

#[register_api(summary = "Health check endpoint for monitoring system status")]
#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "Operational", "uptime": "99.9%" }))
}

#[register_api(
    summary = "Performs addition on two integers",
    params = "a:integer, b:integer"
)]
#[get("/math/add")]
async fn add(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a + query.b }))
}

#[register_api(
    summary = "Performs multiplication on two integers",
    params = "a:integer, b:integer"
)]
#[get("/math/multiply")]
async fn multiply(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a * query.b }))
}

#[register_api(
    summary = "Simulates a search operation with query limits",
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

#[register_api(summary = "Returns the profile data for the system administrator")]
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

    // Initialize the Ferivonus registry to collect all registered routes
    let registry = ApiRegistry::new();

    // Print interface URLs to the console on startup
    registry.print_startup_info(addr);

    let registry_data = web::Data::new(registry);

    HttpServer::new(move || {
        App::new()
            .app_data(registry_data.clone())
            // Configure Ferivonus internal routes (/fer-ui and /ferivonus.json)
            .configure(ferivonus_config)
            // Register application services
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
