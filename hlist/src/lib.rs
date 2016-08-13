//
// Copyright (c) 2016 Plausible Labs Cooperative, Inc.
// All rights reserved.
//

// The following allows for using macros defined in the separate hlist_macros crate.
#![feature(plugin, custom_attribute)]
#![plugin(hlist_macros)]

// The following is necessary to make exported macros visible.
#[macro_use]
mod macros;

mod hlist;

pub use self::macros::*;
pub use self::hlist::*;
