//! Alternative implementation of `SingleRangeStorage`.

use core::{alloc::AllocError, cmp, fmt::{self, Debug}, hint, mem::{self, MaybeUninit}, ptr::{self, NonNull}};

use crate::traits::{Capacity, RangeStorage, SingleRangeStorage};

use super::{Builder, Inner};

/// SingleRange is a composite of 2 SingleRangeStorage.
///
/// It will first attempt to allocate from the first storage if possible, and otherwise use the second storage if
/// necessary.
pub struct SingleRange<F, S, FB, SB>(Inner<F, S, FB, SB>);

impl<F, S, FB, SB> SingleRange<F, S, FB, SB> {
    /// Creates an instance containing the First alternative.
    pub fn first(first: F, second_builder: SB) -> Self { Self(Inner::first(first, second_builder)) }

    /// Creates an instance containing the Second alternative.
    pub fn second(second: S, first_builder: FB) -> Self { Self(Inner::second(second, first_builder)) }
}

impl<F, S, FB, SB> RangeStorage for SingleRange<F, S, FB, SB>
    where
        F: SingleRangeStorage,
        S: SingleRangeStorage,
        FB: Builder<F>,
        SB: Builder<S>,
{
    type Handle<T> = SingleRangeHandle<F::Handle<T>, S::Handle<T>>;

    type Capacity = S::Capacity;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        match &self.0 {
            Inner::First(ref first) => into_second::<F, S>(first.maximum_capacity::<T>()),
            Inner::Second(ref second) => second.maximum_capacity::<T>(),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>) {
        match &mut self.0 {
            Inner::First(ref mut first) => first.deallocate(handle.first),
            Inner::Second(ref mut second) => second.deallocate(handle.second),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn get<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        match &self.0 {
            Inner::First(ref first) => first.get(handle.first),
            Inner::Second(ref second) => second.get(handle.second),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn try_grow<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        match &mut self.0 {
            Inner::First(ref mut first) => {
                let grow = into_first::<F, S>(new_capacity)
                    .and_then(|new_capacity| first.try_grow(handle.first, new_capacity));

                match grow {
                    Ok(first) => Ok(SingleRangeHandle { first }),
                    Err(_) => {
                        if let Inner::First(first) = mem::replace(&mut self.0, Inner::Poisoned) {
                            let (second, result) = first.transform(|first: &mut F, second: &mut S| {
                                let new_handle = second.allocate(new_capacity)?;
                                transfer(first.get(handle.first), second.get(new_handle));
                                Ok(SingleRangeHandle { second: new_handle })
                            });
                            self.0 = Inner::Second(second);
                            return result;
                        }
                        //  Safety:
                        //  -   self.0 was First before invoking replace, hence replace returns First.
                        hint::unreachable_unchecked();
                    },
                }
            },
            Inner::Second(ref mut second) =>
                second.try_grow(handle.second, new_capacity).map(|second| SingleRangeHandle{ second }),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }

    unsafe fn try_shrink<T>(&mut self, handle: Self::Handle<T>, new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        match &mut self.0 {
            Inner::First(ref mut first) =>
                first.try_shrink(handle.first, into_first::<F, S>(new_capacity)?)
                    .map(|first| SingleRangeHandle{ first }),

            Inner::Second(ref mut second) => {
                let shrink = second.try_shrink(handle.second, new_capacity);

                match shrink {
                    Ok(second) => Ok(SingleRangeHandle{ second }),
                    Err(_) => {
                        let new_capacity = into_first::<F, S>(new_capacity)?;

                        if let Inner::Second(second) = mem::replace(&mut self.0, Inner::Poisoned) {
                            let (first, result) = second.transform(|second: &mut S, first: &mut F| {
                                let new_handle = first.allocate(new_capacity)?;
                                transfer(second.get(handle.second), first.get(new_handle));
                                Ok(SingleRangeHandle { first: new_handle })
                            });
                            self.0 = Inner::First(first);
                            return result;
                        }
                        //  Safety:
                        //  -   self.0 was Second before invoking replace, hence replace returns Second.
                        hint::unreachable_unchecked();
                    },
                }
            },
            Inner::Poisoned => panic!("Poisoned"),
        }
    }
}

impl<F, S, FB, SB> SingleRangeStorage for SingleRange<F, S, FB, SB>
    where
        F: SingleRangeStorage,
        S: SingleRangeStorage,
        FB: Builder<F>,
        SB: Builder<S>,
{
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        match &mut self.0 {
            Inner::First(ref mut first) => {
                let handle = into_first::<F, S>(capacity)
                    .and_then(|capacity| first.allocate(capacity));

                match handle {
                    Ok(first) => Ok(SingleRangeHandle{ first }),
                    Err(_) => {
                        if let Inner::First(first) = mem::replace(&mut self.0, Inner::Poisoned) {
                            let (second, result) = first.transform(|_, second: &mut S| {
                                second.allocate(capacity).map(|second| SingleRangeHandle { second })
                            });
                            self.0 = Inner::Second(second);
                            return result;
                        }
                        //  Safety:
                        //  -   self.0 was First before invoking replace, hence replace returns First.
                        unsafe { hint::unreachable_unchecked() };
                    }
                }
            },
            Inner::Second(ref mut second) =>
                second.allocate(capacity).map(|second| SingleRangeHandle{ second }),
            Inner::Poisoned => panic!("Poisoned"),
        }
    }
}

impl<F, S, FB, SB> Debug for SingleRange<F, S, FB, SB> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "SingleRange")
    }
}

impl<F: Default, S, FB, SB: Default> Default for SingleRange<F, S, FB, SB> {
    fn default() -> Self { Self(Inner::default()) }
}

/// SingleRangeHandle, an alternative between 2 handles.
#[derive(Clone, Copy)]
pub union SingleRangeHandle<F: Copy, S: Copy> {
    first: F,
    second: S,
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

fn into_second<F: RangeStorage, S: RangeStorage>(capacity: F::Capacity) -> S::Capacity {
    S::Capacity::from_usize(capacity.into_usize())
        .expect("Second to have a greater capacity type than First")
}

unsafe fn transfer<T>(from: NonNull<[MaybeUninit<T>]>, mut to: NonNull<[MaybeUninit<T>]>) {
    let from = from.as_ref();
    let to = to.as_mut();

    ptr::copy_nonoverlapping(from.as_ptr(), to.as_mut_ptr(), cmp::min(from.len(), to.len()));
}
