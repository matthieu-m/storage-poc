//! Fallback implementation of `SingleElementStorage`.

use core::{fmt::{self, Debug}, marker::Unsize, ptr::NonNull};

use rfc2580::Pointee;

use crate::traits::{ElementStorage, SingleElementStorage};

/// SingleElement is a fallback implementation of 2 SingleElementStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct SingleElement<F, S> {
    first: F,
    second: S,
}

impl<F, S> SingleElement<F, S> {
    /// Creates an instance.
    pub fn new(first: F, second: S) -> Self { Self{ first, second, } }
}

impl<F, S> ElementStorage for SingleElement<F, S>
    where
        F: SingleElementStorage,
        S: SingleElementStorage,
{
    type Handle<T: ?Sized + Pointee> = SingleElementHandle<F::Handle<T>, S::Handle<T>>;

    unsafe fn release<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        use SingleElementHandle::*;

        match handle {
            First(first) => self.first.release(first),
            Second(second) => self.second.release(second),
        }
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        use SingleElementHandle::*;

        match handle {
            First(first) => self.first.get(first),
            Second(second) => self.second.get(second),
        }
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        use SingleElementHandle::*;

        match handle {
            First(first) => First(self.first.coerce(first)),
            Second(second) => Second(self.second.coerce(second)),
        }
    }
}

impl<F, S> SingleElementStorage for SingleElement<F, S>
    where
        F: SingleElementStorage,
        S: SingleElementStorage,
{
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        use SingleElementHandle::*;

        match self.first.create(value) {
            Ok(handle) => Ok(First(handle)),
            Err(value) => self.second.create(value).map(|handle| Second(handle)),
        }
    }
}

impl<F, S> Debug for SingleElement<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElement")
    }
}

impl<F: Default, S: Default> Default for SingleElement<F, S> {
    fn default() -> Self { Self::new(F::default(), S::default()) }
}

/// SingleElementHandle, an alternative between 2 handles.
#[derive(Clone, Copy)]
pub enum SingleElementHandle<F: Copy, S: Copy> {
    /// First storage handle.
    First(F),
    /// Second storage handle.
    Second(S),
}

impl<F: Copy, S: Copy> Debug for SingleElementHandle<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleElementHandle")
    }
}
