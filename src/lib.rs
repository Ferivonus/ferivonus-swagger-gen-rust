use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

pub use header_html_macros::register_api;

pub const UI_PATH: &str = "/fer-ui/";
pub const SPEC_PATH: &str = "/ferivonus.json";

pub struct RoutePlugin {
    pub path: &'static str,
    pub method: &'static str,
    pub summary: &'static str,
    pub response_type: &'static str,
    pub params: &'static [(&'static str, &'static str)],
}

inventory::collect!(RoutePlugin);

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiParam {
    pub name: String,
    pub p_type: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ApiMetadata {
    pub path: String,
    pub method: String,
    pub summary: String,
    pub parameters: Vec<ApiParam>,
    pub response_type: String,
}

pub struct ApiRegistry {
    pub routes: Mutex<Vec<ApiMetadata>>,
    pub enable_ui: bool,
}

impl ApiRegistry {
    // Varsayilan olarak UI aktiftir
    pub fn new() -> Self {
        let registry = Self {
            routes: Mutex::new(Vec::new()),
            enable_ui: true,
        };
        registry.load_automatic_routes();
        registry
    }

    // Arayuzu deaktif etmek icin zincirlenebilir (builder) metod
    pub fn deactivate_ui(mut self) -> Self {
        self.enable_ui = false;
        self
    }

    // Sunucu baslarken loglari otomatik yazdiran yardimci metod
    pub fn print_startup_info(&self, addr: &str) {
        println!("Ferivonus Server is starting...");
        if self.enable_ui {
            println!("Interface:     http://{}{}", addr, UI_PATH);
            println!("Specification: http://{}{}", addr, SPEC_PATH);
        } else {
            println!("Warning: API Documentation interface is currently DISABLED.");
        }
    }

    fn load_automatic_routes(&self) {
        let mut routes = self.routes.lock().unwrap();
        for plugin in inventory::iter::<RoutePlugin> {
            let parameters = plugin
                .params
                .iter()
                .map(|(name, p_type)| ApiParam {
                    name: name.to_string(),
                    p_type: p_type.to_string(),
                })
                .collect();

            routes.push(ApiMetadata {
                path: plugin.path.to_string(),
                method: plugin.method.to_string(),
                summary: plugin.summary.to_string(),
                parameters,
                response_type: plugin.response_type.to_string(),
            });
        }
    }
}

pub fn ferivonus_config(cfg: &mut web::ServiceConfig) {
    cfg.route(SPEC_PATH, web::get().to(ferivonus_spec))
        .route(UI_PATH, web::get().to(fer_ui));
}

pub async fn ferivonus_spec(reg: web::Data<ApiRegistry>) -> impl Responder {
    if !reg.enable_ui {
        return HttpResponse::NotFound().body("API Documentation is disabled.");
    }

    let routes = reg.routes.lock().unwrap();
    let mut paths = serde_json::Map::new();

    for r in routes.iter() {
        let params_json: Vec<serde_json::Value> = r
            .parameters
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name, "in": "query", "required": true, "schema": { "type": p.p_type }
                })
            })
            .collect();

        let schema_type = if r.response_type == "text/plain" {
            "string"
        } else {
            "object"
        };

        let op = serde_json::json!({
            "summary": r.summary,
            "parameters": params_json,
            "responses": {
                "200": {
                    "description": "Success",
                    "content": {
                        r.response_type.as_str(): { "schema": { "type": schema_type } }
                    }
                }
            }
        });

        let mut methods = serde_json::Map::new();
        methods.insert(r.method.to_lowercase(), op);
        paths.insert(r.path.clone(), serde_json::Value::Object(methods));
    }

    let doc = serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "Ferivonus Automated API", "version": "1.0.0" },
        "paths": paths
    });
    HttpResponse::Ok().json(doc)
}

pub async fn fer_ui(reg: web::Data<ApiRegistry>) -> impl Responder {
    if !reg.enable_ui {
        return HttpResponse::NotFound().body("Fer-UI Interface is currently disabled.");
    }

    let html = r#"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
        <title>Fer-UI</title>
        <style>
            .swagger-ui .topbar { display: none; }
            body { margin: 0; background-color: #fafafa; }
        </style>
    </head>
    <body>
        <div id="fer-ui-root"></div>
        <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
        <script>
            window.onload = () => {
                window.ui = SwaggerUIBundle({ 
                    url: "/ferivonus.json", 
                    dom_id: '#fer-ui-root',
                    defaultModelsExpandDepth: -1,
                    docExpansion: "list"
                });
            };
        </script>
    </body>
    </html>"#;

    HttpResponse::Ok().content_type("text/html").body(html)
}
