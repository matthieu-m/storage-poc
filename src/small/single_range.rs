//! Small implementation of `SingleRangeStorage`.

use core::{alloc::{Allocator, AllocError}, fmt::{self, Debug}, mem::MaybeUninit, ptr::NonNull};

use crate::{
    allocator::{self, AllocatorBuilder},
    alternative::{self, DefaultBuilder},
    inline,
    traits::{RangeStorage, SingleRangeStorage},
};

/// Generic inline SingleRangeStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleRange<S, A> {
    inner: Inner<S, A>,
}

impl<S: Default, A> SingleRange<S, A> {
    /// Create new instance.
    pub fn new(allocator: A) -> Self { Self { inner: Inner::first(Default::default(), AllocatorBuilder(allocator)) } }
}

impl<S, A: Allocator> RangeStorage for SingleRange<S, A> {
    type Handle<T> = <Inner<S, A> as RangeStorage>::Handle<T>;

    type Capacity = <Inner<S, A> as RangeStorage>::Capacity;

    fn maximum_capacity<T>(&self) -> Self::Capacity { self.inner.maximum_capacity::<T>() }

    unsafe fn release<T>(&mut self, handle: Self::Handle<T>) {
        self.inner.release(handle)
    }

    unsafe fn get<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        self.inner.get(handle)
    }

    unsafe fn try_grow<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        self.inner.try_grow(handle, new_capacity)
    }

    unsafe fn try_shrink<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        self.inner.try_shrink(handle, new_capacity)
    }
}

impl<S, A: Allocator> SingleRangeStorage for SingleRange<S, A> {
    fn acquire<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        self.inner.acquire(capacity)
    }
}

impl<S, A> Debug for SingleRange<S, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRange")
    }
}

impl<S: Default, A: Default> Default for SingleRange<S, A> {
    fn default() -> Self { Self::new(A::default()) }
}


//
//  Implementation
//

type Inner<S, A> =
    alternative::SingleRange<inline::SingleRange<usize, S, 1>, allocator::SingleRange<A>, DefaultBuilder, AllocatorBuilder<A>>;

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleRange::<u8, NonAllocator>::default();
}

#[test]
fn new_unconditional_success() {
    SingleRange::<u8, _>::new(NonAllocator);
}

#[test]
fn acquire_zero_success() {
    let mut storage = SingleRange::<[u8; 2], _>::new(NonAllocator);

    let handle = storage.acquire::<String>(0).unwrap();

    assert_eq!(0, unsafe { storage.get(handle) }.len());
}

#[test]
fn acquire_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleRange::<[u8; 2], _>::new(allocator.clone());
    let handle = storage.acquire::<String>(1).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.release(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn acquire_failure() {
    let mut storage = SingleRange::<[u8; 2], _>::new(NonAllocator);
    storage.acquire::<String>(1).unwrap_err();
}

} // mod tests
