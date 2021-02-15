//! Simple implementation of `SingleElementStorage<T>`.

use core::{fmt::{self, Debug}, marker::Unsize, mem::MaybeUninit, ptr::{self, NonNull}};

use rfc2580::{self, Pointee};

use crate::{traits::SingleElementStorage, utils};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<S> {
    data: MaybeUninit<S>,
}

impl<S> SingleElement<S> {
    /// Creates an instance of SingleElement.
    pub fn new() -> Self { Self { data: MaybeUninit::uninit(), } }
}

impl<S> SingleElementStorage for SingleElement<S> {
    type Handle<T: ?Sized + Pointee> = SingleElementHandle<T>;

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        if let Err(_) = utils::validate_layout::<T, S>() {
            return Err(value);
        }

        let meta = rfc2580::into_non_null_parts(NonNull::from(&value)).0;

        //  Safety:
        //  -   `self.data` points to an appropriate (layout-wise) memory area.
        unsafe { ptr::write(self.data.as_mut_ptr() as *mut T, value) };

        //  Safety:
        //  -   There is a valid value stored, since just now.
        Ok(SingleElementHandle(meta))
    }

    unsafe fn release<T: ?Sized + Pointee>(&mut self, _: Self::Handle<T>) {}

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        let pointer: NonNull<u8> = NonNull::from(&self.data).cast();

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

impl<S> Debug for SingleElement<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

impl<S> Default for SingleElement<S> {
    fn default() -> Self { Self::new() }
}


/// Handle of SingleElementStorage.
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

#[cfg(test)]
mod tests {

use super::*;

#[test]
fn new_unconditional_success() {
    SingleElement::<u8>::new();
}

#[test]
fn create_success() {
    let mut storage = SingleElement::<[u8; 2]>::new();
    storage.create(1u8).unwrap();
}

#[test]
fn create_insufficient_size() {
    let mut storage = SingleElement::<u8>::new();
    storage.create([1u8, 2, 3]).unwrap_err();
}

#[test]
fn create_insufficient_alignment() {
    let mut storage = SingleElement::<[u8; 32]>::new();
    storage.create([1u32]).unwrap_err();
}

#[test]
fn coerce() {
    let mut storage = SingleElement::<[u8; 32]>::new();

    let handle = storage.create([1u8, 2u8]).unwrap();

    //  Safety:
    //  -   `handle` is valid.
    let handle = unsafe { storage.coerce::<[u8], _>(handle) };

    //  Safety:
    //  -   `handle` is valid.
    unsafe { storage.destroy(handle) };
}

} // mod tests
