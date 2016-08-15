//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// HList macro implementations based on:
//   https://github.com/epsilonz/shoggoth.rs
//

/// Shorthand for building an `HList` from the given elements.
///
/// # Examples
///
/// ```
/// #[macro_use]
/// extern crate hlist;
/// use hlist::*;
/// 
/// # fn main() {
/// let x: HCons<u8, HCons<u32, HNil>> = hlist!(1u8, 666u32);
/// # }
/// ```
#[macro_export]
macro_rules! hlist {
    {} => {
        HNil
    };
    { $head:expr } => {
        HCons($head, HNil)
    };
    { $head:expr, $($tail:expr),+ } => {
        HCons($head, hlist!($($tail),+))
    };
}
