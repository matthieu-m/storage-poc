//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::Layout, fmt::{self, Debug}, marker::{PhantomData, Unsize}, mem, ops::CoerceUnsized, ptr::{self, NonNull}};
use alloc::alloc::{Allocator, Global};

use crate::traits::{Element, SingleElementStorage};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<T: ?Sized, A: Allocator = Global> {
    pointer: NonNull<T>,
    allocator: A,
    _marker: PhantomData<T>,
}

impl<T: ?Sized, A: Allocator> SingleElement<T, A> {
    /// Attempts to create an instance of SingleElement.
    pub fn new(allocator: A) -> Self {
        Self { pointer: dangling(), allocator, _marker: PhantomData }
    }
}

impl<T: ?Sized, A: Allocator> SingleElementStorage<T> for SingleElement<T, A> {
    fn create(&mut self, value: T) -> Result<Element<T>, T>
        where
            T: Sized,
    {
        self.allocate(value, |v| NonNull::from(v))
    }

    fn create_unsize<V: Unsize<T>>(&mut self, value: V) -> Result<Element<T>, V> {
        self.allocate(value, |v| NonNull::from(v as &mut T))
    }

    unsafe fn destroy(&mut self) {
        //  Safety:
        //  -   The storage is assumed to be initialized.
        let element = self.get();

        //  Safety:
        //  -   `element` points to a valid value.
        let layout = Layout::for_value(element.as_ref());

        //  Safety:
        //  -   `element` is exclusively accessible.
        //  -   `element` is suitably aligned.
        //  -   `element` points to a valid value.
        ptr::drop_in_place(element.as_ptr());

        //  Safety:
        //  -   `element` was allocated by call to `self.allocator`.
        //  -   `layout` matches that of allocation.
        self.allocator.deallocate(element.cast(), layout);
    }

    unsafe fn get(&self) -> Element<T> { self.pointer }
}

impl<T: ?Sized, U: ?Sized, A: Allocator> CoerceUnsized<SingleElement<U, A>> for SingleElement<T, A>
    where
        T: Unsize<U>,
{
}

impl<T: ?Sized, A: Allocator + Default> Default for SingleElement<T, A> {
    fn default() -> Self {
        let allocator = A::default();
        Self::new(allocator)
    }
}

impl<T: ?Sized, A: Allocator> Debug for SingleElement<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

//
//  Implementation
//

//  Attempts to create a dangling pointer, if `T` is `!Sized`, since apparently `NonNull:dangling()` cannot.
fn dangling<T: ?Sized>() -> NonNull<T> {
    const SIZE: usize = mem::size_of::<usize>() * 2;
    assert!(SIZE >= mem::size_of::<*mut T>());

    let array: [u8; SIZE] = [0xf0; SIZE];

    let pointer : *mut T = unsafe { mem::transmute_copy(&array) };

    //  Safety:
    //  -   `pointer` is not null.
    unsafe { NonNull::new_unchecked(pointer) }
}

impl<T: ?Sized, A: Allocator> SingleElement<T, A> {
    //  Allocates memory for `data`, and initializes the memory with `data`.
    //
    //  #   Safety
    //
    //  Assumes that the layout of storage has been validated to fit `V`.
    fn allocate<V, F>(&mut self, data: V, convert: F) -> Result<Element<T>, V>
        where
            F: FnOnce(&mut V) -> Element<T>,
    {
        if let Ok(mut slice) = self.allocator.allocate(Layout::for_value(&data)) {
            //  Safety:
            //  -   `slice` is initialized.
            let pointer = unsafe { slice.as_mut() }.as_mut_ptr() as *mut V;

            //  Safety:
            //  -   `pointer` points to an appropriate (layout-wise) memory area.
            unsafe { ptr::write(pointer, data) }; 

            //  Safety:
            //  -   `pointer` is not null.
            self.pointer = convert(unsafe { &mut *pointer });

            Ok(self.pointer)
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
    SingleElement::<u8>::default();
}

#[test]
fn new_unconditional_success() {
    SingleElement::<u8, _>::new(NonAllocator);
}

#[test]
fn create_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<u32,  _>::new(allocator.clone());
    storage.create(1u32).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy() };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_failure() {
    let mut storage = SingleElement::<u8, _>::new(NonAllocator);
    storage.create(1u8).unwrap_err();
}

#[test]
fn create_unsize_allocated_success() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<[u32], _>::new(allocator.clone());
    storage.create_unsize([1u32, 2, 3]).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    unsafe { storage.destroy() };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn create_unsize_failure() {
    let mut storage = SingleElement::<[u8], _>::new(NonAllocator);
    storage.create_unsize([1u8, 2, 3]).unwrap_err();
}

#[test]
fn coerce_unsized() {
    let allocator = SpyAllocator::default();

    let mut storage = SingleElement::<[u8; 2], _>::new(allocator.clone());
    storage.create([1u8, 2]).unwrap();

    assert_eq!(1, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    let mut storage : SingleElement<[u8], _> = storage;

    assert_eq!([1, 2], unsafe { storage.get().as_ref() });

    unsafe { storage.destroy() };

    assert_eq!(1, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

}
