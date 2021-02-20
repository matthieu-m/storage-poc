//! Simple implementation of `MultiElementStorage<T>`.

use core::{alloc::Layout, fmt::{self, Debug}, marker::Unsize, ptr::{self, NonNull}};
use alloc::alloc::{Allocator, Global};

use rfc2580::Pointee;

use crate::traits::{ElementStorage, MultiElementStorage};

/// Generic allocator-based MultiElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct MultiElement<A: Allocator = Global> {
    allocator: A,
}

impl<A: Allocator> MultiElement<A> {
    /// Attempts to create an instance of SingleElement.
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }
}

impl<A: Allocator> ElementStorage for MultiElement<A> {
    type Handle<T: ?Sized + Pointee> = NonNull<T>;

    unsafe fn release<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is valid, and points to valid meta-data, if not valid data.
        let layout = Layout::for_value_raw(handle.as_ptr() as *const T);

        //  Safety:
        //  -   `handle` is valid.
        //  -   `layout` matches the one used for the allocation.
        self.allocator.deallocate(handle.cast(), layout);
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        handle
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        handle
    }
}

impl<A: Allocator> MultiElementStorage for MultiElement<A> {
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T>  {
        if let Ok(mut slice) = self.allocator.allocate(Layout::new::<T>()) {
            //  Safety:
            //  -   `slice` is initialized.
            let pointer = unsafe { slice.as_mut() }.as_mut_ptr() as *mut T;

            //  Safety:
            //  -   `pointer` points to an appropriate (layout-wise) memory area.
            unsafe { ptr::write(pointer, value) }; 

            //  Safety:
            //  -   `pointer` is not null and valid.
            Ok(NonNull::from(unsafe { &mut *pointer }))
        } else {
            Err(value)
        }
    }
}

impl<A: Allocator + Default> Default for MultiElement<A> {
    fn default() -> Self { Self::new(A::default()) }
}

impl<A: Allocator> Debug for MultiElement<A> {
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
