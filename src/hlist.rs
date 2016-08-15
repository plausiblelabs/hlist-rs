//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This implementation based on List type from:
//   https://github.com/epsilonz/shoggoth.rs
//

/// A heterogeneous list that can hold elements of different types.
pub trait HList {
    /// Creates a new `HCons` with the given `X` value in head position.
    fn cons<X>(self, x: X) -> HCons<X, Self> where Self: Sized {
        HCons(x, self)
    }
}

/// An empty `HList` used as the terminal element.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HNil;

impl HList for HNil {
}

/// The "cons" of a head element of type `H` and a tail `HList`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HCons<H, T: HList>(pub H, pub T);

impl<H, T: HList> HCons<H, T> {
    /// Returns a reference to the head element of this list.
    pub fn head(&self) -> &H {
        &self.0
    }

    /// Returns a reference to the tail of this list.
    pub fn tail(&self) -> &T {
        &self.1
    }
}

impl<H, T: HList> HList for HCons<H, T> {
}

/// Allows for conversion from an `HList` to an instance of the `Self` type.
pub trait FromHList<H> where H: HList {
    fn from_hlist(hlist: H) -> Self;
}

/// Allows for copying the contents of `Self` into an `HList`.
pub trait ToHList<H> where H: HList {
    fn to_hlist(&self) -> H;
}

/// Allows for converting (and consuming) `Self` into an `HList`.
pub trait IntoHList<H> where H: HList {
    fn into_hlist(self) -> H;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn head_should_work() {
        let hlist = HCons(1u8, HNil);
        assert_eq!(*hlist.head(), 1u8);
    }

    #[test]
    fn tail_should_work() {
        let hlist = HCons(1u8, HNil);
        assert_eq!(*hlist.tail(), HNil);
    }

    #[test]
    fn hlist_macros_should_work() {
        {
            let hlist1 = HNil;
            let hlist2 = hlist!();
            assert_eq!(hlist1, hlist2);
        }

        {
            let hlist1 = HCons(1u8, HNil);
            let hlist2 = hlist!(1u8);
            assert_eq!(hlist1, hlist2);
        }

        {
            let hlist1 = HCons(1u8, HCons(2i32, HCons("three", HNil)));
            let hlist2 = hlist!(1u8, 2i32, "three");
            assert_eq!(hlist1, hlist2);
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    #[HListSupport]
    struct TestInnerStruct {
        f1: u8,
        f2: u8
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    #[HListSupport]
    struct TestStruct {
        foo: u8,
        bar: TestInnerStruct
    }

    #[test]
    fn converting_struct_to_from_hlist_should_work() {
        {
            let s = TestInnerStruct::from_hlist(hlist!(1u8, 2u8));
            assert_eq!(s.f1, 1u8);
            assert_eq!(s.f2, 2u8);
            let hlist0 = s.to_hlist();
            assert_eq!(hlist0, hlist!(1u8, 2u8));
            let hlist1 = s.into_hlist();
            assert_eq!(hlist0, hlist1);
        }

        {
            let s = TestStruct::from_hlist(hlist!(7u8, TestInnerStruct::from_hlist(hlist!(1u8, 2u8))));
            assert_eq!(s.foo, 7u8);
            assert_eq!(s.bar, TestInnerStruct { f1: 1, f2: 2 });
            let hlist0 = s.to_hlist();
            assert_eq!(hlist0, hlist!(7u8, TestInnerStruct { f1: 1, f2: 2 }));
            let hlist1 = s.into_hlist();
            assert_eq!(hlist0, hlist1);
        }
    }
}
