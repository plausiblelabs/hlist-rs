//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

#![crate_type = "dylib"]
#![feature(rustc_private, plugin_registrar, quote, slice_patterns)]

extern crate syntax;
extern crate rustc;
extern crate rustc_plugin;

use syntax::ast;
use syntax::ast::{Item, ItemKind, MetaItem, StructField, VariantData};
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::parse::token::intern;
use syntax::ext::base::{Annotatable, ExtCtxt, MultiItemDecorator, SyntaxExtension};
use syntax::ptr::P;
use syntax::tokenstream::TokenTree;
use rustc_plugin::Registry;
 
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("HListSupport"), SyntaxExtension::MultiDecorator(Box::new(HListSupportDecorator)));
}

/// Handles the `HListSupport` attribute applied to a struct by generating `FromHList`, `ToHList`, and `IntoHList`
/// implementations for the struct.
struct HListSupportDecorator;
impl MultiItemDecorator for HListSupportDecorator {
    fn expand(&self, cx: &mut ExtCtxt, _span: Span, mitem: &MetaItem, item: &Annotatable, push: &mut FnMut(Annotatable)) {
        match *item {
            Annotatable::Item(ref struct_item) => {
                match struct_item.node {
                    ItemKind::Struct(VariantData::Struct(ref struct_fields, _), _) => {
                        for struct_field in struct_fields.iter() {
                            if struct_field.ident.is_none() {
                                cx.span_err(mitem.span, "`HListSupport` may only be applied to structs with named fields");
                                return;
                            }
                        }
                        derive_as_hlist(cx, push, &struct_item, struct_fields);
                    }
                    _ => {
                        cx.span_err(mitem.span, "`HListSupport` may only be applied to structs");
                        return;
                    }
                }
            }
            _ => {
                cx.span_err(mitem.span, "`HListSupport` may only be applied to struct items");
                return;
            }
        }
    }
}

/// Generates implementations of the `FromHList`, `ToHList`, and `IntoHList` traits for a struct.
fn derive_as_hlist(cx: &mut ExtCtxt, push: &mut FnMut(Annotatable), struct_item: &Item, struct_fields: &Vec<StructField>) {
    // Extract the struct name
    let struct_name = struct_item.ident;

    // Extract the field names
    let field_names: Vec<ast::Ident> = struct_fields.iter().map(|f| {
        match f.ident {
            Some(ident) => ident,
            None => {
                panic!("`HListSupport` may only be applied to structs with named fields")
            }
        }
    }).collect();
    
    // Extract the field types
    let field_types: Vec<P<ast::Ty>> = struct_fields.iter().map(|f| f.ty.clone()).collect();

    // Build the HList type
    let hlist_type = hlist_type(cx, &field_types);

    // Build the HList pattern
    let hlist_pat = hlist_pattern(cx, &field_names);

    // Build the struct initializer
    let struct_field_init = struct_field_init(cx, &field_names);

    // Build the HList initializer for ToHList
    let hlist_cloned_init = hlist_cloned_init(cx, &field_names);

    // Build the HList initializer for IntoHList
    let hlist_init = hlist_init(cx, &field_names);

    // Push the FromHList impl item
    push_new_item(push, struct_item, quote_item!(
        cx,
        #[allow(dead_code)]
        impl FromHList<$hlist_type> for $struct_name {
            fn from_hlist(hlist: $hlist_type) -> Self {
                match hlist {
                    $hlist_pat => $struct_name { $struct_field_init }
                }
            }
        }
    ));

    // Push the ToHList impl item
    push_new_item(push, struct_item, quote_item!(
        cx,
        #[allow(dead_code)]
        impl ToHList<$hlist_type> for $struct_name {
            fn to_hlist(&self) -> $hlist_type {
                $hlist_cloned_init
            }
        }
    ));

    // Push the IntoHList impl item
    push_new_item(push, struct_item, quote_item!(
        cx,
        #[allow(dead_code)]
        impl IntoHList<$hlist_type> for $struct_name {
            fn into_hlist(self) -> $hlist_type {
                $hlist_init
            }
        }
    ));
}

/// Pushes the given item into the AST.  The new item will inherit the lint attributes of the existing item
/// from which the new item was derived.
fn push_new_item(push: &mut FnMut(Annotatable), existing_item: &Item, new_item_ptr: Option<P<Item>>) {
    let new_item = new_item_ptr.unwrap();
    
    // Keep the lint attributes of the previous item to control how the
    // generated implementations are linted
    let mut attrs = new_item.attrs.clone();
    attrs.extend(existing_item.attrs.iter().filter(|a| {
        match &a.name()[..] {
            "allow" | "warn" | "deny" | "forbid" => true,
            _ => false,
        }
    }).cloned());

    // Push the new item into the AST
    push(Annotatable::Item(P(ast::Item {
        attrs: attrs,
        ..(*new_item).clone()
    })))
}

/// Recursive function that builds up an HList type from an array of field types.
fn hlist_type(cx: &mut ExtCtxt, field_types: &[P<ast::Ty>]) -> P<ast::Ty> {
    if field_types.is_empty() {
        quote_ty!(cx, HNil)
    } else {
        let lhs = field_types[0].clone();
        let rhs = hlist_type(cx, &field_types[1..]);
        quote_ty!(cx, HCons<$lhs, $rhs>)
    }
}

/// Recursive function that builds up an HList pattern from an array of field names.
fn hlist_pattern(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> P<ast::Pat> {
    if field_names.is_empty() {
        quote_pat!(cx, HNil)
    } else {
        let lhs = field_names[0].clone();
        let rhs = hlist_pattern(cx, &field_names[1..]);
        quote_pat!(cx, HCons($lhs, $rhs))
    }
}

/// Recursive function that builds up an HList initializer from an array of field names.
fn hlist_init(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> P<ast::Expr> {
    if field_names.is_empty() {
        quote_expr!(cx, HNil)
    } else {
        let lhs = field_names[0].clone();
        let rhs = hlist_init(cx, &field_names[1..]);
        quote_expr!(cx, HCons(self.$lhs, $rhs))
    }
}

/// Recursive function that builds up an HList initializer from an array of field names.
fn hlist_cloned_init(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> P<ast::Expr> {
    if field_names.is_empty() {
        quote_expr!(cx, HNil)
    } else {
        let lhs = field_names[0].clone();
        let rhs = hlist_cloned_init(cx, &field_names[1..]);
        quote_expr!(cx, HCons(self.$lhs.clone(), $rhs))
    }
}

/// Builds up a struct initializer list from an array of field names.
fn struct_field_init(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> Vec<TokenTree> {
    let mut tts: Vec<TokenTree> = Vec::new();
    if field_names.len() >= 1 {
        let f0 = field_names[0].clone();
        tts.extend_from_slice(&quote_tokens!(cx, $f0: $f0));

        for field_name in &field_names[1..] {
            tts.extend_from_slice(&quote_tokens!(cx, , $field_name: $field_name));
        }
    }
    tts
}
