//! # Ferivonus Macros
//!
//! This crate provides the procedural macros for the `ferivonus-swagger-gen` documentation engine.
//! It handles the compile-time parsing of Actix-web route handlers and automatically generates
//! the `inventory::submit!` blocks required to register API metadata globally.
//!
//! This crate is generally not intended to be explicitly imported by end-users. Its macros
//! are re-exported by the main `ferivonus-swagger-gen` crate.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ItemFn, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Internal structure to hold the parsed arguments from the `#[register_api(...)]` attribute.
struct RegisterArgs {
    summary: String,
    response_type: String,
    params: Vec<(String, String)>,
}

impl Parse for RegisterArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut summary = String::new();
        let mut response_type = "application/json".to_string();
        let mut params = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let val: LitStr = input.parse()?;

            match key.to_string().as_str() {
                "summary" => summary = val.value(),
                "response_type" => response_type = val.value(),
                "params" => {
                    for p in val.value().split(',') {
                        if let Some((name, p_type)) = p.split_once(':') {
                            params.push((name.trim().to_string(), p_type.trim().to_string()));
                        }
                    }
                }
                _ => {}
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(RegisterArgs {
            summary,
            response_type,
            params,
        })
    }
}

/// Registers an Actix-web route handler into the Ferivonus OpenAPI documentation engine.
///
/// This procedural macro inspects the attached function and its Actix routing attributes
/// (such as `#[get(...)]` or `#[post(...)]`) to extract the HTTP method and endpoint path.
/// It then generates an `inventory::submit!` call to register this metadata into the
/// global `ApiRegistry` at compile time.
///
/// # Arguments
///
/// * `summary` - A brief description of the endpoint's functionality. This acts as the title in the Swagger UI.
/// * `params` - A comma-separated list of query parameters in `name:type` format (e.g., `"id:integer, q:string"`).
/// * `response_type` - The expected MIME type of the response. Defaults to `"application/json"`.
///
/// # Example
/// ```ignore
/// #[register_api(summary = "Retrieve user details", params = "user_id:integer")]
/// #[get("/users/detail")]
/// async fn get_user_detail(query: web::Query<UserQuery>) -> impl Responder {
///     // Implementation here
/// }
/// ```
#[proc_macro_attribute]
pub fn register_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as RegisterArgs);
    let input = parse_macro_input!(item as ItemFn);

    // Default fallbacks for path and method extraction
    let mut extracted_path = format!("/{}", input.sig.ident);
    let mut extracted_method = "GET".to_string();

    // Iterate through the function attributes to find Actix-web routing macros
    for attr in &input.attrs {
        if let syn::Meta::List(meta) = &attr.meta {
            if let Some(ident) = meta.path.get_ident() {
                let method_name = ident.to_string().to_uppercase();
                if ["GET", "POST", "PUT", "DELETE", "PATCH"].contains(&method_name.as_str()) {
                    extracted_method = method_name;
                    if let Ok(lit) = meta.parse_args::<syn::LitStr>() {
                        extracted_path = lit.value();
                    }
                }
            }
        }
    }

    let summary = args.summary;
    let r_type = args.response_type;
    let param_names = args.params.iter().map(|(n, _)| n);
    let param_types = args.params.iter().map(|(_, t)| t);

    let expanded = quote! {
        #input

        inventory::submit! {
            ferivonus_swagger_gen::RoutePlugin {
                path: #extracted_path,
                method: #extracted_method,
                summary: #summary,
                response_type: #r_type,
                params: &[ #( (#param_names, #param_types) ),* ],
            }
        }
    };

    TokenStream::from(expanded)
}
