use actix_web::{HttpResponse, Responder, ResponseError, http::StatusCode, web};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Mutex;

pub use ferivonus_macros::{ApiSchema, register_api};

pub const UI_PATH: &str = "/fer-ui/";
pub const SPEC_PATH: &str = "/ferivonus.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct ErrorResponse {
    pub error_code: String,
    pub message: String,
}

inventory::submit! {
    SchemaPlugin {
        name: "ErrorResponse",
        schema_type: "object",
        fields: &[
            ("error_code", "string"),
            ("message", "string")
        ],
    }
}

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized(msg) => write!(f, "Yetkisiz: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Kötü İstek: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Bulunamadı: {}", msg),
            ApiError::Internal(msg) => write!(f, "Sunucu Hatası: {}", msg),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let (code, message) = match self {
            ApiError::Unauthorized(msg) => ("UNAUTHORIZED", msg),
            ApiError::BadRequest(msg) => ("BAD_REQUEST", msg),
            ApiError::NotFound(msg) => ("NOT_FOUND", msg),
            ApiError::Internal(msg) => ("INTERNAL_ERROR", msg),
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse {
            error_code: code.to_string(),
            message: message.to_string(),
        })
    }
}

#[doc(hidden)]
pub struct RoutePlugin {
    pub path: &'static str,
    pub method: &'static str,
    pub summary: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub request_body: Option<&'static str>,
    pub responses: &'static [(&'static str, &'static str)],
    pub security: Option<&'static str>,
    pub tags: &'static [&'static str],
}

#[doc(hidden)]
pub struct SchemaPlugin {
    pub name: &'static str,
    pub schema_type: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}

inventory::collect!(RoutePlugin);
inventory::collect!(SchemaPlugin);

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
    pub request_body: Option<String>,
    pub responses: Vec<(String, String)>,
    pub security: Option<String>,
    pub tags: Vec<String>,
}

pub struct ApiRegistry {
    pub routes: Mutex<Vec<ApiMetadata>>,
    pub enable_ui: bool,
}

impl ApiRegistry {
    pub fn new() -> Self {
        let registry = Self {
            routes: Mutex::new(Vec::new()),
            enable_ui: true,
        };
        registry.load_automatic_routes();
        registry
    }

    pub fn print_startup_info(&self, addr: &str) {
        println!("🚀 Ferivonus API Engine is Online!");

        if self.enable_ui {
            let ui_url = format!("http://{}{}", addr, UI_PATH);
            let spec_url = format!("http://{}{}", addr, SPEC_PATH);

            println!("📖 UI Interface:  {}", ui_url);
            println!("⚙️  JSON Spec:     {}", spec_url);

            println!("🌐 Opening browser automatically...");
            if let Err(e) = webbrowser::open(&ui_url) {
                println!("⚠️ Could not open browser automatically: {}", e);
            }
        } else {
            println!("🔒 API Documentation is currently disabled.");
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

            let responses = plugin
                .responses
                .iter()
                .map(|(c, m)| (c.to_string(), m.to_string()))
                .collect();

            let tags = plugin.tags.iter().map(|t| t.to_string()).collect();

            routes.push(ApiMetadata {
                path: plugin.path.to_string(),
                method: plugin.method.to_string(),
                summary: plugin.summary.to_string(),
                parameters,
                request_body: plugin.request_body.map(|s| s.to_string()),
                responses,
                security: plugin.security.map(|s| s.to_string()),
                tags,
            });
        }
    }
}

pub fn ferivonus_config(cfg: &mut web::ServiceConfig) {
    cfg.route(SPEC_PATH, web::get().to(ferivonus_spec))
        .route(UI_PATH, web::get().to(fer_ui));
}

fn parse_openapi_type(field_type: &str) -> serde_json::Value {
    if let Some(inner) = field_type.strip_prefix("array:") {
        serde_json::json!({
            "type": "array",
            "items": parse_openapi_type(inner)
        })
    } else if let Some(inner) = field_type.strip_prefix("option:") {
        let mut base = parse_openapi_type(inner);
        if let Some(obj) = base.as_object_mut() {
            obj.insert("nullable".to_string(), serde_json::json!(true));
        }
        base
    } else if let Some(inner) = field_type.strip_prefix("ref:") {
        serde_json::json!({ "$ref": format!("#/components/schemas/{}", inner) })
    } else if let Some(inner) = field_type.strip_prefix("string:") {
        serde_json::json!({ "type": "string", "format": inner })
    } else {
        serde_json::json!({ "type": field_type })
    }
}

pub async fn ferivonus_spec(reg: web::Data<ApiRegistry>) -> impl Responder {
    if !reg.enable_ui {
        return HttpResponse::NotFound().body("API Documentation is disabled.");
    }

    let mut paths = serde_json::Map::new();
    let routes = reg.routes.lock().unwrap();

    for r in routes.iter() {
        let params_json: Vec<serde_json::Value> = r
            .parameters
            .iter()
            .map(|p| {
                let is_path_param = r.path.contains(&format!("{{{}}}", p.name));
                let location = if is_path_param { "path" } else { "query" };
                serde_json::json!({
                    "name": p.name,
                    "in": location,
                    "required": is_path_param,
                    "schema": parse_openapi_type(&p.p_type)
                })
            })
            .collect();

        let mut responses_json = serde_json::Map::new();
        for (code, model) in &r.responses {
            let schema_node = if model.to_lowercase() == "string" {
                serde_json::json!({ "type": "string" })
            } else {
                serde_json::json!({ "$ref": format!("#/components/schemas/{}", model) })
            };

            let description = match code.as_str() {
                "200" => "Success",
                "201" => "Created",
                "204" => "No Content",
                "400" => "Bad Request",
                "401" => "Unauthorized",
                "403" => "Forbidden",
                "404" => "Not Found",
                "500" => "Internal Server Error",
                _ => "Response",
            };

            responses_json.insert(
                code.clone(),
                serde_json::json!({
                    "description": description,
                    "content": {
                        "application/json": { "schema": schema_node }
                    }
                }),
            );
        }

        let mut op = serde_json::json!({
            "summary": r.summary,
            "parameters": params_json,
            "responses": responses_json,
            "tags": r.tags,
        });

        if let Some(sec) = &r.security {
            if sec == "Bearer" {
                if let Some(obj) = op.as_object_mut() {
                    obj.insert(
                        "security".to_string(),
                        serde_json::json!([ { "bearerAuth": [] } ]),
                    );
                }
            }
        }

        if let Some(body) = &r.request_body {
            if let Some(obj) = op.as_object_mut() {
                obj.insert(
                    "requestBody".to_string(),
                    serde_json::json!({
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": format!("#/components/schemas/{}", body) }
                            }
                        }
                    }),
                );
            }
        }

        let method_lower = r.method.to_lowercase();

        if !paths.contains_key(&r.path) {
            paths.insert(r.path.clone(), serde_json::json!({}));
        }

        if let Some(path_obj) = paths.get_mut(&r.path).and_then(|v| v.as_object_mut()) {
            path_obj.insert(method_lower, op);
        }
    }

    let mut schemas = serde_json::Map::new();
    for schema in inventory::iter::<SchemaPlugin> {
        if schema.schema_type == "enum" {
            let variants: Vec<&str> = schema.fields.iter().map(|(v, _)| *v).collect();
            schemas.insert(
                schema.name.to_string(),
                serde_json::json!({
                    "type": "string",
                    "enum": variants
                }),
            );
        } else {
            let mut properties = serde_json::Map::new();
            for (f_name, f_type) in schema.fields {
                properties.insert(f_name.to_string(), parse_openapi_type(f_type));
            }
            schemas.insert(
                schema.name.to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": properties
                }),
            );
        }
    }

    let doc = serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "Ferivonus Engine", "version": "2.0.0" },
        "paths": paths,
        "components": {
            "schemas": schemas,
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        }
    });
    HttpResponse::Ok().json(doc)
}

pub async fn fer_ui(reg: web::Data<ApiRegistry>) -> impl Responder {
    if !reg.enable_ui {
        return HttpResponse::NotFound().body("Fer-UI Interface is currently disabled.");
    }

    let html = r##"
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
        <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
        <title>Ferivonus API Engine</title>
        <style>
            /* 🎨 FERIVONUS ULTIMATE UI */
            body { 
                font-family: 'Inter', sans-serif !important; 
                background-color: #f3f4f6; 
                color: #1f2937; 
                margin: 0; 
                padding: 0; 
            }
            .swagger-ui { font-family: 'Inter', sans-serif !important; padding: 20px; }
            .swagger-ui .topbar { display: none; }
            
            /* Kart Tasarımları */
            .swagger-ui .opblock { 
                border-radius: 12px; 
                box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1); 
                border: 1px solid #e5e7eb; 
                background: #fff; 
                margin-bottom: 20px; 
                transition: all 0.2s ease; 
            }
            .swagger-ui .opblock:hover { 
                transform: translateY(-2px); 
                box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1); 
            }
            .swagger-ui .opblock .opblock-summary { border-radius: 12px; padding: 12px; }
            .swagger-ui .opblock .opblock-summary-method { border-radius: 8px; font-weight: 700; text-transform: uppercase; }
            
            /* Tablo Düzenlemeleri */
            .swagger-ui table.responses-table > thead { display: none !important; }
            .swagger-ui table.responses-table .response-col_status { display: none !important; } 
            
            /* Şemalar / Modeller Kısmı */
            .swagger-ui section.models { 
                border-radius: 12px; 
                border: 1px solid #e5e7eb; 
                box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
                background-color: white;
                margin-top: 40px;
            }
            .swagger-ui section.models h4 { border-bottom: 1px solid #f3f4f6; padding: 15px; margin: 0; }
            .swagger-ui .model-box { background: #f9fafb; border-radius: 8px; }
        </style>
    </head>
    <body>
        <div id="fer-ui-root"></div>
        <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
        <script>
            const TabbedResponsesPlugin = function(system) {
                return {
                    wrapComponents: {
                        responses: (Original, system) => (props) => {
                            const React = system.React;
                            const [activeTab, setActiveTab] = React.useState(null);

                            const responseMap = props.responses;
                            let codes = [];
                            try {
                                if (responseMap && typeof responseMap.keySeq === 'function') {
                                    codes = responseMap.keySeq().toArray();
                                }
                            } catch (e) { codes = ["200"]; }

                            React.useEffect(() => {
                                if (codes.length > 0 && !activeTab) {
                                    const preferred = codes.find(c => c.startsWith('2')) || codes[0];
                                    setActiveTab(preferred);
                                }
                            }, [codes]);

                            const tabs = codes.length > 1 ? React.createElement("div", { 
                                style: { 
                                    display: "flex", gap: "8px", marginBottom: "15px", flexWrap: "wrap", 
                                    padding: "8px", backgroundColor: "#f9fafb", borderRadius: "10px", 
                                    border: "1px solid #e5e7eb" 
                                } 
                            },
                                codes.map(code => {
                                    const isActive = activeTab === code;
                                    let bgColor = "#6366f1"; // Indigo
                                    if (code.startsWith("2")) bgColor = "#10b981"; // Emerald
                                    else if (code.startsWith("4")) bgColor = "#f59e0b"; // Amber
                                    else if (code.startsWith("5")) bgColor = "#ef4444"; // Rose

                                    return React.createElement("button", {
                                        key: code,
                                        onClick: (e) => { e.preventDefault(); setActiveTab(code); },
                                        style: {
                                            padding: "6px 14px",
                                            backgroundColor: isActive ? bgColor : "white",
                                            color: isActive ? "white" : "#4b5563",
                                            border: "1px solid",
                                            borderColor: isActive ? bgColor : "#d1d5db",
                                            borderRadius: "6px",
                                            fontWeight: "600",
                                            fontSize: "13px",
                                            cursor: "pointer",
                                            transition: "all 0.2s ease",
                                            boxShadow: isActive ? `0 4px 6px -1px ${bgColor}40` : "none"
                                        }
                                    }, `Status: ${code}`);
                                })
                            ) : null;

                            const wrapperId = "fer-res-" + Math.random().toString(36).substr(2, 9);
                            
                            const css = `
                                #${wrapperId} .responses-table:not(.live-responses-table) tbody tr.response { display: none !important; }
                                #${wrapperId} .responses-table:not(.live-responses-table) tbody tr.response[data-code="${activeTab}"] { display: table-row !important; }
                            `;

                            return React.createElement("div", { id: wrapperId, className: "fer-responses-wrapper" },
                                React.createElement("style", null, css),
                                tabs,
                                React.createElement("div", { className: "original-responses" },
                                    React.createElement(Original, props)
                                )
                            );
                        }
                    }
                };
            };

            window.onload = () => {
                window.ui = SwaggerUIBundle({ 
                    url: "/ferivonus.json", 
                    dom_id: '#fer-ui-root',
                    deepLinking: true,
                    presets: [ SwaggerUIBundle.presets.apis ],
                    defaultModelsExpandDepth: 1,
                    defaultModelExpandDepth: 1, 
                    docExpansion: "list",
                    plugins: [ TabbedResponsesPlugin ] 
                });
            };
        </script>
    </body>
    </html>"##;

    HttpResponse::Ok().content_type("text/html").body(html)
}
