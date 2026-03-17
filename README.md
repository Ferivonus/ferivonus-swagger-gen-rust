# Ferivonus Swagger Gen

Automated, zero-configuration Swagger and OpenAPI documentation engine for Actix-web.

Ferivonus Swagger Gen is a high-performance documentation engine that eliminates the need for manual OpenAPI (Swagger) specification writing. By leveraging Rust's procedural macros and the inventory-based collection mechanism, it extracts metadata directly from your Actix-web source code at compile time and serves it through a modern, interactive UI.

## Core Features

- Zero Configuration: No YAML or JSON files required. Your code is the source of truth.
- Type Safety: Deep integration with Rust's type system, including full support for Enums and nested Structs.
- Smart Parameter Detection: Automatically distinguishes between Path and Query parameters based on your route definitions.
- Standardized Error Handling: Built-in ApiError and ErrorResponse system to ensure consistent error reporting across your entire API.
- Premium UI Experience: Features a tabbed response interface, Inter-font styling, and automatic browser opening for a better developer experience (DX).

## Installation

Add the following to your Cargo.toml file:

```toml
[dependencies]
actix-web = "4"
serde = { version = "1.0", features = ["derive"] }
ferivonus-swagger-gen = "0.2.2"
```

## Technical Architecture

The engine works by using procedural macros to generate "plugins" at compile time. These plugins are collected into a global registry using the `inventory` crate. When the `ApiRegistry` is initialized at runtime, it iterates over these collected plugins to build a complete OpenAPI 3.0.0 specification.

### Supported Type Mappings

The `#[derive(ApiSchema)]` macro automatically translates Rust types to OpenAPI formats:

- i8, i16, i32, i64, u32... -> integer
- f32, f64 -> number
- bool -> boolean
- String, &str -> string
- Uuid -> string (format: uuid)
- DateTime -> string (format: date-time)
- Vec<T> -> array
- Option<T> -> nullable: true
- Enums -> string (enum: ["variant1", "variant2"])

## Quick Start

The following example demonstrates a complete setup with Scopes, Security, and Error Handling.

```rust
use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use ferivonus_swagger_gen::{ferivonus_config, register_api, ApiRegistry, ApiSchema, ApiError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, ApiSchema)]
enum UserStatus {
    Active,
    Inactive,
    Banned,
}

#[derive(Deserialize, ApiSchema)]
struct CreateUserRequest {
    username: String,
    status: UserStatus,
}

#[derive(Serialize, ApiSchema)]
struct UserResponse {
    id: u32,
    username: String,
    status: UserStatus,
}

#[register_api(
    summary = "Get user details by ID",
    tags = "User Management",
    params = "id:integer",
    response_model = "UserResponse"
)]
#[get("/users/{id}")]
async fn get_user(path: web::Path<u32>) -> Result<HttpResponse, ApiError> {
    let user_id = path.into_inner();
    if user_id == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }
    Ok(HttpResponse::Ok().json(UserResponse {
        id: user_id,
        username: "ferivonus".into(),
        status: UserStatus::Active,
    }))
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
            .service(get_user)
    })
    .bind(addr)?
    .run()
    .await
}
```

## Detailed Macro Usage

### #[register_api(...)]

Attribute macro for Actix-web handlers.

- summary: (Required) A concise description of what the endpoint does.
- tags: Used for grouping endpoints in the UI. Multiple tags are comma-separated.
- params: Used to document parameters. If a parameter name matches a placeholder in the URL (e.g., {id}), it is marked as a Path parameter; otherwise, it is treated as a Query parameter.
- request_body: The name of the struct (derived with ApiSchema) representing the JSON input.
- response_model: The name of the struct representing a successful response.
- overload_responses: Allows overriding default responses (e.g., "201:MyModel, 400:ErrorResponse").
- security: Use "Bearer" to require a JWT token for the endpoint.

### #[derive(ApiSchema)]

Derive macro for structs and enums.

- Fields: Supports named and unnamed (tuple) fields.
- Collections: Deeply parses Vec and Option wrappers.
- Enums: Extracts variant names and presents them as a dropdown selection in the UI.

## Global Error Management

The library provides a built-in `ApiError` enum that implements `actix_web::ResponseError`. When an `ApiError` is returned from a handler:

1. It automatically sets the correct HTTP status code (400, 401, 404, or 500).
2. It returns a standardized JSON object using the `ErrorResponse` schema.

This ensures that frontend developers always receive a consistent error format regardless of the endpoint.

## UI Endpoints

By default, the following routes are mounted:

- Swagger UI: http://127.0.0.1:8080/fer-ui/ (Interactive documentation)
- OpenAPI JSON: http://127.0.0.1:8080/ferivonus.json (Raw specification)

## Disabling UI in Production

To disable documentation and automatic browser opening in production environments, use the `deactivate_ui` method:

```rust
let registry = ApiRegistry::new().deactivate_ui();
```

---

Developed by Ferivonus
