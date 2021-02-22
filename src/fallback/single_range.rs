//! Alternative implementation of `SingleRangeStorage`.

use core::{alloc::AllocError, cmp, fmt::{self, Debug}, mem::MaybeUninit, ptr::{self, NonNull}};

use crate::traits::{Capacity, RangeStorage, SingleRangeStorage};

/// SingleRange is a composite of 2 SingleRangeStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct SingleRange<F, S> {
    first: F,
    second: S,
}

impl<F, S> SingleRange<F, S> {
    /// Creates an instance.
    pub fn new(first: F, second: S) -> Self { Self{ first, second, } }
}

impl<F, S> RangeStorage for SingleRange<F, S>
    where
        F: SingleRangeStorage,
        S: SingleRangeStorage,
{
    type Handle<T> = SingleRangeHandle<F::Handle<T>, S::Handle<T>>;

    type Capacity = S::Capacity;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        let first = self.first.maximum_capacity::<T>();
        let second = self.second.maximum_capacity::<T>();

        let result = first.into_usize().saturating_add(second.into_usize());

        if let Some(result) = S::Capacity::from_usize(result) {
            result
        } else {
            second
        }
    }

    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>) {
        use SingleRangeHandle::*;
    
        match handle {
            First(first) => self.first.deallocate(first),
            Second(second) => self.second.deallocate(second),
        }
    }

    unsafe fn get<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        use SingleRangeHandle::*;
    
        match handle {
            First(first) => self.first.get(first),
            Second(second) => self.second.get(second),
        }
    }

    unsafe fn try_grow<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        use SingleRangeHandle::*;

        match handle {
            First(first) => {
                let first_capacity = into_first::<F, S>(new_capacity);

                match first_capacity.and_then(|new_capacity| self.first.try_grow(first, new_capacity)) {
                    Ok(handle) => Ok(First(handle)),
                    Err(_) => {
                        let second = self.second.allocate(new_capacity)?;
                        transfer(self.first.get(first), self.second.get(second));
                        self.first.deallocate(first);
                        Ok(Second(second))
                    }
                }
            },
            Second(second) =>
                self.second.try_grow(second, new_capacity)
                    .map(|handle| Second(handle)),
        }
    }

    unsafe fn try_shrink<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        use SingleRangeHandle::*;

        let first_capacity = into_first::<F, S>(new_capacity);

        match handle {
            First(first) =>
                self.first.try_shrink(first, first_capacity?)
                    .map(|handle| First(handle)),
            Second(second) =>
                if let Ok(first) = first_capacity.and_then(|cap| self.first.allocate(cap)) {
                    transfer(self.second.get(second), self.first.get(first));
                    self.second.deallocate(second);
                    Ok(First(first))
                } else {
                    self.second.try_shrink(second, new_capacity)
                        .map(|handle| Second(handle))
                },
        }
    }
}

impl<F, S> SingleRangeStorage for SingleRange<F, S>
    where
        F: SingleRangeStorage,
        S: SingleRangeStorage,
{
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        use SingleRangeHandle::*;
    
        let first_capacity = into_first::<F, S>(capacity);

        if let Ok(first) = first_capacity.and_then(|cap| self.first.allocate(cap)) {
            Ok(First(first))
        } else {
            self.second.allocate(capacity)
                .map(|handle| Second(handle))
        }
    }
}

impl<F, S> Debug for SingleRange<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRange")
    }
}

impl<F: Default, S: Default> Default for SingleRange<F, S> {
    fn default() -> Self { Self::new(F::default(), S::default()) }
}

/// SingleRangeHandle, an alternative between 2 handles.
#[derive(Clone, Copy)]
pub enum SingleRangeHandle<F: Copy, S: Copy> {
    /// First storage handle.
    First(F),
    /// Second storage handle.
    Second(S),
}

impl<F: Copy, S: Copy> Debug for SingleRangeHandle<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRangeHandle")
    }
}

//
//  Implementation
//

fn into_first<F: RangeStorage, S: RangeStorage>(capacity: S::Capacity) -> Result<F::Capacity, AllocError> {
    F::Capacity::from_usize(capacity.into_usize())
        .ok_or(AllocError)
}

unsafe fn transfer<T>(from: NonNull<[MaybeUninit<T>]>, mut to: NonNull<[MaybeUninit<T>]>) {
    let from = from.as_ref();
    let to = to.as_mut();

    ptr::copy_nonoverlapping(from.as_ptr(), to.as_mut_ptr(), cmp::min(from.len(), to.len()));
}
