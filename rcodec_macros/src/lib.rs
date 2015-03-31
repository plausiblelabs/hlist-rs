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

use syntax::ast::{TokenTree, TtToken};
use syntax::parse::token;
use syntax::ext::base::{MacResult, MacEager, DummyResult};
use syntax::util::small_vector::SmallVector;

use syntax::codemap::Span;
use syntax::ptr::P;
use syntax::ast;
use syntax::ast::{Item, MetaItem, Expr};
use syntax::attr;
use syntax::ext::base::{Decorator, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ext::deriving::generic::{combine_substructure, MethodDef, Substructure, TraitDef, ty};
use syntax::parse::token::{intern, InternedString};
use rustc::plugin::Registry;
 
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("record_struct", expand_record_struct);
    reg.register_syntax_extension(intern("HListSupport"), Decorator(Box::new(expand_hlist_support)));
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

fn hnil_path() -> ty::Path<'static> {
    ty::Path::new(vec!["rcodec","hlist","HNil"])
}

fn hcons_path(types: Vec<&'static str>) -> ty::Path<'static> {
    if types.len() == 0 {
        hnil_path()
    } else {
        let head = types[0];
        let tail = if types.len() > 1 { types[1..].to_vec() } else { vec![] };
        let head_ty = ty::Ty::Literal(ty::Path::new_local(head));
        let tail_ty = ty::Ty::Literal(hcons_path(tail));
        ty::Path::new_(vec!["rcodec","hlist","HCons"], None, vec![Box::new(head_ty), Box::new(tail_ty)], true)
    }
}

pub fn expand_hlist_support(cx: &mut ExtCtxt, span: Span, mitem: &MetaItem, item: &Item, push: &mut FnMut(P<Item>)) {
    // let struct_def = match item.node {
    //     ast::ItemStruct(ref struct_def, ref _generics) => {
    //         struct_def
    //     },
    //     _ => {
    //         cx.span_err(span, "`HListSupport` may only be applied to structs");
    //         return;
    //     }
    // };

    let hlist_type = || {
        // TODO: I have no idea how to convert ast::Ty to ty::Path :(
        //let field_tys: Vec<P<ast::Ty>> = struct_def.fields.iter()
        // let field_tys: Vec<&'static str> = struct_def.fields.iter()
        //     .map(|field| field.node.ty.clone())
        //     .map(|field_ty| field_ty.to_string())
        //     .collect();
        hcons_path(vec!["u8", "u8"])
        //hcons_path(field_tys);
    };

    // impl AsHList<hlist_type> for <struct_type>
    // e.g.:
    //   impl AsHList<HCons<u8, HNil>> for Foo
    let trait_def = TraitDef {
        span: span,
        attributes: Vec::new(),
        path: ty::Path::new_(vec!["rcodec","codec","AsHList"], None, vec![Box::new(ty::Ty::Literal(hlist_type()))], true),
        additional_bounds: Vec::new(),
        generics: ty::LifetimeBounds::empty(),
        methods: vec![
            // fn from_hlist(<hlist_type>) -> Self;
            MethodDef {
                name: "from_hlist",
                generics: ty::LifetimeBounds::empty(),
                explicit_self: None,
                args: vec![ty::Ty::Literal(hlist_type())],
                ret_ty: ty::Self_,
                attributes: vec![attr::mk_attr_outer(attr::mk_attr_id(),
                                                     attr::mk_name_value_item_str(InternedString::new("inline"),
                                                                                  InternedString::new("always")))],
                combine_substructure: combine_substructure(Box::new(from_hlist_substructure))
            },
            // fn to_hlist(&self) -> <hlist_type>;
            MethodDef {
                name: "to_hlist",
                generics: ty::LifetimeBounds::empty(),
                explicit_self: ty::borrowed_explicit_self(),
                args: vec![],
                ret_ty: ty::Ty::Literal(hlist_type()),
                attributes: vec![attr::mk_attr_outer(attr::mk_attr_id(),
                                                     attr::mk_name_value_item_str(InternedString::new("inline"),
                                                                                  InternedString::new("always")))],
                combine_substructure: combine_substructure(Box::new(to_hlist_substructure))
            }
        ],
        associated_types: vec![],
    };
    trait_def.expand(cx, mitem, item, |a| push(a))
}

fn from_hlist_substructure(cx: &mut ExtCtxt, trait_span: Span, _substr: &Substructure) -> P<Expr> {
    let stmts = Vec::new();

    // let fields = match *substr.fields {
    //     Struct(ref fs) | EnumMatching(_, _, ref fs) => fs,
    //     _ => cx.span_bug(trait_span, "Unsupported substructure in `HListSupport`")
    // };

    // TODO: Generate impl that looks like this:
    // match hlist {
    //     record_struct_hlist_pattern!($($fieldname),+) => $stype { $($fieldname: $fieldname),+ }
    // }

    cx.expr_block(cx.block(trait_span, stmts, None))
}

fn to_hlist_substructure(cx: &mut ExtCtxt, trait_span: Span, _substr: &Substructure) -> P<Expr> {
    let stmts = Vec::new();

    // let fields = match *substr.fields {
    //     Struct(ref fs) | EnumMatching(_, _, ref fs) => fs,
    //     _ => cx.span_bug(trait_span, "Unsupported substructure in `HListSupport`")
    // };

    // TODO: Generate impl that looks like this:
    // hlist!($(self.$fieldname.clone()),+)
    // for &FieldInfo { ref self_, span, .. } in fields.iter() {
    //     stmts.push(make_hlist(span, self_.clone()));
    // }

    cx.expr_block(cx.block(trait_span, stmts, None))
}
