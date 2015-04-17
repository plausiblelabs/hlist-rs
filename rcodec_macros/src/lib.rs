//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

#![crate_type = "dylib"]
#![feature(rustc_private, plugin_registrar, quote, slice_patterns, collections)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::ast::{Ident, Item, ItemStruct, MetaItem, StructFieldKind, StructDef, TokenTree};
use syntax::attr::AttrMetaMethods;
use syntax::codemap::Span;
use syntax::parse::token::intern;
use syntax::ext::base::{ExtCtxt, Decorator};
use syntax::ptr::P;
use rustc::plugin::Registry;
 
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("AsHList"), Decorator(Box::new(expand_hlist_support)));
}

/// Handles the `AsHList` attribute applied to a struct by generating an `AsHList` implementation for the struct.
pub fn expand_hlist_support(cx: &mut ExtCtxt, _span: Span, mitem: &MetaItem, item: &Item, push: &mut FnMut(P<Item>)) {
    match item.node {
        ItemStruct(ref struct_def, _) => {
            for spanned_field in struct_def.fields.iter() {
                let field = &spanned_field.node;
                match field.kind {
                    StructFieldKind::NamedField(_, _) => {}
                    _ => {
                        cx.span_err(mitem.span, "`Lensed` may only be applied to structs with named fields");
                        return;
                    }
                }
            }
            derive_as_hlist(cx, push, item, struct_def);
        }
        
        _ => {
            cx.span_err(mitem.span, "`AsHList` may only be applied to structs");
            return;
        }
    }
}

/// Generates an implementation of the `AsHList` implementation for a struct.
fn derive_as_hlist(cx: &mut ExtCtxt, push: &mut FnMut(P<Item>), struct_item: &Item, struct_def: &StructDef) {
    // Extract the struct name
    let struct_name = struct_item.ident;

    // Extract the field names
    let field_names: Vec<ast::Ident> = struct_def.fields.iter().map(|f| {
        let field = &f.node;
        match field.kind {
            StructFieldKind::NamedField(ident, _) => ident,
            _ => {
                panic!("`AsHList` may only be applied to structs with named fields")
            }
        }
    }).collect();
    
    // Extract the field types
    let field_types: Vec<P<ast::Ty>> = struct_def.fields.iter().map(|f| f.node.ty.clone()).collect();

    // Build the HList type
    let hlist_type = hlist_type(cx, &field_types);

    // Build the HList pattern
    let hlist_pat = hlist_pattern(cx, &field_names);

    // Build the struct initializer
    let struct_field_init = struct_field_init(cx, &field_names);

    // Build the HList initializer
    let hlist_init = hlist_init(cx, &field_names);
    
    // Push the impl item
    push_new_item(push, struct_item, quote_item!(
        cx,
        #[allow(dead_code)]
        impl AsHList<$hlist_type> for $struct_name {
            fn from_hlist(hlist: $hlist_type) -> Self {
                match hlist {
                    $hlist_pat => $struct_name { $struct_field_init }
                }
            }
            
            fn to_hlist(&self) -> $hlist_type {
                $hlist_init
            }
        }
    ));
}

/// Pushes the given item into the AST.  The new item will inherit the lint attributes of the existing item
/// from which the new item was derived.
fn push_new_item(push: &mut FnMut(P<Item>), existing_item: &Item, new_item_ptr: Option<P<Item>>) {
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
    push(P(ast::Item {
        attrs: attrs,
        ..(*new_item).clone()
    }))
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
fn hlist_init(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> Vec<TokenTree> {
    if field_names.is_empty() {
        quote_tokens!(cx, HNil)
    } else {
        let lhs = field_names[0].clone();
        let rhs = hlist_init(cx, &field_names[1..]);
        quote_tokens!(cx, HCons(self.$lhs.clone(), $rhs))
    }
}

/// Builds up a struct initializer list from an array of field names.
fn struct_field_init(cx: &mut ExtCtxt, field_names: &[ast::Ident]) -> Vec<TokenTree> {
    let mut tts: Vec<TokenTree> = Vec::new();
    if field_names.len() >= 1 {
        let f0 = field_names[0].clone();
        tts.push_all(&quote_tokens!(cx, $f0: $f0));

        for field_name in &field_names[1..] {
            tts.push_all(&quote_tokens!(cx, , $field_name: $field_name));
        }
    }
    tts
}
