//! Simple implementation of `SingleRangeStorage`.

use core::{alloc::{Allocator, AllocError, Layout}, fmt::{self, Debug}, mem::MaybeUninit, ptr::NonNull};

use crate::{alternative::Builder, traits::{RangeStorage, SingleRangeStorage}};

use super::AllocatorBuilder;

/// Generic allocator-based SingleRangeStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleRange<A> {
    allocator: A,
}

impl<A> SingleRange<A> {
    /// Creates an instance of SingleRange.
    pub fn new(allocator: A) -> Self { Self { allocator, } }
}

impl<A: Allocator> RangeStorage for SingleRange<A> {
    type Handle<T> = NonNull<[MaybeUninit<T>]>;

    type Capacity = usize;

    fn maximum_capacity<T>(&self) -> Self::Capacity { usize::MAX }

    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>) {
        if handle.len() > 0 {
            let layout = Self::layout_of(handle);
            let pointer = Self::from_handle(handle);
            self.allocator.deallocate(pointer, layout);
        }
    }

    unsafe fn resolve<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        handle
    }

    unsafe fn resolve_mut<T>(&mut self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        handle
    }

    unsafe fn try_grow<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        debug_assert!(handle.len() < new_capacity);

        if handle.len() == 0 {
            return self.allocate::<T>(new_capacity);
        }

        let old_layout = Self::layout_of(handle);
        let old_pointer = Self::from_handle(handle);

        let new_layout = Self::layout_for::<T>(new_capacity)?;
        let new_pointer = self.allocator.grow(old_pointer, old_layout, new_layout)?;

        Ok(Self::into_handle(new_pointer, new_capacity))
    }

    unsafe fn try_shrink<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        debug_assert!(handle.len() > new_capacity);

        if handle.len() == 0 {
            return Err(AllocError);
        }

        let old_layout = Self::layout_of(handle);
        let old_pointer = Self::from_handle(handle);

        if new_capacity == 0 {
            self.allocator.deallocate(old_pointer, old_layout);
            return Ok(Self::dangling_handle());
        }

        let new_layout = Self::layout_for::<T>(new_capacity)?;
        let new_pointer = self.allocator.shrink(old_pointer, old_layout, new_layout)?;

        Ok(Self::into_handle(new_pointer, new_capacity))
    }
}

impl<A: Allocator> SingleRangeStorage for SingleRange<A> {
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        if capacity == 0 {
            return Ok(Self::dangling_handle());
        }

        let layout = Self::layout_for::<T>(capacity)?;
        let pointer = self.allocator.allocate(layout)?;
        Ok(Self::into_handle(pointer, capacity))
    }
}

impl<A: Allocator> Builder<SingleRange<A>> for A {
    fn from_storage(storage: SingleRange<A>) -> A { storage.allocator }

    fn into_storage(self) -> SingleRange<A> { SingleRange::new(self) }
}

impl<A> Builder<SingleRange<A>> for AllocatorBuilder<A> {
    fn from_storage(storage: SingleRange<A>) -> Self { AllocatorBuilder(storage.allocator) }

    fn into_storage(self) -> SingleRange<A> { SingleRange::new(self.0) }
}

impl<A> Debug for SingleRange<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRangeA")
    }
}

impl<A: Default> Default for SingleRange<A> {
    fn default() -> Self { Self::new(A::default()) }
}

//
//  Implementation
//
impl<A: Allocator> SingleRange<A> {
    fn dangling_handle<T>() -> NonNull<[MaybeUninit<T>]> {
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }

    fn layout_for<T>(capacity: usize) -> Result<Layout, AllocError> {
        debug_assert!(capacity > 0);

        Layout::array::<T>(capacity).map_err(|_| AllocError)
    }

    fn layout_of<T>(handle: NonNull<[MaybeUninit<T>]>) -> Layout {
        debug_assert!(handle.len() > 0);

        Layout::array::<T>(handle.len()).expect("Valid handle")
    }

    fn from_handle<T>(handle: NonNull<[MaybeUninit<T>]>) -> NonNull<u8> {
        debug_assert!(handle.len() > 0);

        handle.as_non_null_ptr().cast()
    }

    fn into_handle<T>(pointer: NonNull<[u8]>, capacity: usize) -> NonNull<[MaybeUninit<T>]> {
        NonNull::slice_from_raw_parts(pointer.as_non_null_ptr().cast(), capacity)
    }
}

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleRange::<NonAllocator>::default();
}

#[test]
fn new_unconditional_success() {
    SingleRange::new(NonAllocator);
}

#[test]
fn allocate_zero_success() {
    let mut storage = SingleRange::new(NonAllocator);

    let slice = storage.allocate::<String>(0).unwrap();

    assert_eq!(0, slice.len());
}

#[test]
fn allocate_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleRange::new(allocator.clone());
    let handle = storage.allocate::<String>(1).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.deallocate(handle) };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn allocate_failure() {
    let mut storage = SingleRange::new(NonAllocator);
    storage.allocate::<String>(1).unwrap_err();
}

} // mod tests
