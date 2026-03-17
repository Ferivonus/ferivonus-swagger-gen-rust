# Ferivonus Swagger Gen

Automated, zero-configuration Swagger and OpenAPI documentation engine for Actix-web.

Say goodbye to manually writing OpenAPI schemas! `ferivonus-swagger-gen` automatically generates your `swagger.json` and serves a beautiful Swagger UI directly from your Actix-web routes using simple macros.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
actix-web = "4"
serde = { version = "1.0", features = ["derive"] }
ferivonus-swagger-gen = "0.1.1"
```

## Quick Start (Example)

Here is a complete example of how to use the Ferivonus engine. Just add the `#[register_api]` macro to your Actix routes, and the engine will do the rest!

```rust
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use ferivonus_swagger_gen::{ApiRegistry, ferivonus_config, register_api};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct MathQuery {
    a: i32,
    b: i32,
}

#[derive(Serialize)]
struct User {
    id: u32,
    username: String,
    role: String,
}

// 1. Simply attach the macro to your route
#[register_api(summary = "System health and uptime check")]
#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "Operational" }))
}

// 2. It automatically detects your parameters!
#[register_api(summary = "Mathematical addition", params = "a:integer, b:integer")]
#[get("/math/add")]
async fn add(query: web::Query<MathQuery>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "result": query.a + query.b }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:8080";

    // 3. Initialize the registry
    let registry = ApiRegistry::new();
    registry.print_startup_info(addr);

    let registry_data = web::Data::new(registry);

    HttpServer::new(move || {
        App::new()
            .app_data(registry_data.clone())
            // 4. Mount the auto-generated Swagger UI and JSON spec
            .configure(ferivonus_config)
            .service(status)
            .service(add)
    })
    .bind(addr)?
    .run()
    .await
}
```

## Accessing the UI

Once your server is running, simply navigate to:

- **Swagger UI:** `http://127.0.0.1:8080/fer-ui/`
- **OpenAPI JSON Spec:** `http://127.0.0.1:8080/ferivonus.json`

## Configuration

The UI is enabled by default. If you want to disable it in production, you can easily do so using the builder pattern:

```rust
let registry = ApiRegistry::new().deactivate_ui();
```
