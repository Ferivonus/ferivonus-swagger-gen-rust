use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ItemFn, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct RegisterArgs {
    summary: String,
    params: Vec<(String, String)>,
    request_body: Option<String>,
    responses: Vec<(String, String)>,
    security: Option<String>,
    tags: Vec<String>,
}

impl Parse for RegisterArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut summary = String::new();
        let mut params = Vec::new();
        let mut request_body = None;
        let mut response_model = None;
        let mut overload_responses = Vec::new();
        let mut security = None;
        let mut tags = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let val: LitStr = input.parse()?;

            match key.to_string().as_str() {
                "summary" => summary = val.value(),
                "request_body" => request_body = Some(val.value()),
                "response_model" => response_model = Some(val.value()),
                "security" => security = Some(val.value()),
                "tags" => {
                    for t in val.value().split(',') {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            tags.push(trimmed.to_string());
                        }
                    }
                }
                "overload_responses" => {
                    for r in val.value().split(',') {
                        if let Some((code, model)) = r.split_once(':') {
                            overload_responses
                                .push((code.trim().to_string(), model.trim().to_string()));
                        }
                    }
                }
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

        let responses = if overload_responses.is_empty() {
            let success_model = response_model.unwrap_or_else(|| "string".to_string());
            vec![
                ("200".to_string(), success_model),
                ("400".to_string(), "ErrorResponse".to_string()),
                ("401".to_string(), "ErrorResponse".to_string()),
                ("404".to_string(), "ErrorResponse".to_string()),
                ("500".to_string(), "ErrorResponse".to_string()),
            ]
        } else {
            overload_responses
        };

        if tags.is_empty() {
            tags.push("Default".to_string());
        }

        Ok(RegisterArgs {
            summary,
            params,
            request_body,
            responses,
            security,
            tags,
        })
    }
}

#[proc_macro_attribute]
pub fn register_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as RegisterArgs);
    let input = parse_macro_input!(item as ItemFn);

    let mut extracted_path = format!("/{}", input.sig.ident);
    let mut extracted_method = "GET".to_string();

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
    let param_names = args.params.iter().map(|(n, _)| n);
    let param_types = args.params.iter().map(|(_, t)| t);
    let res_codes = args.responses.iter().map(|(c, _)| c);
    let res_models = args.responses.iter().map(|(_, m)| m);
    let tags_list = args.tags.iter();

    let req_body = match args.request_body {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let sec_quote = match args.security {
        Some(s) => quote! { Some(#s) },
        None => quote! { None },
    };

    let expanded = quote! {
        #input

        inventory::submit! {
            ferivonus_swagger_gen::RoutePlugin {
                path: #extracted_path,
                method: #extracted_method,
                summary: #summary,
                params: &[ #( (#param_names, #param_types) ),* ],
                request_body: #req_body,
                responses: &[ #( (#res_codes, #res_models) ),* ],
                security: #sec_quote,
                tags: &[ #( #tags_list ),* ],
            }
        }
    };

    TokenStream::from(expanded)
}

fn parse_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let rust_type = segment.ident.to_string();

                if rust_type == "Vec" || rust_type == "Option" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_mapped = parse_type(inner_ty);
                            if rust_type == "Vec" {
                                return format!("array:{}", inner_mapped);
                            } else {
                                return format!("option:{}", inner_mapped);
                            }
                        }
                    }
                }

                return match rust_type.as_str() {
                    "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
                    | "isize" => "integer".to_string(),
                    "f32" | "f64" => "number".to_string(),
                    "bool" => "boolean".to_string(),
                    "String" | "str" | "char" => "string".to_string(),
                    "DateTime" | "NaiveDate" | "NaiveDateTime" => "string:date-time".to_string(),
                    "Uuid" => "string:uuid".to_string(),
                    _ => format!("ref:{}", rust_type),
                };
            }
            "string".to_string()
        }
        syn::Type::Reference(type_ref) => parse_type(&type_ref.elem),
        _ => "string".to_string(),
    }
}

#[proc_macro_derive(ApiSchema)]
pub fn derive_api_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();

    let mut is_enum = false;
    let mut fields = Vec::new();

    match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(named_fields) => {
                for field in &named_fields.named {
                    let field_name = field.ident.as_ref().unwrap().to_string();
                    let field_type = parse_type(&field.ty);
                    fields.push(quote! { (#field_name, #field_type) });
                }
            }
            syn::Fields::Unnamed(unnamed_fields) => {
                for (i, field) in unnamed_fields.unnamed.iter().enumerate() {
                    let field_name = format!("field_{}", i);
                    let field_type = parse_type(&field.ty);
                    fields.push(quote! { (#field_name, #field_type) });
                }
            }
            syn::Fields::Unit => {}
        },
        syn::Data::Enum(data) => {
            is_enum = true;
            for variant in &data.variants {
                let variant_name = variant.ident.to_string();
                fields.push(quote! { (#variant_name, "") });
            }
        }
        _ => {}
    }

    let schema_type = if is_enum { "enum" } else { "object" };

    let expanded = quote! {
        inventory::submit! {
            ferivonus_swagger_gen::SchemaPlugin {
                name: #struct_name_str,
                schema_type: #schema_type,
                fields: &[ #( #fields ),* ],
            }
        }
    };

    TokenStream::from(expanded)
}
