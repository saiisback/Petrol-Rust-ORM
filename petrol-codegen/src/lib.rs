use inflector::cases::snakecase::to_snake_case;
use petrol_core::schema::{Field, FieldType, Model, ScalarType, Schema};
use petrol_core::PetrolError;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub struct GenerateOptions {
    pub module_name: String,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            module_name: "petrol".into(),
        }
    }
}

pub fn generate(schema: &Schema) -> Result<String, PetrolError> {
    generate_with_options(schema, &GenerateOptions::default())
}

pub fn generate_with_options(
    schema: &Schema,
    options: &GenerateOptions,
) -> Result<String, PetrolError> {
    if schema.models.is_empty() {
        return Err(PetrolError::validation("schema contains no models"));
    }

    let module_ident = format_ident!("{}", options.module_name);
    let mut modules = Vec::new();
    let mut re_exports = Vec::new();

    for model in &schema.models {
        modules.push(render_model_module(model));
        let module_name = format_ident!("{}", to_snake_case(&model.name));
        let struct_ident = format_ident!("{}", model.name);
        re_exports.push(quote! { pub use self::#module_ident::#module_name::#struct_ident; });
    }

    let tokens: TokenStream = quote! {
        pub mod #module_ident {
            use serde::{Deserialize, Serialize};
            #( #modules )*
        }
        #( #re_exports )*
    };

    Ok(tokens.to_string())
}

fn render_model_module(model: &Model) -> TokenStream {
    let module_ident = format_ident!("{}", to_snake_case(&model.name));
    let struct_ident = format_ident!("{}", model.name);

    let fields: Vec<_> = model
        .fields
        .iter()
        .filter(|field| matches!(field.r#type, FieldType::Scalar(_, _)))
        .collect();

    let struct_fields: Vec<_> = fields
        .iter()
        .map(|field| render_struct_field(field))
        .collect();

    let module_tokens: TokenStream = quote! {
        pub mod #module_ident {
            use super::*;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct #struct_ident {
                #( #struct_fields ),*
            }
        }
    };

    module_tokens
}

fn render_struct_field(field: &Field) -> TokenStream {
    let field_ident = format_ident!("{}", to_snake_case(&field.name));
    let ty = scalar_rust_type(field);
    let serde_attr = if field.column_name() != field.name {
        let column = field.column_name();
        quote! { #[serde(rename = #column)] }
    } else {
        quote! {}
    };

    quote! {
        #serde_attr
        pub #field_ident: #ty
    }
}

fn scalar_rust_type(field: &Field) -> TokenStream {
    let (scalar, modifiers) = match &field.r#type {
        FieldType::Scalar(scalar, modifiers) => (scalar, modifiers),
        FieldType::Relation(_) => panic!("relation fields not supported here"),
    };

    let base = match scalar {
        ScalarType::Int => quote! { i32 },
        ScalarType::BigInt => quote! { i64 },
        ScalarType::Float => quote! { f64 },
        ScalarType::Decimal => quote! { rust_decimal::Decimal },
        ScalarType::String => quote! { String },
        ScalarType::Boolean => quote! { bool },
        ScalarType::DateTime => quote! { chrono::DateTime<chrono::Utc> },
        ScalarType::Date => quote! { chrono::NaiveDate },
        ScalarType::Uuid => quote! { uuid::Uuid },
        ScalarType::Json => quote! { serde_json::Value },
        ScalarType::Bytes => quote! { Vec<u8> },
    };

    let ty = if modifiers.list {
        quote! { Vec<#base> }
    } else {
        base
    };

    if modifiers.optional {
        quote! { Option<#ty> }
    } else {
        ty
    }
}
