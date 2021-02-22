//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::{Allocator, AllocError, Layout}, fmt::{self, Debug}, marker::Unsize, ptr::NonNull};

use rfc2580::{self, Pointee};

use crate::{alternative::Builder, traits::{ElementStorage, SingleElementStorage}, utils};

use super::AllocatorBuilder;

/// Generic allocator-based SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<A> {
    allocator: A,
}

impl<A> SingleElement<A> {
    /// Creates an instance of SingleElement.
    pub fn new(allocator: A) -> Self { Self { allocator } }
}

impl<A: Allocator> ElementStorage for SingleElement<A> {
    type Handle<T: ?Sized + Pointee> = NonNull<T>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `element` points to a valid value.
        let layout = Layout::for_value(handle.as_ref());

        //  Safety:
        //  -   `element` was allocated by call to `self.allocator`.
        //  -   `layout` matches that of allocation.
        self.allocator.deallocate(handle.cast(), layout);
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> { handle }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        handle
    }
}

impl<A: Allocator> SingleElementStorage for SingleElement<A> {
    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::MetaData) -> Result<Self::Handle<T>, AllocError> {
        let slice = self.allocator.allocate(utils::layout_of::<T>(meta))?;

        let pointer: NonNull<u8> = slice.as_non_null_ptr().cast();

        Ok(rfc2580::from_non_null_parts(meta, pointer))
    }
}

impl<A> Builder<SingleElement<A>> for AllocatorBuilder<A> {
    fn from_storage(storage: SingleElement<A>) -> Self { AllocatorBuilder(storage.allocator) }

    fn into_storage(self) -> SingleElement<A> { SingleElement::new(self.0) }
}

impl<A: Default> Default for SingleElement<A> {
    fn default() -> Self {
        let allocator = A::default();
        Self::new(allocator)
    }
}

impl<A> Debug for SingleElement<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleElement::<NonAllocator>::default();
}

#[test]
fn new_unconditional_success() {
    SingleElement::new(NonAllocator);
}

#[test]
fn create_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::new(allocator.clone());
    let handle = storage.create(1u32).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_failure() {
    let mut storage = SingleElement::new(NonAllocator);
    storage.create(1u8).unwrap_err();
}

#[test]
fn coerce() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::new(allocator.clone());
    let handle = storage.create([1u8, 2]).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    let handle = unsafe { storage.coerce::<[u8], _>(handle) };

    assert_eq!([1, 2], unsafe { storage.get(handle).as_ref() });

    unsafe { storage.destroy(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

} // mod tests
