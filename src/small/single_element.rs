//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::Layout, fmt::{self, Debug}, marker::{PhantomData, Unsize}, mem::{self, MaybeUninit}, ptr::{self, NonNull}};
use alloc::alloc::{Allocator, Global};

use rfc2580::{self, Pointee};

use crate::{traits::{Element, SingleElementStorage}, utils};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<T: ?Sized + Pointee, S, A: Allocator = Global> {
    inner: Inner<T, S>,
    allocator: A,
    _marker: PhantomData<T>,
}

impl<T: ?Sized, S, A: Allocator> SingleElement<T, S, A> {
    /// Attempts to create an instance of SingleElement.
    pub fn new(allocator: A) -> Self {
        let inner = Inner::default();
        Self { inner, allocator, _marker: PhantomData }
    }
}

impl<T: ?Sized + Pointee, S, A: Allocator> SingleElementStorage<T> for SingleElement<T, S, A> {
    fn create(&mut self, value: T) -> Result<Element<T>, T>
        where
            T: Sized,
    {
        let meta = rfc2580::into_raw_parts(&value as *const T).0;

        //  Safety:
        //  -   The layout of `S` has been validated as part of `new`.
        unsafe { self.write(meta, value) }
    }

    fn create_unsize<V: Unsize<T>>(&mut self, value: V) -> Result<Element<T>, V> {
        let meta = rfc2580::into_raw_parts(&value as &T as *const T).0;

        //  Safety:
        //  -   The layout of `S` has been validated, just above.
        unsafe { self.write(meta, value) }
    }

    unsafe fn destroy(&mut self) {
        //  Safety:
        //  -   The storage is assumed to be initialized.
        let element = self.get().as_ptr();

        //  Safety:
        //  -   `element` points to a valid value.
        let layout = Layout::for_value(&*element);

        //  Safety:
        //  -   `element` is exclusively accessible.
        //  -   `element` is suitably aligned.
        //  -   `element` points to a valid value.
        ptr::drop_in_place(element);

        if let Inner::Allocated(pointer) = mem::replace(&mut self.inner, Inner::default()) {
            //  Safety:
            //  -   `pointer` was allocated by call to `self.allocator`.
            //  -   `layout` matches that of allocation.
            self.allocator.deallocate(pointer.cast(), layout);
        }
    }

    unsafe fn get(&self) -> Element<T> {
        match self.inner {
            Inner::Inline(meta, ref data) => {
                //  Safety:
                //  -   `self.meta` and `self.data` are assumed to be suitably initialized.
                let meta = *meta.as_ptr();
                let data = data.as_ptr() as *const u8;

                let pointer = rfc2580::from_raw_parts(meta, data);

                //  Safety:
                //  -   `pointer` is not null, as `data` is not null.
                NonNull::new_unchecked(pointer as *mut T)
            },
            Inner::Allocated(pointer) => pointer,
        }
    }
}

impl<T: ?Sized, S, A: Allocator + Default> Default for SingleElement<T, S, A> {
    fn default() -> Self {
        let allocator = A::default();
        Self::new(allocator)
    }
}

impl<T: ?Sized, S, A: Allocator> Debug for SingleElement<T, S, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

//
//  Implementation
//
enum Inner<T: ?Sized + Pointee, S> {
    Inline(MaybeUninit<T::MetaData>, MaybeUninit<S>),
    Allocated(NonNull<T>)
}

impl<T: ?Sized, S> Default for Inner<T, S> {
    fn default() -> Self { Inner::Inline(MaybeUninit::uninit(), MaybeUninit::uninit()) }
}

impl<T: ?Sized + Pointee, S, A: Allocator> SingleElement<T, S, A> {
    //  Writes an instance of `V`, with the appropriate meta-data for `T`, in the storage.
    //
    //  Assumes that the storage is unoccupied, otherwise the current memory is overwritten, which may lead to resource
    //  leaks.
    //
    //  #   Safety
    //
    //  Assumes that the layout of storage has been validated to fit `V`.
    unsafe fn write<V>(&mut self, meta: T::MetaData, data: V) -> Result<Element<T>, V> {
        if let Ok(_) = utils::validate_layout::<V, S>() {
            let meta = MaybeUninit::new(meta);
            let mut storage = MaybeUninit::<S>::uninit();

            //  Safety:
            //  -   `storage.as_mut_ptr()` points to an appropriate (layout-wise) memory area.
            ptr::write(storage.as_mut_ptr() as *mut V, data);

            self.inner = Inner::Inline(meta, storage);

            Ok(self.get())
        } else if let Ok(mut storage) = self.allocator.allocate(Layout::for_value(&data)) {
            //  Safety:
            //  -   `pointer` points to an appropriate (layout-wise) memory area.
            ptr::write(storage.as_mut().as_mut_ptr() as *mut V, data); 

            //  Safety:
            let pointer = rfc2580::from_raw_parts(meta, storage.as_ref().as_ptr());
            
            //  Safety:
            //  -   `pointer` is not null.
            let pointer = NonNull::new_unchecked(pointer as *mut T);

            self.inner = Inner::Allocated(pointer);

            Ok(pointer)
        } else {
            Err(data)
        }
    }
}

#[cfg(test)]
mod tests {

use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn default_unconditional_success() {
    SingleElement::<u8, u8>::default();
}

#[test]
fn new_unconditional_success() {
    SingleElement::<u8, u8, _>::new(NonAllocator);
}

#[test]
fn create_inline_success() {
    let mut storage = SingleElement::<u8, [u8; 2], _>::new(NonAllocator);
    storage.create(1u8).unwrap();
}

#[test]
fn create_allocated_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<u32, u8, _>::new(allocator.clone());
    storage.create(1u32).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy() };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_insufficient_size() {
    let mut storage = SingleElement::<[u8; 2], u8, _>::new(NonAllocator);
    storage.create([1u8, 2]).unwrap_err();
}

#[test]
fn create_insufficient_alignment() {
    let mut storage = SingleElement::<u32, [u8; 32], _>::new(NonAllocator);
    storage.create(1u32).unwrap_err();
}

#[test]
fn create_unsize_inline_success() {
    let mut storage = SingleElement::<[u8], u32, _>::new(NonAllocator);
    storage.create_unsize([1u8, 2, 3]).unwrap();
}

#[test]
fn create_unsize_allocated_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<[u32], u8, _>::new(allocator.clone());
    storage.create_unsize([1u32, 2, 3]).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy() };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_unsize_insufficient_size() {
    let mut storage = SingleElement::<[u8], u8, _>::new(NonAllocator);
    storage.create_unsize([1u8, 2, 3]).unwrap_err();
}

#[test]
fn create_unsize_insufficient_alignment() {
    let mut storage = SingleElement::<[u32], [u8; 32], _>::new(NonAllocator);
    storage.create_unsize([1u32]).unwrap_err();
}

}
