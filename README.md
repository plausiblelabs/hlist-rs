# pl-hlist

[![Build Status][travis-badge]][travis-url]
[![Crates.io][crates-badge]][crates-url]
[![Docs.rs][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]

[travis-badge]: https://travis-ci.com/plausiblelabs/hlist-rs.svg?branch=master
[travis-url]: https://travis-ci.com/plausiblelabs/hlist-rs
[crates-badge]: https://img.shields.io/crates/v/pl-hlist.svg
[crates-url]: https://crates.io/crates/pl-hlist
[docs-badge]: https://docs.rs/pl-hlist/badge.svg
[docs-url]: https://docs.rs/pl-hlist
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE

This Rust library provides support for heterogeneous lists (known as `HList`s).  An `HList` is a functional,
tuple-like, strongly-typed data structure that can contain elements of differing types.

There are three layers in this library:

- A basic `HList` data structure consisting of `HCons` and `HNil` types.
- An `hlist!` macro for constructing an `HList` from a series of elements.
- An `HListSupport` plugin/attribute that, when declared for a struct, allows for easy conversion
of struct instances to/from an `HList` representation.

See the next section for more details on usage of these layers.

## Usage

Add a dependency to your `Cargo.toml`:

```toml
[dependencies]
pl-hlist = "1.0"
```

Then, in your crate:

```rust
use pl_hlist::*;
```

An `HList` can be constructed manually as follows:

```rust
let x: HCons<u8, HCons<u32, HNil>> = HCons(1u8, HCons(666u32, HNil));
```

The `hlist!` macro provides a convenient shorthand for constructing an `HList`:

```rust
let x: HCons<u8, HCons<u32, HNil>> = hlist!(1u8, 666u32);
```

The custom `HListSupport` derive attribute can be applied to a struct declaration to automatically implement support for converting that struct to/from an `HList` representation:

```rust
#[derive(HListSupport)]
struct TestStruct {
    foo: u8,
    bar: u32
}

let hlist0 = hlist!(1u8, 666u32);
let s = TestStruct::from_hlist(hlist0);
assert_eq!(s.foo, 1u8);
assert_eq!(s.bar, 666u32);
let hlist1 = s.into_hlist();
assert_eq!(hlist0, hlist1);
```

# License

`pl-hlist` is distributed under an MIT license.  See LICENSE for more details.
