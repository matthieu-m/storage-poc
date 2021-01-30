//! Proof-of-Concept implementation of a Box parameterized by a Storage.

use core::{fmt::{self, Debug}, marker::{PhantomData, Unsize}, ops::{Deref, DerefMut}};

use rfc2580::Pointee;

use crate::traits::SingleElementStorage;

/// A PoC Box.
pub struct RawBox<T: ?Sized + Pointee, S: SingleElementStorage<T>> {
    storage: S,
    _marker: PhantomData<T>,
}

impl<T: Pointee, S: SingleElementStorage<T>> RawBox<T, S> {
    /// Creates an instance of Self, containing `value` stored in `storage`.
    pub fn new(value: T, mut storage: S) -> Result<Self, (T, S)> {
        match storage.create(value) {
            Ok(_) => Ok(RawBox { storage, _marker: PhantomData }),
            Err(value) => Err((value, storage)),
        }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage<T>> RawBox<T, S> {
    /// Creates an instance of Self, containing `value` stored in `storage`.
    pub fn new_unsize<V: Unsize<T>>(value: V, mut storage: S) -> Result<Self, (V, S)> {
        match storage.create_unsize(value) {
            Ok(_) => Ok(RawBox { storage, _marker: PhantomData }),
            Err(value) => Err((value, storage)),
        }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage<T>> Deref for RawBox<T, S> {
    type Target = T;

    fn deref(&self) -> &T {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        let pointer = unsafe { self.storage.get().as_ptr() };

        //  Safety:
        //  -   `pointer` is pointing to a valid value.
        unsafe { &*pointer }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage<T>> DerefMut for RawBox<T, S> {
    fn deref_mut(&mut self) -> &mut T {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        let pointer = unsafe { self.storage.get().as_ptr() };

        //  Safety:
        //  -   `pointer` is pointing to a valid value.
        unsafe { &mut *pointer }
    }
}

impl<T: ?Sized + Pointee, S: SingleElementStorage<T>> Drop for RawBox<T, S> {
    fn drop(&mut self) {
        //  Safety:
        //  -   There is a value stored, as per constructor's invariants.
        unsafe { self.storage.destroy() };
    }
}

impl<T: ?Sized + Pointee + Debug, S: SingleElementStorage<T>> Debug for RawBox<T, S> {
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
    let storage = SingleElement::<u8, u8>::new().unwrap();
    let mut boxed = RawBox::new(1, storage).unwrap();

    assert_eq!(1u8, *boxed);

    *boxed = 2;

    assert_eq!(2u8, *boxed);
}

#[test]
fn slice_storage() {
    let storage = SingleElement::<[u8], [u8; 4]>::new_unsize();
    let mut boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);
}

#[test]
fn trait_storage() {
    let storage = SingleElement::<dyn Debug, [u8; 4]>::new_unsize();
    let boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
}

}

#[cfg(all(test, feature = "alloc"))]
mod test_small {

use crate::small::SingleElement;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn sized_inline() {
    let storage = SingleElement::<u8, u8, _>::new(NonAllocator);
    let mut boxed = RawBox::new(1, storage).unwrap();

    assert_eq!(1u8, *boxed);

    *boxed = 2;

    assert_eq!(2u8, *boxed);
}

#[test]
fn sized_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<u32, u8, _>::new(allocator.clone());
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
    let storage = SingleElement::<u32, u8, _>::new(NonAllocator);
    RawBox::new(1, storage).unwrap_err();
}

#[test]
fn slice_inline() {
    let storage = SingleElement::<[u8], [u8; 4], _>::new(NonAllocator);
    let mut boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!([1u8, 2, 3], &*boxed);

    boxed[2] = 4;

    assert_eq!([1u8, 2, 4], &*boxed);
}

#[test]
fn slice_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<[u8], [u8; 2], _>::new(allocator.clone());
    let mut boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

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
    let storage = SingleElement::<[u8], [u8; 2], _>::new(NonAllocator);
    RawBox::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

#[test]
fn trait_inline() {
    let storage = SingleElement::<dyn Debug, [u8; 4], _>::new(NonAllocator);
    let boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
}

#[test]
fn trait_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<dyn Debug, [u8; 2], _>::new(allocator.clone());
    let boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn trait_failure() {
    let storage = SingleElement::<dyn Debug, [u8; 2], _>::new(NonAllocator);
    RawBox::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

}

#[cfg(all(test, feature = "alloc"))]
mod test_allocator {

use crate::allocator::SingleElement;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn sized_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<u32, _>::new(allocator.clone());
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
    let storage = SingleElement::<u8, _>::new(NonAllocator);
    RawBox::new(1, storage).unwrap_err();
}

#[test]
fn slice_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<[u8], _>::new(allocator.clone());
    let mut boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

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
    let storage = SingleElement::<[u8], _>::new(NonAllocator);
    RawBox::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

#[test]
fn trait_allocated() {
    let allocator = SpyAllocator::default();

    let storage = SingleElement::<dyn Debug, _>::new(allocator.clone());
    let boxed = RawBox::new_unsize([1u8, 2, 3], storage).unwrap();

    assert_eq!("RawBox{ [1, 2, 3] }", format!("{:?}", boxed));
    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    drop(boxed);

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn trait_failure() {
    let storage = SingleElement::<dyn Debug, _>::new(NonAllocator);
    RawBox::new_unsize([1u8, 2, 3], storage).unwrap_err();
}

}
