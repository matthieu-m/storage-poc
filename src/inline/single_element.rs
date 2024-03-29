//! Simple implementation of `SingleElementStorage<T>`.

use core::{alloc::AllocError, fmt::{self, Debug}, marker::Unsize, mem::MaybeUninit, ptr::{NonNull, Pointee}};

use crate::{traits::{ElementStorage, SingleElementStorage}, utils};

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

impl<S> ElementStorage for SingleElement<S> {
    type Handle<T: ?Sized + Pointee> = SingleElementHandle<T>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, _: Self::Handle<T>) {}

    unsafe fn resolve<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        let pointer: NonNull<()> = NonNull::from(&self.data).cast();

        NonNull::from_raw_parts(pointer, handle.0)
    }

    unsafe fn resolve_mut<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) -> NonNull<T> {
        let pointer: NonNull<()> = NonNull::from(&mut self.data).cast();

        NonNull::from_raw_parts(pointer, handle.0)
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.resolve(handle);

        let meta = (element.as_ptr() as *mut U).to_raw_parts().1;

        SingleElementHandle(meta)
    }
}

impl<S> SingleElementStorage for SingleElement<S> {
    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::Metadata) -> Result<Self::Handle<T>, AllocError> {
        let _ = utils::validate_layout::<T, S>(meta)?;

        Ok(SingleElementHandle(meta))
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
pub struct SingleElementHandle<T: ?Sized + Pointee>(T::Metadata);

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
