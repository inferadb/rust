//! Derive macros for the InferaDB SDK.
//!
//! This crate provides derive macros for implementing the `Resource` and `Subject`
//! traits, enabling type-safe authorization operations.
//!
//! ## Usage
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! inferadb = { version = "0.1", features = ["derive"] }
//! ```
//!
//! ## Examples
//!
//! ```rust,ignore
//! use inferadb::derive::{Resource, Subject};
//!
//! #[derive(Resource)]
//! #[resource(type = "document")]
//! struct Document {
//!     #[resource(id)]
//!     id: String,
//!     title: String,
//! }
//!
//! #[derive(Subject)]
//! #[subject(type = "user")]
//! struct User {
//!     #[subject(id)]
//!     id: String,
//!     name: String,
//! }
//!
//! // Now you can use these with the InferaDB SDK
//! let doc = Document { id: "readme".into(), title: "README".into() };
//! let user = User { id: "alice".into(), name: "Alice".into() };
//!
//! // Type-safe API
//! assert_eq!(doc.as_resource_ref(), "document:readme");
//! assert_eq!(user.as_subject_ref(), "user:alice");
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Ident, LitStr, Result, parse_macro_input};

/// Derive macro for implementing the `Resource` trait.
///
/// ## Attributes
///
/// - `#[resource(type = "...")]` - Required. The resource type name.
/// - `#[resource(id)]` - Required on one field. The field containing the resource ID.
///
/// ## Example
///
/// ```rust,ignore
/// #[derive(Resource)]
/// #[resource(type = "document")]
/// struct Document {
///     #[resource(id)]
///     id: String,
///     title: String,
/// }
/// ```
#[proc_macro_derive(Resource, attributes(resource))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_resource_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for implementing the `Subject` trait.
///
/// ## Attributes
///
/// - `#[subject(type = "...")]` - Required. The subject type name.
/// - `#[subject(id)]` - Required on one field. The field containing the subject ID.
///
/// ## Example
///
/// ```rust,ignore
/// #[derive(Subject)]
/// #[subject(type = "user")]
/// struct User {
///     #[subject(id)]
///     id: String,
///     name: String,
/// }
/// ```
#[proc_macro_derive(Subject, attributes(subject))]
pub fn derive_subject(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_subject_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_resource_impl(input: DeriveInput) -> Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse #[resource(type = "...")] from struct attributes
    let resource_type = parse_type_attr(&input, "resource")?.ok_or_else(|| {
        Error::new_spanned(&input, "missing #[resource(type = \"...\")] attribute")
    })?;

    // Find the field with #[resource(id)]
    let id_field = find_id_field(&input.data, "resource")?;

    Ok(quote! {
        impl #impl_generics ::inferadb::Resource for #name #ty_generics #where_clause {
            fn resource_type() -> &'static str {
                #resource_type
            }

            fn resource_id(&self) -> &str {
                &self.#id_field
            }
        }
    })
}

fn derive_subject_impl(input: DeriveInput) -> Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse #[subject(type = "...")] from struct attributes
    let subject_type = parse_type_attr(&input, "subject")?.ok_or_else(|| {
        Error::new_spanned(&input, "missing #[subject(type = \"...\")] attribute")
    })?;

    // Find the field with #[subject(id)]
    let id_field = find_id_field(&input.data, "subject")?;

    Ok(quote! {
        impl #impl_generics ::inferadb::Subject for #name #ty_generics #where_clause {
            fn subject_type() -> &'static str {
                #subject_type
            }

            fn subject_id(&self) -> &str {
                &self.#id_field
            }
        }
    })
}

/// Parse the `type = "..."` value from `#[resource(...)]` or `#[subject(...)]` attributes.
fn parse_type_attr(input: &DeriveInput, attr_name: &str) -> Result<Option<String>> {
    for attr in &input.attrs {
        if !attr.path().is_ident(attr_name) {
            continue;
        }

        let mut type_value = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("type") {
                let value: LitStr = meta.value()?.parse()?;
                type_value = Some(value.value());
            }
            Ok(())
        })?;

        if type_value.is_some() {
            return Ok(type_value);
        }
    }
    Ok(None)
}

/// Find the field marked with `#[resource(id)]` or `#[subject(id)]`.
fn find_id_field(data: &Data, attr_name: &str) -> Result<Ident> {
    let fields = match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            Fields::Unnamed(_) => {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    "tuple structs are not supported",
                ));
            },
            Fields::Unit => {
                return Err(Error::new(
                    proc_macro2::Span::call_site(),
                    "unit structs are not supported",
                ));
            },
        },
        Data::Enum(_) => {
            return Err(Error::new(proc_macro2::Span::call_site(), "enums are not supported"));
        },
        Data::Union(_) => {
            return Err(Error::new(proc_macro2::Span::call_site(), "unions are not supported"));
        },
    };

    for field in fields {
        for attr in &field.attrs {
            if !attr.path().is_ident(attr_name) {
                continue;
            }

            // Check for #[resource(id)] or #[subject(id)]
            let mut is_id_field = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    is_id_field = true;
                }
                Ok(())
            });

            if is_id_field {
                return field
                    .ident
                    .clone()
                    .ok_or_else(|| Error::new_spanned(field, "expected named field"));
            }
        }
    }

    Err(Error::new(
        proc_macro2::Span::call_site(),
        format!("no field marked with #[{}(id)]", attr_name),
    ))
}

#[cfg(test)]
mod tests {
    // Tests are in the integration tests since proc-macros can't be tested directly
}
