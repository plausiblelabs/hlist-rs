//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

#![crate_type = "dylib"]
#![feature(rustc_private, plugin_registrar, quote)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::ast::{TokenTree, TtToken};
use syntax::parse::token;
use syntax::codemap::Span;
use syntax::ext::base::{ExtCtxt, MacResult, MacEager, DummyResult};
use syntax::util::small_vector::SmallVector;
use syntax::ptr::P;
use rustc::plugin::Registry;
 
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("record_struct", expand_record_struct)
}

/// This is a roundabout way of dealing with the fact that rustc does not allow for macros that expand to a type.
/// Ideally we would just have a regular record_struct! macro that builds the HList type from the field types that
/// are provided as arguments to the macro, but we can't use a macro to build up the HList type.  As a workaround,
/// we now define record_struct! as a compiler plugin, and then build up the HList type from the raw syntax tree.
/// Kind of awful, but it gets the job done.
fn expand_record_struct(cx: &mut ExtCtxt, span: Span, args: &[TokenTree]) -> Box<MacResult> {
    let usage_error = "record_struct! macro expects arguments in the form: (struct_name, (field_name: field_type),+)";

    // Args should look something like: [Ident, Comma, Ident, Colon, Ident, Comma, Ident, Colon, Ident]
    if args.len() < 5 || args.len() % 2 == 0 {
        cx.span_err(span, usage_error);
        return DummyResult::any(span);
    }

    // Extract the struct name
    let struct_name = match args[0] {
        TtToken(_, token::Ident(name, _)) => name,
        _ => {
            cx.span_err(span, usage_error);
            return DummyResult::any(span);
        }
    };

    // Extract the field tokens
    let field_tokens = &args[2..];

    // Extract the field types
    let mut token_index = 0;
    let mut field_types: Vec<ast::Ident> = Vec::new();
    loop {
        // Extract the field type
        let field_name_and_type: &[TokenTree] = &field_tokens[token_index .. token_index + 3];
        let field_type = match field_name_and_type {
            [TtToken(_, token::Ident(_, _)), TtToken(_, token::Colon), TtToken(_, token::Ident(field_type, _))] => field_type,
            _ => {
                cx.span_err(span, usage_error);
                return DummyResult::any(span);
            }
        };
        field_types.push(field_type);

        // Stop when there are no more commas
        token_index += 3;
        if token_index == field_tokens.len() {
            break;
        }

        // Eat the comma
        match field_tokens[token_index] {
            TtToken(_, token::Comma) => token_index += 1,
            _ => {
                cx.span_err(span, usage_error);
                return DummyResult::any(span);
            }
        };
    }

    // Build the HList type from the field types
    let hlist_type = hlist_type(cx, &*field_types);
    
    // The quote_item! macro needs a Vec when we insert the field tokens back in, so we convert here
    let field_tokens_vec = field_tokens.to_vec();

    // Build the quasiquoted call to the private macro
    MacEager::items(SmallVector::one(
        quote_item!(cx, record_struct_with_hlist_type!($struct_name, $hlist_type, $field_tokens_vec);).unwrap()
    ))
}

/// Recursive function that builds up an HList type from an array of field types.
fn hlist_type(cx: &mut ExtCtxt, field_types: &[ast::Ident]) -> P<ast::Ty> {
    if field_types.is_empty() {
        quote_ty!(cx, HNil)
    } else {
        let lhs = field_types[0];
        let rhs = hlist_type(cx, &field_types[1..]);
        quote_ty!(cx, HCons<$lhs, $rhs>)
    }
}
