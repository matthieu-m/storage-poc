//! Simple implementation of `SingleElementStorage<T>`.

use core::{fmt::{self, Debug}, marker::{PhantomData, Unsize}, mem::{self, MaybeUninit}, ptr::{self, NonNull}};

use rfc2580::{self, Pointee};

use crate::{traits::{Element, SingleElementStorage}, utils};

/// Generic inline SingleElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleElement<T: ?Sized + Pointee, S> {
    meta: MaybeUninit<T::MetaData>,
    data: MaybeUninit<S>,
    _marker: PhantomData<T>,
}

impl<T, S> SingleElement<T, S> {
    /// Attempts to create an instance of SingleElement.
    ///
    /// Fails if `Layout<T>` cannot be accomidated by `Layout<Storage>`.
    pub fn new() -> Option<Self> {
        if let Ok(_) = utils::validate_layout::<T, S>() {
            //  Safety:
            //  -   `T` was validated to fit within `S`.
            Some(unsafe { Self::default() })
        } else {
            None
        }
    }
}

impl<T: ?Sized, S> SingleElement<T, S> {
    /// Creates an instance for dynamically sized type.
    ///
    /// Validation of size and alignment are delayed to the initialization of the storage.
    pub fn new_unsize() -> Self {
        assert!(mem::size_of::<*const T>() > mem::size_of::<*const u8>());

        //  Safety:
        //  -   `create_unsize` which will verify the suitability of `S`'s layout.
        unsafe { Self::default() }
    }
}

impl<T: ?Sized + Pointee, S> SingleElementStorage<T> for SingleElement<T, S> {
    fn create(&mut self, value: T) -> Result<Element<T>, T>
        where
            T: Sized,
    {
        let meta = rfc2580::into_raw_parts(&value as *const T).0;

        //  Safety:
        //  -   The layout of `S` has been validated as part of `new`.
        unsafe { self.write(meta, value) };

        //  Safety:
        //  -   There is a valid value stored, since just now.
        Ok(unsafe { self.get() })
    }

    fn create_unsize<V: Unsize<T>>(&mut self, value: V) -> Result<Element<T>, V> {
        if let Ok(_) = utils::validate_layout::<V, S>() {
            let meta = rfc2580::into_raw_parts(&value as &T as *const T).0;

            //  Safety:
            //  -   The layout of `S` has been validated, just above.
            unsafe { self.write(meta, value) };

            //  Safety:
            //  -   There is a valid value stored, since just now.
            Ok(unsafe { self.get() })
        } else {
            Err(value)
        }
    }

    unsafe fn destroy(&mut self) {
        //  Safety:
        //  -   The storage is assumed to be initialized.
        let element = self.get().as_ptr();

        //  Safety:
        //  -   `element` is exclusively accessible.
        //  -   `element` is suitably aligned.
        //  -   `element` points to a valid value.
        ptr::drop_in_place(element);
    }

    unsafe fn get(&self) -> Element<T> {
        //  Safety:
        //  -   `self.meta` is assumed to be initialized.
        let meta = *self.meta.as_ptr();

        let data = self.data.as_ptr() as *const u8;

        let pointer = rfc2580::from_raw_parts(meta, data);

        //  Safety:
        //  -   `pointer` is not null, as `data` is not null.
        NonNull::new_unchecked(pointer as *mut T)
    }
}

impl<T: ?Sized, S> Debug for SingleElement<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

//
//  Implementation
//
impl<T: ?Sized + Pointee, S> SingleElement<T, S> {
    //  Creates a default instance.
    //
    //  #   Safety
    //
    //  Does not, in any way, validate that the storage is suitable for storing an instance of `T`.
    unsafe fn default() -> Self {
        Self { meta: MaybeUninit::uninit(), data: MaybeUninit::uninit(), _marker: PhantomData }
    }

    //  Writes an instance of `V`, with the appropriate meta-data for `T`, in the storage.
    //
    //  Assumes that the storage is unoccupied, otherwise the current memory is overwritten, which may lead to resource
    //  leaks.
    //
    //  #   Safety
    //
    //  Assumes that the layout of storage has been validated to fit `V`.
    unsafe fn write<V>(&mut self, meta: T::MetaData, data: V) {
        debug_assert!(utils::validate_layout::<V, S>().is_ok());

        //  Safety:
        //  -   `self.meta` exactly matches `meta`.
        ptr::write(self.meta.as_mut_ptr(), meta);

        //  Safety:
        //  -   `pointer` points to an appropriate (layout-wise) memory area.
        ptr::write(self.data.as_mut_ptr() as *mut V, data);
    }
}

#[cfg(test)]
mod tests {

use super::*;

#[test]
fn new_success() {
    SingleElement::<u8, u8>::new().unwrap();
}

#[test]
fn new_insufficient_size() {
    SingleElement::<[u8; 2], u8>::new().unwrap_none();
}

#[test]
fn new_insufficient_alignment() {
    SingleElement::<u32, [u8; 4]>::new().unwrap_none();
}

#[test]
fn new_unsize_unconditional_success() {
    SingleElement::<[u32], u8>::new_unsize();
    SingleElement::<dyn Debug, u8>::new_unsize();
}

#[test]
fn create_success() {
    let mut storage = SingleElement::<u8, [u8; 2]>::new().unwrap();
    storage.create(1u8).unwrap();
}

#[test]
fn create_unsize_success() {
    let mut storage = SingleElement::<[u8], u32>::new_unsize();
    storage.create_unsize([1u8, 2, 3]).unwrap();
}

#[test]
fn create_unsize_insufficient_size() {
    let mut storage = SingleElement::<[u8], u8>::new_unsize();
    storage.create_unsize([1u8, 2, 3]).unwrap_err();
}

#[test]
fn create_unsize_insufficient_alignment() {
    let mut storage = SingleElement::<[u32], [u8; 32]>::new_unsize();
    storage.create_unsize([1u32]).unwrap_err();
}

}
