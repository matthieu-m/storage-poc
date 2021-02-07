//! Proof-of-Concept implementation of a Box parameterized by a Storage.

use core::{fmt::{self, Debug}, marker::Unsize, ops::{CoerceUnsized, Deref, DerefMut}};

use rfc2580::Pointee;

use crate::traits::SingleElementStorage;

/// A PoC Box.
pub struct RawBox<T: ?Sized + Pointee, S: SingleElementStorage> {
    storage: S,
    handle: S::Handle<T>,
}

impl<T: Pointee, S: SingleElementStorage> RawBox<T, S> {
    /// Creates an instance of Self, containing `value` stored in `storage`.
    pub fn new(value: T, mut storage: S) -> Result<Self, (T, S)> {
        match storage.create(value) {
            Ok(handle) => Ok(RawBox { storage, handle }),
            Err(value) => Err((value, storage)),
        }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage> RawBox<T, S> {
    /// Creates an instance of Self, containing `value` stored in `storage`.
    pub fn new_unsize<V: Unsize<T>>(value: V, mut storage: S) -> Result<Self, (V, S)> {
        match storage.create(value) {
            Ok(handle) => Ok({
                let handle = unsafe { storage.coerce::<T, _>(handle) };
                RawBox { storage, handle }
            }),
            Err(value) => Err((value, storage)),
        }
    }
}

impl<T, U, S> CoerceUnsized<RawBox<U, S>> for RawBox<T, S>
    where
        T: ?Sized + Pointee,
        U: ?Sized + Pointee,
        S: SingleElementStorage,
        S::Handle<T>: CoerceUnsized<S::Handle<U>>,
{
}

impl<T: ?Sized + Pointee, S: SingleElementStorage> Deref for RawBox<T, S> {
    type Target = T;

    fn deref(&self) -> &T {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        let pointer = unsafe { self.storage.get(self.handle).as_ptr() };

        //  Safety:
        //  -   `pointer` is pointing to a valid value.
        unsafe { &*pointer }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage> DerefMut for RawBox<T, S> {
    fn deref_mut(&mut self) -> &mut T {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        let pointer = unsafe { self.storage.get(self.handle).as_ptr() };

        //  Safety:
        //  -   `pointer` is pointing to a valid value.
        unsafe { &mut *pointer }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage> Drop for RawBox<T, S> {
    fn drop(&mut self) {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        unsafe { self.storage.destroy(self.handle) };
    }
}

impl<T: ?Sized + Pointee + Debug, S: SingleElementStorage> Debug for RawBox<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let value: &T = &*self;
        write!(f, "RawBox{{ {:?} }}", value)
    }
}

#[cfg(test)]
mod test_inline {

use crate::inline::SingleElement;

use super::*;

#[test]
fn sized_storage() {
    let storage = SingleElement::<u8>::new();
    let mut boxed = RawBox::new(1u8, storage).unwrap();

    assert_eq!(1u8, *boxed);

    *boxed = 2;

    assert_eq!(2u8, *boxed);
}

#[test]
fn slice_storage() {
    let storage = SingleElement::<[u8; 4]>::new();
    let mut boxed: RawBox<[u8], _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);
}

#[test]
fn trait_storage() {
    let storage = SingleElement::<[u8; 4]>::new();
    let boxed: RawBox<dyn Debug, _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
}

} // mod test_inline

#[cfg(all(test, feature = "alloc"))]
mod test_small {

use crate::small::SingleElement;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn sized_inline() {
    let storage = SingleElement::<u8, _>::new(NonAllocator);
    let mut boxed = RawBox::new(1u8, storage).unwrap();

    assert_eq!(1u8, *boxed);

    *boxed = 2;

    assert_eq!(2u8, *boxed);
}

#[test]
fn sized_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<u8, _>::new(allocator.clone());
    let mut boxed = RawBox::new(1u32, storage).unwrap();

    assert_eq!(1u32, *boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    *boxed = 2;

    assert_eq!(2u32, *boxed);

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn sized_failure() {
    let storage = SingleElement::<u8, _>::new(NonAllocator);
    RawBox::new(1, storage).unwrap_err();
}

#[test]
fn slice_inline() {
    let storage = SingleElement::<[u8; 4], _>::new(NonAllocator);
    let mut boxed : RawBox<[u8], _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);
}

#[test]
fn slice_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<[u8; 2], _>::new(allocator.clone());
    let mut boxed : RawBox<[u8], _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn slice_failure() {
    let storage = SingleElement::<[u8; 2], _>::new(NonAllocator);
    RawBox::<[u8], _>::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

#[test]
fn trait_inline() {
    let storage = SingleElement::<[u8; 4], _>::new(NonAllocator);
    let boxed : RawBox<dyn Debug, _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
}

#[test]
fn trait_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<[u8; 2], _>::new(allocator.clone());
    let boxed : RawBox<dyn Debug, _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn trait_failure() {
    let storage = SingleElement::<[u8; 2], _>::new(NonAllocator);
    RawBox::<dyn Debug, _>::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

} // mod test_small

#[cfg(all(test, feature = "alloc"))]
mod test_allocator {

use crate::allocator::SingleElement;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn sized_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::new(allocator.clone());
    let mut boxed = RawBox::new(1, storage).unwrap();

    assert_eq!(1u32, *boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());


    *boxed = 2;

    assert_eq!(2u32, *boxed);

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn sized_failure() {
    let storage = SingleElement::new(NonAllocator);
    RawBox::new(1, storage).unwrap_err();
}

#[test]
fn slice_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::new(allocator.clone());
    let mut boxed : RawBox<[u8], _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn slice_failure() {
    let storage = SingleElement::new(NonAllocator);
    RawBox::<[u8], _>::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

/*

FIXME: ICE...

#[test]
fn slice_coerce() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::new(allocator.clone());
    let boxed = RawBox::new([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], *boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    let coerced : RawBox<[u8], _> = boxed;

    assert_eq!([1u8, 2, 3], *coerced);

    drop(coerced);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

*/

#[test]
fn trait_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::new(allocator.clone());
    let boxed : RawBox<dyn Debug, _> = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn trait_failure() {
    let storage = SingleElement::new(NonAllocator);
    RawBox::<dyn Debug, _>::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

/*

//FIXME: ICE

#[test]
fn trait_coerce() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::new(allocator.clone());
    let boxed = RawBox::new([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], *boxed);
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    let coerced : RawBox<dyn Debug, _> = boxed;

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", coerced));

    drop(coerced);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

*/

} // mod test_allocator
