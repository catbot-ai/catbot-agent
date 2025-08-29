use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ToJsonSchema, attributes(gemini))]
pub fn derive_to_json_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut fn_name = None;
    let mut fn_description = None;

    for attr in &input.attrs {
        if attr.path().is_ident("gemini") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit = value.parse::<syn::LitStr>()?;
                    fn_name = Some(lit.value());
                    Ok(())
                } else if meta.path.is_ident("description") {
                    let value = meta.value()?;
                    let lit = value.parse::<syn::LitStr>()?;
                    fn_description = Some(lit.value());
                    Ok(())
                } else {
                    Err(meta.error("unsupported gemini attribute at struct level, expected 'name' or 'description'"))
                }
            }).unwrap_or_else(|e| panic!("Failed to parse struct-level gemini attribute: {e}"));
        }
    }

    let fn_name = fn_name.unwrap_or_else(|| format!("execute_{}", name.to_string().to_lowercase()));
    let fn_description = fn_description.unwrap_or_else(|| format!("Function for {name}"));

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("ToJsonSchema only supports named fields"),
        },
        _ => panic!("ToJsonSchema only supports structs"),
    };

    let mut properties = Vec::new();
    let mut required = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_type = &field.ty;

        let json_type = match field_type {
            syn::Type::Path(type_path) if type_path.path.is_ident("String") => "string",
            syn::Type::Path(type_path) if type_path.path.is_ident("bool") => "boolean",
            syn::Type::Path(type_path) if type_path.path.is_ident("i32") => "integer",
            syn::Type::Path(type_path) if type_path.path.is_ident("i64") => "integer",
            syn::Type::Path(type_path) if type_path.path.is_ident("f32") => "number",
            syn::Type::Path(type_path) if type_path.path.is_ident("f64") => "number",
            _ => panic!(
                "Unsupported field type '{}' for field '{}'",
                quote!(#field_type),
                field_name
            ),
        };

        let mut description = None;
        let mut optional = false;

        for attr in &field.attrs {
            if attr.path().is_ident("gemini") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let lit = value.parse::<syn::LitStr>()?;
                        description = Some(lit.value());
                        Ok(())
                    } else if meta.path.is_ident("optional") {
                        if meta.input.is_empty() {
                            optional = true;
                            Ok(())
                        } else {
                            Err(meta.error("'optional' attribute takes no value"))
                        }
                    } else {
                        Err(meta.error(
                            "unsupported gemini attribute, expected 'description' or 'optional'",
                        ))
                    }
                })
                .unwrap_or_else(|e| {
                    panic!("Failed to parse field gemini attribute for '{field_name}': {e}",)
                });
            }
        }

        let description = description.unwrap_or_else(|| format!("No description for {field_name}"));

        properties.push(quote! {
            #field_name: {
                "type": #json_type,
                "description": #description
            }
        });

        if !optional {
            required.push(field_name);
        }
    }

    let expanded = quote! {
        impl json_schema::ToJsonSchema for #name {
            fn to_json_schema() -> json_schema::Value {
                json_schema::json!({
                    "name": #fn_name,
                    "description": #fn_description,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            #(#properties),*
                        },
                        "required": [#(#required),*]
                    }
                })
            }
        }
    };

    TokenStream::from(expanded)
}
