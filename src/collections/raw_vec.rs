//! Proof-of-Concept implementation of a Vec parameterized by a Storage.

use core::{cmp, fmt::{self, Debug}, mem::MaybeUninit, ops::{Deref, DerefMut}, ptr};

use crate::traits::{Capacity, SingleRangeStorage};

/// A PoC Vec.
pub struct RawVec<T, S: SingleRangeStorage> {
    len: S::Capacity,
    data: S::Handle<T>,
    storage: S,
}

impl<T, S: SingleRangeStorage> RawVec<T, S> {
    /// Creates a new instance.
    pub fn new(mut storage: S) -> Self {
        let zero = Self::into_capacity(0);

        let len = zero;
        let data = storage.acquire(zero).expect("Zero-capacity allocation should always succeed");

        Self { len, data, storage, }
    }

    /// Returns whether `self` is empty, or not.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Returns the number of elements in `self`.
    pub fn len(&self) -> usize { self.len.into_usize() }

    /// Clears `self`, destroying all elements and resetting its length to 0.
    pub fn clear(&mut self) {
        while let Some(_) = self.pop() {}
    }

    /// Attempts to push a new element at the back.
    pub fn try_push(&mut self, e: T) -> Result<(), T> {
        let len = self.len();

        let slice = self.raw_slice_mut();

        if len >= slice.len() {
            return self.try_push_grow(e);
        }

        //  Safety:
        //  -   `len < slice.len()`.
        let slot = unsafe { slice.get_unchecked_mut(len) };

        slot.write(e);

        self.len = Self::into_capacity(len + 1);

        Ok(())
    }

    /// Pushes an element at the back.
    ///
    /// #   Panics
    ///
    /// If cannot grow.
    pub fn push(&mut self, e: T) {
        self.try_push(e)
            .map_err(|_| ())
            .expect("Sufficient capacity");
    }

    /// Pops the back element, if any.
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let len = self.len();

        let slice = self.raw_slice_mut();

        //  Safety:
        //  -   `len > 0`, as `self` is not empty.
        //  -   As an invariant, `slice.len() >= self.len`.
        let slot = unsafe { slice.get_unchecked(len - 1) };

        //  Safety:
        //  -   `slot` is valid for reads, properly aligned.
        //  -   `slot` contains an initialized value of `T`.
        let result = unsafe { ptr::read(slot.as_ptr()) };

        self.len = Self::into_capacity(len - 1);

        Some(result)
    }
}

impl<T: Debug, S: SingleRangeStorage> Debug for RawVec<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let slice: &[T] = &*self;
        write!(f, "{:?}", slice)
    }
}

impl<T, S: Default + SingleRangeStorage> Default for RawVec<T, S> {
    fn default() -> Self { RawVec::new(S::default()) }
}

impl<T, S: SingleRangeStorage> Deref for RawVec<T, S> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let len = self.len();
        let slice = self.raw_slice();

        //  Safety:
        //  -   Invariant: `slice.len() >= self.len()`.
        let slice = unsafe { slice.get_unchecked(0..len) };

        //  Safety:
        //  -   Invariant, `self.raw_slice()[0..len]` are initialized.
        unsafe { MaybeUninit::slice_assume_init_ref(slice) }
    }
}

impl<T, S: SingleRangeStorage> DerefMut for RawVec<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len();
        let slice = self.raw_slice_mut();

        //  Safety:
        //  -   Invariant: `slice.len() >= self.len()`.
        let slice = unsafe { slice.get_unchecked_mut(0..len) };

        //  Safety:
        //  -   Invariant, `self.raw_slice()[0..len]` are initialized.
        unsafe { MaybeUninit::slice_assume_init_mut(slice) }
    }
}

impl<T, S: SingleRangeStorage> Drop for RawVec<T, S> {
    fn drop(&mut self) {
        self.clear();

        //  Safety:
        //  -   `self.data` is valid.
        unsafe { self.storage.release(self.data) };
    }
}

//
//  Implementation
//

impl<T, S: SingleRangeStorage> RawVec<T, S> {
    fn into_capacity(n: usize) -> S::Capacity {
        S::Capacity::from_usize(n).expect("n <= S::maximum_capacity()")
    }

    fn raw_slice(&self) -> &[MaybeUninit<T>] {
        //  Safety:
        //  -   `self.data` is valid and points to valid data.
        let range = unsafe { self.storage.get(self.data) };

        //  Safety:
        //  -   `range` points to valid data.
        //  -   The lifetime of the slice is actually that of `self.storage`.
        unsafe { &*range.as_ptr() }
    }

    fn raw_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        //  Safety:
        //  -   `self.data` is valid and points to valid data.
        let range = unsafe { self.storage.get(self.data) };

        //  Safety:
        //  -   `range` points to valid data.
        //  -   The lifetime of the slice is actually that of `self.storage`.
        unsafe { &mut *range.as_ptr() }
    }

    #[inline(never)]
    fn try_push_grow(&mut self, e: T) -> Result<(), T> {
        let len = self.len.into_usize();
        let new_cap = cmp::max(1, len * 2);

        //  Safety:
        //  -   `self.data` is a valid handle pointing to valid data.
        self.data = match unsafe { self.storage.try_grow(self.data, Self::into_capacity(new_cap)) } {
            Ok(handle) => handle,
            Err(_) => return Err(e),
        };

        let slice = self.raw_slice_mut();

        //  Safety:
        //  -   `len < slice.len()`.
        let slot = unsafe { slice.get_unchecked_mut(len) };

        slot.write(e);

        self.len = Self::into_capacity(len + 1);

        Ok(())
    }
}

#[cfg(test)]
mod test_inline {

use core::mem;

use crate::inline::SingleRange;

use super::*;

#[test]
fn size() {
    type Storage = SingleRange<u8, u8, 31>;
    type Vec = RawVec<u8, Storage>;

    assert_eq!(32, mem::size_of::<Vec>());
}

#[test]
fn smoke_test() {
    type Storage = SingleRange<u8, u8, 31>;
    type Vec = RawVec<u8, Storage>;

    let mut vec = Vec::default();

    for i in 0..31 {
        vec.push(i);
    }

    assert_eq!(Some(&2), vec.get(2));

    assert_eq!(
        "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30]",
        format!("{:?}", vec)
    );
}

#[test]
fn try_push_failure() {
    type Storage = SingleRange<u8, u8, 1>;
    type Vec = RawVec<u8, Storage>;

    let mut vec = Vec::default();
    vec.push(0);

    assert_eq!(Err(42), vec.try_push(42));
}

} // mod test_inline

#[cfg(all(test, feature = "alloc"))]
mod test_allocator {

use core::mem;
use alloc::alloc::Global;

use crate::allocator::SingleRange;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn size() {
    type Storage = SingleRange<Global>;
    type Vec = RawVec<u8, Storage>;

    assert_eq!(mem::size_of::<usize>() * 3, mem::size_of::<Vec>());
}

#[test]
fn smoke_test() {
    type Storage = SingleRange<SpyAllocator>;
    type Vec = RawVec<u8, Storage>;

    let allocator = SpyAllocator::default();

    let storage = SingleRange::new(allocator.clone());
    let mut vec = Vec::new(storage);

    assert_eq!(0, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    for i in 0..31 {
        vec.push(i);

        assert_eq!(allocator.allocated() - 1, allocator.deallocated());
    }

    assert_eq!(Some(&2), vec.get(2));

    assert_eq!(
        "[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30]",
        format!("{:?}", vec)
    );

    mem::drop(vec);

    assert_eq!(6, allocator.allocated());
    assert_eq!(6, allocator.deallocated());
}

#[test]
fn try_push_failure() {
    type Storage = SingleRange<NonAllocator>;
    type Vec = RawVec<u8, Storage>;

    let mut vec = Vec::default();

    assert_eq!(Err(42), vec.try_push(42));
}
    
} // mod test_allocator
