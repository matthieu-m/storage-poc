//! Simple implementation of `MultiElementStorage`.

use core::{alloc::{Allocator, AllocError, Layout}, fmt::{self, Debug}, marker::Unsize, ptr::{NonNull, Pointee}};

use crate::{alternative::Builder, traits::{ElementStorage, MultiElementStorage}, utils};

use super::AllocatorBuilder;

/// Generic allocator-based MultiElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct MultiElement<A> {
    allocator: A,
}

impl<A> MultiElement<A> {
    /// Attempts to create an instance of SingleElement.
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }
}

impl<A: Allocator> ElementStorage for MultiElement<A> {
    type Handle<T: ?Sized + Pointee> = NonNull<T>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is valid, and points to valid meta-data, if not valid data.
        let layout = Layout::for_value_raw(handle.as_ptr() as *const T);

        //  Safety:
        //  -   `handle` is valid.
        //  -   `layout` matches the one used for the allocation.
        self.allocator.deallocate(handle.cast(), layout);
    }

    unsafe fn resolve<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        handle
    }

    unsafe fn resolve_mut<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) -> NonNull<T> {
        handle
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        handle
    }
}

impl<A: Allocator> MultiElementStorage for MultiElement<A> {
    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::Metadata) -> Result<Self::Handle<T>, AllocError> {
        let slice = self.allocator.allocate(utils::layout_of::<T>(meta))?;

        let pointer: NonNull<()> = slice.as_non_null_ptr().cast();

        Ok(NonNull::from_raw_parts(pointer, meta))
    }
}

impl<A> Builder<MultiElement<A>> for AllocatorBuilder<A> {
    fn from_storage(storage: MultiElement<A>) -> Self { AllocatorBuilder(storage.allocator) }

    fn into_storage(self) -> MultiElement<A> { MultiElement::new(self.0) }
}

impl<A: Default> Default for MultiElement<A> {
    fn default() -> Self { Self::new(A::default()) }
}

impl<A> Debug for MultiElement<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "MultiElement")
    }
}

//
//  Implementation
//

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    MultiElement::<NonAllocator>::default();
}

#[test]
fn new_unconditional_success() {
    MultiElement::new(NonAllocator);
}

#[test]
fn create_success() {
    let allocator = SpyAllocator::default();

    let mut storage = MultiElement::new(allocator.clone());
    let handle = storage.create(1u32).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_failure() {
    let mut storage = MultiElement::new(NonAllocator);
    storage.create(1u8).unwrap_err();
}

#[test]
fn coerce_success() {
    let allocator = SpyAllocator::default();

    let mut storage = MultiElement::new(allocator.clone());
    let handle = storage.create([1u32, 2, 3]).unwrap();
    let handle = unsafe { storage.coerce::<[u32], _>(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

}
