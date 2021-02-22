//! Simple implementation of `SingleRangeStorage`.

use core::{alloc::AllocError, cmp, fmt::{self, Debug}, marker::PhantomData, mem::{self, MaybeUninit}, ptr::NonNull};

use crate::{traits::{Capacity, RangeStorage, SingleRangeStorage}, utils};

/// Generic inline SingleRangeStorage.
///
/// `S` is the underlying storage, used to specify the size and alignment.
pub struct SingleRange<C, S, const N: usize> {
    data: [MaybeUninit<S>; N],
    _marker: PhantomData<fn(C) -> C>,
}

impl<C, S, const N: usize> SingleRange<C, S, N> {
    /// Creates an instance of SingleRange.
    pub fn new() -> Self { Self { data: MaybeUninit::uninit_array(), _marker: PhantomData, } }
}

impl<C: Capacity, S, const N: usize> RangeStorage for SingleRange<C, S, N> {
    type Handle<T> = SingleRangeHandle<T>;

    type Capacity = C;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        assert!(mem::size_of::<S>().checked_mul(N).is_some());

        //  The maximum capacity cannot exceed what can fit in an `isize`.
        let capacity = cmp::min(C::max().into_usize(), N);

        C::from_usize(mem::size_of::<S>() * capacity / mem::size_of::<T>())
            .or_else(|| C::from_usize(capacity))
            .expect("Cannot fail, since capacity <= C::max()")
    }

    unsafe fn deallocate<T>(&mut self, _handle: Self::Handle<T>) {}

    unsafe fn get<T>(&self, _handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        let pointer: NonNull<MaybeUninit<T>> = NonNull::from(&self.data).cast();

        NonNull::slice_from_raw_parts(pointer, N)
    }
}

impl<C: Capacity, S, const N: usize> SingleRangeStorage for SingleRange<C, S, N> {
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        utils::validate_array_layout::<T, [MaybeUninit<S>; N]>(capacity.into_usize())
            .map(|_| SingleRangeHandle::new())
            .map_err(|_| AllocError)
    }
}

impl<C, S, const N: usize> Debug for SingleRange<C, S, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRange")
    }
}

impl<C, S, const N: usize> Default for SingleRange<C, S, N> {
    fn default() -> Self { Self::new() }
}


/// Handle of SingleRange.
pub struct SingleRangeHandle<T>(PhantomData<fn(T)->T>);

impl<T> SingleRangeHandle<T> {
    fn new() -> Self { Self(PhantomData) }
}

impl<T> Clone for SingleRangeHandle<T> {
    fn clone(&self) -> Self { *self }
}

impl<T> Copy for SingleRangeHandle<T> {}

impl<T> Debug for SingleRangeHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRangeHandle")
    }
}

#[cfg(test)]
mod tests {

use super::*;

#[test]
fn new_unconditional_success() {
    SingleRange::<u8, u8, 42>::new();
}

#[test]
fn allocate_success() {
    let mut storage = SingleRange::<u8, u8, 42>::new();
    storage.allocate::<u8>(2).unwrap();
}

#[test]
fn allocate_insufficient_size() {
    let mut storage = SingleRange::<u8, u8, 2>::new();
    storage.allocate::<u8>(3).unwrap_err();
}

#[test]
fn allocate_insufficient_alignment() {
    let mut storage = SingleRange::<u8, u8, 42>::new();
    storage.allocate::<u32>(1).unwrap_err();
}

} // mod tests
