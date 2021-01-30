//! Various utilities.

use core::{fmt::{self, Debug}, marker::PhantomData, mem};

#[cfg(all(test, feature = "alloc"))]
use core::{cell::Cell, ptr::NonNull};

#[cfg(all(test, feature = "alloc"))]
use alloc::{alloc::{Allocator, AllocError, Global, Layout}, rc::Rc};

/// A marker to signal the absence of ownership of T, while requiring its invariance.
pub struct PhantomInvariant<T: ?Sized>(PhantomData<fn(T) -> T>);

impl<T: ?Sized> Debug for PhantomInvariant<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "PhantomInvariant")
    }
}

impl<T: ?Sized> Default for PhantomInvariant<T> {
    fn default() -> Self { Self(PhantomData) }
}

/// Validates that the layout of `storage` is sufficient to accomodate an instance of `T`.
///
/// Return `Ok` on success, and `Err` on failure.
pub fn validate_layout<T, Storage>() -> Result<(), ()> {
    let validated_size = mem::size_of::<T>() <= mem::size_of::<Storage>();
    let validated_alignment = mem::align_of::<T>() <= mem::align_of::<Storage>();

    if validated_size && validated_alignment {
        Ok(())
    } else {
        Err(())
    }
}

//  A NonAllocator never allocates.
#[cfg(all(test, feature = "alloc"))]
#[derive(Debug, Default)]
pub(crate) struct NonAllocator;

#[cfg(all(test, feature = "alloc"))]
unsafe impl Allocator for NonAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> { Err(AllocError) }
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) { panic!("NonAllocator::deallocate called!") }
}

#[cfg(all(test, feature = "alloc"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct SpyAllocator(Rc<(Cell<usize>, Cell<usize>)>);

#[cfg(all(test, feature = "alloc"))]
impl SpyAllocator {
    pub(crate) fn allocated(&self) -> usize { self.0.0.get() }

    pub(crate) fn deallocated(&self) -> usize { self.0.1.get() }
}

#[cfg(all(test, feature = "alloc"))]
unsafe impl Allocator for SpyAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.0.0.set(self.0.0.get() + 1);
        Global.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.0.1.set(self.0.1.get() + 1);
        Global.deallocate(ptr, layout)
    }
}
