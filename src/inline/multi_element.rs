//! Inline implementation of MultiElementStorage.

use core::{fmt::{self, Debug}, marker::Unsize, mem::MaybeUninit, ptr::{self, NonNull}};

use rfc2580::{self, Pointee};

use crate::{traits::{Element, MultiElementStorage}, utils};

/// Generic inline MultiElementStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct MultiElement<S, const N: usize> {
    next: usize,
    data: [Overlay<S>; N],
}

impl<S, const N: usize> MultiElement<S, N> {
    /// Creates an instance.
    pub fn new() -> Self {
        unsafe { Self::default() }
    }
}

impl<S, const N: usize> MultiElementStorage for MultiElement<S, N> {
    type Handle<T: ?Sized + Pointee> = MultiElementHandle<T>;

    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        if self.next == INVALID_NEXT || utils::validate_layout::<T, S>().is_err() {
            return Err(value);
        }

        let meta = rfc2580::into_non_null_parts(NonNull::from(&value)).0;

        //  Pop slot from linked list.
        let handle = MultiElementHandle(self.next, meta);

        //  Safety:
        //  -   `handle.0` is within bounds by invariant.
        let slot = unsafe { self.data.get_unchecked_mut(handle.0) };

        //  Safety:
        //  -   By invariant, if pointed it contains the "next" field.
        self.next = unsafe { slot.next };

        //  Safety:
        //  -   `value` is assumed to fit within `S`.
        unsafe { ptr::write(slot.data.as_mut_ptr() as *mut T, value) };

        Ok(handle)
    }

    unsafe fn forget<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is assumed to be within range, as part of being valid.
        let slot = self.data.get_unchecked_mut(handle.0);

        //  Place slot back in linked-list.
        slot.next = self.next;
        self.next = handle.0;
    }

    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> Element<T> {
        //  Safety:
        //  -   `handle` is assumed to be within range.
        let slot = self.data.get_unchecked(handle.0);
    
        let pointer: NonNull<u8> = NonNull::from(&slot.data).cast();

        //  Safety:
        //  -   `handle` is assumed to point to a valid element.
        rfc2580::from_non_null_parts(handle.1, pointer)
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U> {
        //  Safety:
        //  -   `handle` is assumed to point to a valid element.
        let element = self.get(handle);

        let meta = rfc2580::into_raw_parts(element.as_ptr() as *mut U).0;

        MultiElementHandle(handle.0, meta)
    }
}

impl<S, const N: usize> Debug for MultiElement<S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "MultiElement{{ next: ")?;
        display_next(f, self.next)?;

        let mut next = self.next;
        while next != INVALID_NEXT {
            write!(f, " -> ")?;

            //  Safety:
            //  -   `next` is assumed to be within range.
            let slot = unsafe { self.data.get_unchecked(next) };

            //  Safety:
            //  -   `slot` contains `next` if pointed to.
            next = unsafe { slot.next };

            display_next(f, next)?;
        }

        write!(f, " }}")
    }
}

impl<S, const N: usize> Default for MultiElement<S, N> {
    fn default() -> Self { Self::new() }
}

/// The Handle for MultiElements.
pub struct MultiElementHandle<T: ?Sized + Pointee>(usize, T::MetaData);

impl<T: ?Sized + Pointee> Clone for MultiElementHandle<T> {
    fn clone(&self) -> Self { *self }
}

impl<T: ?Sized + Pointee> Copy for MultiElementHandle<T> {}

impl<T: ?Sized + Pointee> Debug for MultiElementHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "MultiElementHandle({})", self.0)
    }
}

//
//  Implementation
//

const INVALID_NEXT: usize = usize::MAX;

impl<S, const N: usize> MultiElement<S, N> {
    //  Creates a default instance.
    //
    //  #   Safety
    //
    //  Does not, in any way, validate that the storage is suitable for storing an instance of `T`.
    unsafe fn default() -> Self {
        let mut data: [Overlay<S>; N] = MaybeUninit::uninit().assume_init();

        if N == 0 {
            let next = INVALID_NEXT;
            return Self { next, data, };
        }

        //  Created linked-list of slots, using INVALID_NEXT as sentinel.
        let last = N - 1;

        for index in 0..last {
            data[index].next = index + 1;
        }

        data[last].next = INVALID_NEXT;

        Self { next: 0, data, }
    }
}

union Overlay<S> {
    next: usize,
    data: MaybeUninit<S>,
}

impl<S> Default for Overlay<S> {
    fn default() -> Self { Overlay { next: 0 } }
}

fn display_next(f: &mut fmt::Formatter<'_>, n: usize) -> Result<(), fmt::Error> {
    if n == INVALID_NEXT {
        write!(f, "null")
    } else {
        write!(f, "{}", n)
    }
}

#[cfg(test)]
mod tests {

use super::*;

#[test]
fn new_unconditional_success() {
    MultiElement::<u8, 5>::new();
}

#[test]
fn create_success() {
    let mut storage = MultiElement::<u8, 5>::new();
    let handle = storage.create(4u8).unwrap();
    let element = unsafe { storage.get(handle) };

    assert_eq!(4, unsafe { *element.as_ref() });
}

#[test]
fn create_insufficient_alignment() {
    let mut storage = MultiElement::<[u8; 4], 5>::new();
    storage.create([1u16, 2]).unwrap_err();
}

#[test]
fn create_insufficient_size() {
    let mut storage = MultiElement::<[u8; 2], 5>::new();
    storage.create([1u8, 2, 3]).unwrap_err();
}

#[test]
fn create_insufficient_capacity() {
    let victim = "Hello, World".to_string();
    let mut storage = MultiElement::<String, 1>::new();

    for _ in 0..2 {
        let handle = storage.create(victim.clone()).unwrap();
        let element = unsafe { storage.get(handle) };
        assert_eq!(&victim, unsafe { element.as_ref() });

        storage.create(victim.clone()).unwrap_err();
        unsafe { storage.destroy(handle) };
    }
}

#[test]
fn get_accross_moves() {
    let mut storage = MultiElement::<u8, 5>::new();

    let h1 = storage.create(1u8).unwrap();
    let h2 = storage.create(2u8).unwrap();
    let h3 = storage.create(3u8).unwrap();

    let storage = storage;

    assert_eq!(1, unsafe { *storage.get(h1).as_ptr() });
    assert_eq!(2, unsafe { *storage.get(h2).as_ptr() });
    assert_eq!(3, unsafe { *storage.get(h3).as_ptr() });
}

#[test]
fn coerce_unsize() {
    let mut storage = MultiElement::<[u8; 2], 5>::new();
    let handle = storage.create([1, 2]).unwrap();

    let handle = unsafe { storage.coerce::<[u8], _>(handle) };
    let element = unsafe { storage.get(handle) };

    assert_eq!(&[1, 2], unsafe { element.as_ref() });
}

}
