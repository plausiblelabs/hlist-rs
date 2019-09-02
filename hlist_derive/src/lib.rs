//
// Copyright (c) 2015-2019 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields};

#[proc_macro_derive(HListSupport)]
pub fn hlist_support_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Check that the input type is a struct
    let data_struct: DataStruct;
    if let Data::Struct(s) = input.data {
        data_struct = s
    } else {
        panic!("`HListSupport` may only be applied to structs")
    }

    // Check that the struct has named fields, since that's the only
    // type we support at the moment
    let fields: Fields;
    if let Fields::Named(_) = data_struct.fields {
        fields = data_struct.fields
    } else {
        panic!("`HListSupport` may only be applied to structs with named fields")
    }

    // Extract the struct name
    let struct_name = &input.ident;

    // Build the HList type
    let hlist_type = hlist_type(fields.iter());

    // Build the HList pattern
    let hlist_pat = hlist_pattern(fields.iter());

    // Build the struct initializer
    let struct_field_init = struct_field_init(fields.iter());

    // Build the HList initializer for ToHList
    let hlist_cloned_init = hlist_cloned_init(fields.iter());

    // Build the HList initializer for IntoHList
    let hlist_init = hlist_init(fields.iter());

    // Build the output
    let expanded = quote! {
        // Include the FromHList impl
        #[allow(dead_code)]
        impl FromHList<#hlist_type> for #struct_name {
            fn from_hlist(hlist: #hlist_type) -> Self {
                match hlist {
                    #hlist_pat => #struct_name { #struct_field_init }
                }
            }
        }

        // Include the ToHList impl
        #[allow(dead_code)]
        impl ToHList<#hlist_type> for #struct_name {
            fn to_hlist(&self) -> #hlist_type {
                #hlist_cloned_init
            }
        }

        // Include the IntoHList impl
        #[allow(dead_code)]
        impl IntoHList<#hlist_type> for #struct_name {
            fn into_hlist(self) -> #hlist_type {
                #hlist_init
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

/// Recursive function that builds up an HList type using the types from a
/// series of Fields.
fn hlist_type(mut fields: syn::punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(field) => {
            let lhs = field.ty.to_token_stream();
            let rhs = hlist_type(fields);
            quote!(HCons<#lhs, #rhs>)
        },
        None => {
            quote!(HNil)
        }
    }
}

/// Recursive function that builds up an HList pattern using the names from a
/// series of Fields.
fn hlist_pattern(mut fields: syn::punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(field) => {
            let lhs = field.ident.as_ref();
            let rhs = hlist_pattern(fields);
            quote!(HCons(#lhs, #rhs))
        },
        None => {
            quote!(HNil)
        }
    }
}

/// Recursive function that builds up an HList initializer using the names from a
/// series of Fields.
fn hlist_init(mut fields: syn::punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(field) => {
            let lhs = field.ident.as_ref();
            let rhs = hlist_init(fields);
            quote!(HCons(self.#lhs, #rhs))
        },
        None => {
            quote!(HNil)
        }
    }
}

/// Recursive function that builds up an HList initializer using the names from a
/// series of Fields.
fn hlist_cloned_init(mut fields: syn::punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(field) => {
            let lhs = field.ident.as_ref();
            let rhs = hlist_cloned_init(fields);
            quote!(HCons(self.#lhs.clone(), #rhs))
        },
        None => {
            quote!(HNil)
        }
    }
}

/// Builds up a struct initializer list using the names from a series of Fields.
fn struct_field_init(fields: syn::punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    let field_names = fields.map(|f| f.ident.as_ref());
    quote!(#(#field_names),*)
}
