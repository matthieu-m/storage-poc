//! Various utilities.

use core::{alloc::Layout, fmt::{self, Debug}, marker::PhantomData, mem};

#[cfg(all(test, feature = "alloc"))]
pub(crate) use test::*;

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
pub fn validate_layout<T: ?Sized + LayoutOf, Storage>() -> Result<(), ()> {
    validate_layout_for::<Storage>(T::layout_of())
}

/// Validates that the layout of `storage` is sufficient for `layout`.
///
/// Return `Ok` on success, and `Err` on failure.
pub fn validate_layout_for<Storage>(layout: Layout) -> Result<(), ()> {
    let validated_size = layout.size() <= mem::size_of::<Storage>();
    let validated_alignment = layout.align() <= mem::align_of::<Storage>();

    if validated_size && validated_alignment {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(all(test, feature = "alloc"))]
mod test {

use core::{cell::Cell, ptr::NonNull};

use alloc::{alloc::{Allocator, AllocError, Global, Layout}, rc::Rc};

//  A NonAllocator never allocates.
#[derive(Debug, Default)]
pub(crate) struct NonAllocator;

unsafe impl Allocator for NonAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> { Err(AllocError) }
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) { panic!("NonAllocator::deallocate called!") }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SpyAllocator(Rc<(Cell<usize>, Cell<usize>)>);

#[cfg(all(test, feature = "alloc"))]
impl SpyAllocator {
    pub(crate) fn allocated(&self) -> usize { self.0.0.get() }

    pub(crate) fn deallocated(&self) -> usize { self.0.1.get() }
}

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

} // mod test

//
//  Implementation
//

#[doc(hidden)]
pub trait LayoutOf {
    #[doc(hidden)]
    fn layout_of() -> Layout;
}

impl<T: ?Sized> LayoutOf for T {
    default fn layout_of() -> Layout { Layout::new::<u8>() }
}

impl<T> LayoutOf for T {
    fn layout_of() -> Layout { Layout::new::<T>() }
}
