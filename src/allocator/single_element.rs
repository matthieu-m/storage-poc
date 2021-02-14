//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::{Allocator, Layout}, fmt::{self, Debug}, marker::Unsize, ptr::{self, NonNull}};
use alloc::alloc::Global;

use rfc2580::Pointee;

use crate::traits::SingleElementStorage;

/// Generic allocator-based SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<A: Allocator = Global> {
    allocator: A,
}

impl<A: Allocator> SingleElement<A> {
    /// Creates an instance of SingleElement.
    pub fn new(allocator: A) -> Self { Self { allocator } }
}

impl<A: Allocator> SingleElementStorage for SingleElement<A> {
    type Handle<T: ?Sized + Pointee> = NonNull<T>;

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        if let Ok(mut slice) = self.allocator.allocate(Layout::new::<T>()) {
            //  Safety:
            //  -   `slice` is initialized.
            let pointer = unsafe { slice.as_mut() }.as_mut_ptr() as *mut T;

            //  Safety:
            //  -   `pointer` points to an appropriate (layout-wise) memory area.
            unsafe { ptr::write(pointer, value) }; 

            //  Safety:
            //  -   `pointer` is not null.
            Ok(unsafe { NonNull::new_unchecked(pointer) })
        } else {
            Err(value)
        }
    }

    unsafe fn release<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
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

impl<A: Allocator + Default> Default for SingleElement<A> {
    fn default() -> Self {
        let allocator = A::default();
        Self::new(allocator)
    }
}

impl<A: Allocator> Debug for SingleElement<A> {
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
