use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer, Responder, delete, get, post, put, web,
};
use ferivonus_swagger_gen::{ApiRegistry, ApiSchema, ferivonus_config, register_api};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, ApiSchema)]
enum Role {
    Admin,
    User,
    Guest,
}

#[derive(Deserialize, ApiSchema)]
struct UserCreateRequest {
    username: String,
    age: i32,
    role: Role,
}

#[derive(Deserialize, ApiSchema)]
struct UserUpdateRequest {
    role: Role,
}

#[derive(Serialize, ApiSchema)]
struct UserResponse {
    id: u32,
    username: String,
    role: Role,
}

#[derive(Serialize, ApiSchema)]
struct ErrorResponse {
    error_code: String,
    message: String,
}

#[register_api(
    summary = "Sistemin ayakta olup olmadığını kontrol eder",
    tags = "Sistem",
    overload_responses = "200:string"
)]
#[get("/health")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Ferivonus Engine is up and running! 🚀")
}

#[register_api(
    summary = "Sistem yöneticisinin profilini getirir",
    tags = "Admin İşlemleri",
    security = "Bearer",
    response_model = "UserResponse"
)]
#[get("/users/admin")]
async fn get_admin_profile(req: HttpRequest) -> impl Responder {
    let auth_header = req.headers().get("Authorization");

    match auth_header {
        Some(token) if token.to_str().unwrap_or("") == "Bearer ferivonus_secret_token" => {
            HttpResponse::Ok().json(UserResponse {
                id: 777,
                username: "ferivonus_root".to_string(),
                role: Role::Admin,
            })
        }
        _ => HttpResponse::Unauthorized().json(ErrorResponse {
            error_code: "UNAUTHORIZED".to_string(),
            message: "Geçerli bir Bearer Token gerekli.".to_string(),
        }),
    }
}

#[register_api(
    summary = "Kullanıcıyı sistemden siler (Sadece Admin)",
    tags = "Admin İşlemleri",
    security = "Bearer",
    params = "id:integer",
    overload_responses = "204:string, 401:ErrorResponse, 404:ErrorResponse"
)]
#[delete("/users/{id}")]
async fn delete_user(path: web::Path<u32>, req: HttpRequest) -> impl Responder {
    let auth_header = req.headers().get("Authorization");
    if auth_header.is_none()
        || auth_header.unwrap().to_str().unwrap_or("") != "Bearer ferivonus_secret_token"
    {
        return HttpResponse::Unauthorized().json(ErrorResponse {
            error_code: "UNAUTHORIZED".to_string(),
            message: "Silme işlemi için yetkiniz yok.".to_string(),
        });
    }

    let user_id = path.into_inner();
    println!("🗑️ Kullanıcı silindi. ID: {}", user_id);

    HttpResponse::NoContent().finish()
}

#[derive(Deserialize)]
struct PaginationQuery {
    limit: Option<u32>,
}

#[register_api(
    summary = "Tüm kullanıcıları listeler (Sayfalamalı)",
    tags = "Kullanıcı İşlemleri",
    params = "limit:integer",
    overload_responses = "200:string"
)]
#[get("/users")]
async fn list_users(query: web::Query<PaginationQuery>) -> impl Responder {
    let limit = query.limit.unwrap_or(10);
    println!("📋 {} adet kullanıcı listeleniyor...", limit);

    HttpResponse::Ok().body(format!("{} adet kullanıcı başarıyla getirildi.", limit))
}

#[register_api(
    summary = "ID'ye göre kullanıcı detayını getirir",
    tags = "Kullanıcı İşlemleri",
    params = "id:integer",
    response_model = "UserResponse"
)]
#[get("/users/{id}")]
async fn get_user(path: web::Path<u32>) -> impl Responder {
    let user_id = path.into_inner();

    if user_id == 0 {
        return HttpResponse::NotFound().json(ErrorResponse {
            error_code: "NOT_FOUND".to_string(),
            message: "Kullanıcı bulunamadı.".to_string(),
        });
    }

    HttpResponse::Ok().json(UserResponse {
        id: user_id,
        username: format!("user_{}", user_id),
        role: Role::User,
    })
}

#[register_api(
    summary = "Yeni kullanıcı oluşturur",
    tags = "Kullanıcı İşlemleri",
    request_body = "UserCreateRequest",
    overload_responses = "201:UserResponse, 400:ErrorResponse"
)]
#[post("/users")]
async fn create_user(body: web::Json<UserCreateRequest>) -> impl Responder {
    if body.age < 18 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error_code: "AGE_RESTRICTION".to_string(),
            message: "Kayıt olmak için 18 yaşından büyük olmalısınız.".to_string(),
        });
    }

    HttpResponse::Created().json(UserResponse {
        id: 101,
        username: body.username.clone(),
        role: body.role.clone(),
    })
}

#[register_api(
    summary = "Kullanıcının rolünü günceller",
    tags = "Kullanıcı İşlemleri",
    params = "id:integer",
    request_body = "UserUpdateRequest",
    response_model = "UserResponse"
)]
#[put("/users/{id}/role")]
async fn update_user_role(
    path: web::Path<u32>,
    body: web::Json<UserUpdateRequest>,
) -> impl Responder {
    let user_id = path.into_inner();

    println!("🔄 Kullanıcı ({}) rolü güncellendi.", user_id);

    HttpResponse::Ok().json(UserResponse {
        id: user_id,
        username: format!("user_{}", user_id),
        role: body.role.clone(),
    })
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
            .service(health_check)
            .service(get_admin_profile)
            .service(delete_user)
            .service(list_users)
            .service(get_user)
            .service(create_user)
            .service(update_user_role)
    })
    .bind(addr)?
    .run()
    .await
}
