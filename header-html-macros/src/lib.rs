use proc_macro::TokenStream;
use quote::quote;
use syn::{
    ItemFn, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

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
