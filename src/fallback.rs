//! Simple implementations of the fallback principle, allocating from one and fallbacking to another.
//!
//! This is an attempt at implementing a generic way to combining existing storages, to simplify implementating small
//! storages for example.
//!
//! It is simpler than alternative, however is heavier weight.

use core::{
    alloc::AllocError,
    cmp,
    fmt::{self, Debug},
    marker::Unsize,
    mem::MaybeUninit,
    ptr::{self, NonNull, Pointee},
};

use crate::traits::{
    Capacity, ElementStorage, MultiElementStorage, RangeStorage, SingleElementStorage,
    SingleRangeStorage,
};

/// An allocator that implements ElementStorage, SingleElementStorage, MultiElementStorage,
/// RangeStorage, and SingleRangeStorage, depending on what the supplied allocators implement.
#[derive(Default)]
pub struct Fallback<P, S> {
    /// The primary allocator.
    pub primary: P,
    /// The secondary allocator.
    pub secondary: S,
}

/// The handle used by the [`Fallback`] allocator.
#[derive(Clone, Copy)]
pub enum FallbackHandle<P, S> {
    /// Handle of primary storage.
    Primary(P),
    /// Handle of secondary storage.
    Secondary(S),
}

use FallbackHandle::*;

impl<F, S> ElementStorage for Fallback<F, S>
where
    F: ElementStorage,
    S: ElementStorage,
{
    type Handle<T: ?Sized + Pointee> = FallbackHandle<F::Handle<T>, S::Handle<T>>;

    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        match handle {
            Primary(first) => self.primary.deallocate(first),
            Secondary(second) => self.secondary.deallocate(second),
        }
    }

    unsafe fn resolve<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T> {
        match handle {
            Primary(first) => self.primary.resolve(first),
            Secondary(second) => self.secondary.resolve(second),
        }
    }

    unsafe fn resolve_mut<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) -> NonNull<T> {
        match handle {
            Primary(first) => self.primary.resolve_mut(first),
            Secondary(second) => self.secondary.resolve_mut(second),
        }
    }

    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(
        &self,
        handle: Self::Handle<T>,
    ) -> Self::Handle<U> {
        match handle {
            Primary(first) => Primary(self.primary.coerce(first)),
            Secondary(second) => Secondary(self.secondary.coerce(second)),
        }
    }
}

impl<F, S> SingleElementStorage for Fallback<F, S>
where
    F: SingleElementStorage,
    S: SingleElementStorage,
{
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        match self.primary.create(value) {
            Ok(handle) => Ok(Primary(handle)),
            Err(value) => self.secondary.create(value).map(|handle| Secondary(handle)),
        }
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::Metadata,
    ) -> Result<Self::Handle<T>, AllocError> {
        self.primary
            .allocate::<T>(meta)
            .map(|handle| Primary(handle))
            .or_else(|_| {
                self.secondary
                    .allocate::<T>(meta)
                    .map(|handle| Secondary(handle))
            })
    }
}

impl<F, S> MultiElementStorage for Fallback<F, S>
where
    F: MultiElementStorage,
    S: MultiElementStorage,
{
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        match self.primary.create(value) {
            Ok(handle) => Ok(Primary(handle)),
            Err(value) => self.secondary.create(value).map(|handle| Secondary(handle)),
        }
    }

    fn allocate<T: ?Sized + Pointee>(
        &mut self,
        meta: T::Metadata,
    ) -> Result<Self::Handle<T>, AllocError> {
        self.primary
            .allocate::<T>(meta)
            .map(|handle| Primary(handle))
            .or_else(|_| {
                self.secondary
                    .allocate::<T>(meta)
                    .map(|handle| Secondary(handle))
            })
    }
}

impl<F, S> RangeStorage for Fallback<F, S>
where
    F: SingleRangeStorage,
    S: SingleRangeStorage,
{
    type Handle<T> = FallbackHandle<F::Handle<T>, S::Handle<T>>;

    type Capacity = S::Capacity;

    fn maximum_capacity<T>(&self) -> Self::Capacity {
        let first = self.primary.maximum_capacity::<T>();
        let second = self.secondary.maximum_capacity::<T>();

        let result = first.into_usize().saturating_add(second.into_usize());

        if let Some(result) = S::Capacity::from_usize(result) {
            result
        } else {
            second
        }
    }

    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>) {
        match handle {
            Primary(first) => self.primary.deallocate(first),
            Secondary(second) => self.secondary.deallocate(second),
        }
    }

    unsafe fn resolve<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        match handle {
            Primary(first) => self.primary.resolve(first),
            Secondary(second) => self.secondary.resolve(second),
        }
    }

    unsafe fn resolve_mut<T>(&mut self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]> {
        match handle {
            Primary(first) => self.primary.resolve_mut(first),
            Secondary(second) => self.secondary.resolve_mut(second),
        }
    }

    unsafe fn try_grow<T>(
        &mut self,
        handle: Self::Handle<T>,
        new_capacity: Self::Capacity,
    ) -> Result<Self::Handle<T>, AllocError> {
        match handle {
            Primary(first) => {
                let first_capacity = into_first::<F, S>(new_capacity);

                match first_capacity
                    .and_then(|new_capacity| self.primary.try_grow(first, new_capacity))
                {
                    Ok(handle) => Ok(Primary(handle)),
                    Err(_) => {
                        let second = self.secondary.allocate(new_capacity)?;
                        transfer(self.primary.resolve_mut(first), self.secondary.resolve_mut(second));
                        self.primary.deallocate(first);
                        Ok(Secondary(second))
                    }
                }
            }
            Secondary(second) => self
                .secondary
                .try_grow(second, new_capacity)
                .map(|handle| Secondary(handle)),
        }
    }

    unsafe fn try_shrink<T>(
        &mut self,
        handle: Self::Handle<T>,
        new_capacity: Self::Capacity,
    ) -> Result<Self::Handle<T>, AllocError> {
        let first_capacity = into_first::<F, S>(new_capacity);

        match handle {
            Primary(first) => self
                .primary
                .try_shrink(first, first_capacity?)
                .map(|handle| Primary(handle)),
            Secondary(second) => {
                if let Ok(first) = first_capacity.and_then(|cap| self.primary.allocate(cap)) {
                    transfer(self.secondary.resolve_mut(second), self.primary.resolve_mut(first));
                    self.secondary.deallocate(second);
                    Ok(Primary(first))
                } else {
                    self.secondary
                        .try_shrink(second, new_capacity)
                        .map(|handle| Secondary(handle))
                }
            }
        }
    }
}

impl<F, S> SingleRangeStorage for Fallback<F, S>
where
    F: SingleRangeStorage,
    S: SingleRangeStorage,
{
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        let first_capacity = into_first::<F, S>(capacity);

        if let Ok(first) = first_capacity.and_then(|cap| self.primary.allocate(cap)) {
            Ok(Primary(first))
        } else {
            self.secondary
                .allocate(capacity)
                .map(|handle| Secondary(handle))
        }
    }
}

impl<F, S> Debug for Fallback<F, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "MultiElement")
    }
}

fn into_first<F: RangeStorage, S: RangeStorage>(
    capacity: S::Capacity,
) -> Result<F::Capacity, AllocError> {
    F::Capacity::from_usize(capacity.into_usize()).ok_or(AllocError)
}

unsafe fn transfer<T>(from: NonNull<[MaybeUninit<T>]>, mut to: NonNull<[MaybeUninit<T>]>) {
    let from = from.as_ref();
    let to = to.as_mut();

    ptr::copy_nonoverlapping(
        from.as_ptr(),
        to.as_mut_ptr(),
        cmp::min(from.len(), to.len()),
    );
}
