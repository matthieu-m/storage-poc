//! Simple implementation of `MultiElementStorage`.

use core::{fmt::{self, Debug}, marker::Unsize, ptr::NonNull};

use rfc2580::Pointee;

use crate::traits::{ElementStorage, MultiElementStorage};

/// MultiElement is a fallback implementation of 2 MultiElementStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct MultiElement<F, S> {
    first: F,
    second: S,
}

impl<F, S> MultiElement<F, S> {
    /// Creates an instance of MultiElement.
    pub fn new(first: F, second: S) -> Self { Self { first, second } }
}

impl<F, S> ElementStorage for MultiElement<F, S>
    where
        F: ElementStorage,
        S: ElementStorage,
{
    type Handle<T: ?Sized + Pointee> = MultiElementHandle<F::Handle<T>, S::Handle<T>>;

    unsafe fn release<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        use MultiElementHandle::*;

        match handle {
            First(first) => self.first.release(first),
            Second(second) => self.second.release(second),
        }
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        use MultiElementHandle::*;

        match handle {
            First(first) => self.first.get(first),
            Second(second) => self.second.get(second),
        }
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        use MultiElementHandle::*;

        match handle {
            First(first) => First(self.first.coerce(first)),
            Second(second) => Second(self.second.coerce(second)),
        }
    }
}

impl<F, S> MultiElementStorage for MultiElement<F, S>
    where
        F: MultiElementStorage,
        S: MultiElementStorage,
{
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T>  {
        use MultiElementHandle::*;

        match self.first.create(value) {
            Ok(handle) => Ok(First(handle)),
            Err(value) => self.second.create(value).map(|handle| Second(handle)),
        }
    }
}

impl<F: Default, S: Default> Default for MultiElement<F, S> {
    fn default() -> Self { Self::new(F::default(), S::default()) }
}

impl<F, S> Debug for MultiElement<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "MultiElement")
    }
}

/// MultiElementHandle.
#[derive(Clone, Copy)]
pub enum MultiElementHandle<F: Copy, S: Copy> {
    /// Handle of first storage.
    First(F),
    /// Handle of second storage.
    Second(S),
}
