//! Small implementation of `SingleElementStorage`.

use core::{alloc::{Allocator, AllocError}, fmt::{self, Debug}, marker::Unsize, ptr::{NonNull, Pointee}};

use crate::{
    allocator::{self, AllocatorBuilder},
    alternative::{self, DefaultBuilder},
    inline,
    traits::{ElementStorage, SingleElementStorage},
};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<S, A> {
    inner: Inner<S, A>,
}

impl<S: Default, A> SingleElement<S, A> {
    /// Create new instance.
    pub fn new(allocator: A) -> Self { Self { inner: Inner::first(Default::default(), AllocatorBuilder(allocator)) } }
}

impl<S, A: Allocator> ElementStorage for SingleElement<S, A> {
    type Handle<T: ?Sized + Pointee> = <Inner<S, A> as ElementStorage>::Handle<T>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        self.inner.deallocate(handle)
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        self.inner.get(handle)
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        self.inner.coerce(handle)
    }
}

impl<S, A: Allocator> SingleElementStorage for SingleElement<S, A> {
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        self.inner.create(value)
    }

    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::Metadata) -> Result<Self::Handle<T>, AllocError> {
        self.inner.allocate(meta)
    }
}

impl<S, A> Debug for SingleElement<S, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

impl<S: Default, A: Default> Default for SingleElement<S, A> {
    fn default() -> Self { Self::new(A::default()) }
}


//
//  Implementation
//

type Inner<S, A> =
    alternative::SingleElement<inline::SingleElement<S>, allocator::SingleElement<A>, DefaultBuilder, AllocatorBuilder<A>>;

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleElement::<u8, NonAllocator>::default();
}

#[test]
fn new_unconditional_success() {
    SingleElement::<u8, _>::new(NonAllocator);
}

#[test]
fn create_inline_success() {
    let mut storage = SingleElement::<[u8; 2], _>::new(NonAllocator);
    storage.create(1u8).unwrap();
}

#[test]
fn create_allocated_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<u8, _>::new(allocator.clone());
    let handle = storage.create(1u32).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_insufficient_size() {
    let mut storage = SingleElement::<u8, _>::new(NonAllocator);
    storage.create([1u8, 2]).unwrap_err();
}

#[test]
fn create_insufficient_alignment() {
    let mut storage = SingleElement::<[u8; 32], _>::new(NonAllocator);
    storage.create(1u32).unwrap_err();
}

#[test]
fn coerce_allocated() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<u8, _>::new(allocator.clone());
    let handle = storage.create([1u32, 2, 3]).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    let handle = unsafe { storage.coerce::<[u32], _>(handle) };

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

} // mod tests
