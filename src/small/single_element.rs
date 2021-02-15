//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::Layout, fmt::{self, Debug}, marker::Unsize, mem::{self, MaybeUninit}, ptr::{self, NonNull}};
use alloc::alloc::{Allocator, Global};

use rfc2580::{self, Pointee};

use crate::{traits::SingleElementStorage, utils};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<S, A: Allocator = Global> {
    inner: Inner<S>,
    allocator: A,
}

impl<S, A: Allocator> SingleElement<S, A> {
    /// Attempts to create an instance of SingleElement.
    pub fn new(allocator: A) -> Self {
        let inner = Inner::default();
        Self { inner, allocator }
    }
}

impl<S, A: Allocator> SingleElementStorage for SingleElement<S, A> {
    type Handle<T: ?Sized + Pointee> = SingleElementHandle<T>;

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        let meta = rfc2580::into_non_null_parts(NonNull::from(&value)).0;

        if utils::validate_layout::<T, S>().is_ok() {
            let mut storage = MaybeUninit::<S>::uninit();

            //  Safety:
            //  -   `storage.as_mut_ptr()` points to an appropriate (layout-wise) memory area.
            unsafe { ptr::write(storage.as_mut_ptr() as *mut T, value) };

            self.inner = Inner::Inline(storage);

            Ok(SingleElementHandle(meta))
        } else if let Ok(mut storage) = self.allocator.allocate(Layout::for_value(&value)) {
            let pointer = unsafe { storage.as_mut() }.as_mut_ptr() as *mut T;

            //  Safety:
            //  -   `pointer` points to an appropriate (layout-wise) memory area.
            unsafe { ptr::write(pointer, value) };

            //  Safety:
            //  -   `pointer` is not null.
            let pointer = unsafe { NonNull::new_unchecked(pointer) };

            self.inner = Inner::Allocated(pointer.cast());

            Ok(SingleElementHandle(meta))
        } else {
            Err(value)
        }
    }

    unsafe fn release<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        let element = self.get(handle);

        //  Safety:
        //  -   `handle` is valid, and points to valid meta-data, if not valid data.
        let layout = Layout::for_value_raw(element.as_ptr() as *const T);

        if let Inner::Allocated(pointer) = mem::replace(&mut self.inner, Inner::default()) {
            //  Safety:
            //  -   `pointer` was allocated by call to `self.allocator`.
            //  -   `layout` matches that of allocation.
            self.allocator.deallocate(pointer.cast(), layout);
        }
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        let pointer: NonNull<u8> = match self.inner {
            Inner::Inline(ref data) => NonNull::from(data).cast(),
            Inner::Allocated(pointer) => pointer,
        };

        rfc2580::from_non_null_parts(handle.0, pointer)
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.get(handle);

        let meta = rfc2580::into_raw_parts(element.as_ptr() as *mut U).0;

        SingleElementHandle(meta)
    }
}

impl<S, A: Allocator + Default> Default for SingleElement<S, A> {
    fn default() -> Self { Self::new(A::default()) }
}

impl<S, A: Allocator> Debug for SingleElement<S, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

/// SingleElementHandle for SingleElement.
pub struct SingleElementHandle<T: ?Sized + Pointee>(T::MetaData);

impl<T: ?Sized + Pointee> Clone for SingleElementHandle<T> {
    fn clone(&self) -> Self { *self }
}

impl<T: ?Sized + Pointee> Copy for SingleElementHandle<T> {}

impl<T: ?Sized + Pointee> Debug for SingleElementHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElementHandle")
    }
}

//
//  Implementation
//
enum Inner<S> {
    Inline(MaybeUninit<S>),
    Allocated(NonNull<u8>)
}

impl< S> Default for Inner<S> {
    fn default() -> Self { Inner::Inline(MaybeUninit::uninit()) }
}

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleElement::<u8>::default();
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
