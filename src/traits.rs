//! The various storages available.

use core::{alloc::AllocError, convert::TryInto, marker::Unsize, mem::MaybeUninit, ptr::{self, NonNull}};

use rfc2580::Pointee;

//
//  Element Storage
//

/// A storage for storing elements one at a time.
///
/// This trait is further refined into:
///
/// -   `SingleElementStorage`, which stores up to a single element at any one time.
/// -   `MultiElementStorage`, which may store multiple elements at any one time.
pub trait ElementStorage {
    /// The Handle used to obtain the elements.
    type Handle<T: ?Sized + Pointee> : Clone + Copy;

    /// Destroys the value stored within the storage.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handle` is valid, and the meta-data of the value it represents is valid.
    /// -   This invalidates the value behind the `handle`, hence `get` or `coerce` are no longer safe to be called on
    ///     either it or any of its copies.
    unsafe fn destroy<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.get(handle);

        //  Safety:
        //  -   `element` is valid.
        ptr::drop_in_place(element.as_ptr());

        self.deallocate(handle);
    }

    /// Deallocate the memory without destroying the value within the storage.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handle` is valid, and the meta-data of the value it represents is valid.
    /// -   This invalidates the `handle`, and all of its copies.
    unsafe fn deallocate<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>);

    /// Gets a pointer to the storage to the element.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> NonNull<T>;

    /// Coerces the type of the handle.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid, and was issued by this instance.
    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U>;

}

/// A single element storage.
///
/// Examples of use include: Box.
pub trait SingleElementStorage : ElementStorage {
    /// Stores a `value` within the storage.
    ///
    /// If a value is already stored, it is overwritten and `drop` is not executed.
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        let meta = rfc2580::into_non_null_parts(NonNull::from(&value)).0;

        if let Ok(handle) = self.allocate(meta) {
            //  Safety:
            //  -   `handle` is valid.
            let pointer = unsafe { self.get(handle) };

            //  Safety:
            //  -   `pointer` points to a suitable memory area for `T`.
            unsafe { ptr::write(pointer.as_ptr(), value) };

            Ok(handle)
        } else {
            Err(value)
        }
    }

    /// Attempts to allocate memory, and returns a handle to it.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// If a value is already stored, the memory area may overlap.
    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::MetaData) -> Result<Self::Handle<T>, AllocError>;
}

/// A multi elements storage.
///
/// Examples of use include: BTreeMap, LinkedList, SkipList.
pub trait MultiElementStorage : ElementStorage{
    /// Attempts to store `value` in a newly allocated memory slot.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// #   Safety
    ///
    /// -   The Handle obtained is only valid until `self.destroy` or `self.deallocate` is invoked on it, or one of its
    ///     copies.
    /// -   This may relocate all existing elements, pointers should be re-acquired through their handles.
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T> {
        let meta = rfc2580::into_non_null_parts(NonNull::from(&value)).0;

        if let Ok(handle) = self.allocate(meta) {
            //  Safety:
            //  -   `handle` is valid.
            let pointer = unsafe { self.get(handle) };

            //  Safety:
            //  -   `pointer` points to a suitable memory area for `T`.
            unsafe { ptr::write(pointer.as_ptr(), value) };

            Ok(handle)
        } else {
            Err(value)
        }
    }

    /// Allocates memory, and returns a handle to it.
    ///
    /// This may fail if memory cannot be allocated for it.
    fn allocate<T: ?Sized + Pointee>(&mut self, meta: T::MetaData) -> Result<Self::Handle<T>, AllocError>;
}

//
//  Range Storage
//

/// Capacity type for range storage.
pub trait Capacity : Sized + Clone + Copy {
    /// The maximum possible value of this type.
    fn max() -> Self;

    /// Create from usize.
    fn from_usize(capacity: usize) -> Option<Self>;

    /// Convert back to usize.
    fn into_usize(self) -> usize;
}

/// A storage for (contigous) ranges of elements.
///
/// This trait is further refined into:
///
/// -   `SingleRangeStorage`, which stores up to one single range at any one time.
/// -   `MultiRangeStorage`, which may store multiple ranges at any one time.
pub trait RangeStorage {
    /// The Handle used to obtain the range.
    type Handle<T> : Clone + Copy;

    /// The Capacity type used by the storage.
    ///
    /// The collection may which to use it for related values to keep them as compact as possible.
    type Capacity : Capacity;

    /// Indicates the maximum capacity of a single range possibly available for an element of type `T`.
    fn maximum_capacity<T>(&self) -> Self::Capacity;

    /// Deallocates the memory of the range associated to `handle`, without invoking any destructor.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handles` points to an allocated memory slot, makes no assumption about whether its value is valid.
    /// -   This invalidates `handle`, and all of its copies.
    unsafe fn deallocate<T>(&mut self, handle: Self::Handle<T>);

    /// Gets a pointer to the storage to the range of elements.
    ///
    /// The pointer is only valid as long as the storage is not moved, or the range is not resized.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid, and was issued by this instance.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get<T>(&self, handle: Self::Handle<T>) -> NonNull<[MaybeUninit<T>]>;

    /// Attempts to grow the internal storage to accomodate at least `new_capacity` elements in total.
    ///
    /// If the attempt succeeds, a new handle is returned and `handle` is invalidated.
    unsafe fn try_grow<T>(&mut self, _handle: Self::Handle<T>, _new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        Err(AllocError)
    }

    /// Attempts to shrink the internal storage to accomodate at least `new_capacity` elements in total.
    ///
    /// If the attempt succeeds, a new handle is returned and `handle` is invalidated.
    unsafe fn try_shrink<T>(&mut self, _handle: Self::Handle<T>, _new_capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError> {
        Err(AllocError)
    }
}

/// A single range storage.
///
/// Examples of use include: Vec, VecDeque.
pub trait SingleRangeStorage : RangeStorage {
    /// Allocates memory for a new `Handle`, large enough to at least accomodate the required `capacity`.
    ///
    /// Does not `deallocate` the current handles, nor drop their content. It merely invalidates them.
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError>;
}

/// A multi elements storage.
///
/// Examples of use include: CompactHashMap.
pub trait MultiRangeStorage : RangeStorage{
    /// Allocates memory for a new `Handle`, large enough to at least accomodate the required `capacity`.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// #   Safety
    ///
    /// -   The Handle obtained is only valid until `self.destroy` or `self.deallocate` is invoked on it, or one of its
    ///     copies.
    /// -   This may relocate all existing ranges, which should be re-acquired through their handles.
    fn allocate<T>(&mut self, capacity: Self::Capacity) -> Result<Self::Handle<T>, AllocError>;
}


//
//  Implementations of Capacity.
//

impl Capacity for usize {
    fn max() -> usize { usize::MAX }

    fn from_usize(capacity: usize) -> Option<Self> { Some(capacity) }

    fn into_usize(self) -> usize { self }
}

impl Capacity for u8 {
    fn max() -> Self { u8::MAX }

    fn from_usize(capacity: usize) -> Option<Self> { capacity.try_into().ok() }

    fn into_usize(self) -> usize { self as usize }
}

impl Capacity for u16 {
    fn max() -> Self { u16::MAX }

    fn from_usize(capacity: usize) -> Option<Self> { capacity.try_into().ok() }

    fn into_usize(self) -> usize { self as usize }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl Capacity for u32 {
    fn max() -> Self { u32::MAX }

    fn from_usize(capacity: usize) -> Option<Self> { capacity.try_into().ok() }

    fn into_usize(self)-> usize { self as usize }
}
