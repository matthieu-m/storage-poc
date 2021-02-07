//! The various storages available.

use core::{marker::Unsize, ptr::{self, NonNull}};

use rfc2580::Pointee;

/// A pointer to memory suitable for T.
pub type Element<T> = NonNull<T>;

/// A pointer to memory suitable for the indicated number of Ts.
pub type Range<T> = (NonNull<T>, usize);

/// A single element storage.
///
/// Examples of use include: Box.
pub trait SingleElementStorage {
    /// The Handle used to obtain the elements.
    type Handle<T: ?Sized + Pointee> : Clone + Copy;

    /// Stores a `value` within the storage.
    ///
    /// If a value is already stored, it is overwritten and `drop` is not executed.
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T>;

    /// Destroys the value stored within the storage.
    ///
    /// #   Safety
    ///
    /// -   Assumes that there is a value stored.
    unsafe fn destroy<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.get(handle);

        //  Safety:
        //  -   `element` is valid.
        ptr::drop_in_place(element.as_ptr());

        self.forget(handle);
    }

    /// Deallocate the memory without destroying the value within the storage.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handle` is valid, and the meta-data of the value it represents is valid.
    /// -   This invalidates the `handle`, and all of its copies.
    unsafe fn forget<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>);

    /// Gets a pointer to the storage to the element.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> Element<T>;

    /// Coerces the type of the handle.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U>;
}

/// A multi elements storage.
///
/// Examples of use include: BTreeMap, LinkedList, SkipList.
pub trait MultiElementStorage {
    /// The Handle used to obtain the elements.
    type Handle<T: ?Sized + Pointee> : Clone + Copy;

    /// Attempts to store `value` in a newly allocated memory slot.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// #   Safety
    ///
    /// -   The Element obtained is only valid until `self` is moved, or `self.destroy` is invoked on its handle.
    /// -   This may relocate all existing elements, which should be re-acquired through their handles.
    fn create<T: Pointee>(&mut self, value: T) -> Result<Self::Handle<T>, T>;

    /// Destroys the element associated to `handle` and deallocates its memory slot.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handle` is valid, and the meta-data of the value it represents is valid.
    /// -   This invalidates the `handle`, and all of its copies.
    unsafe fn destroy<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>) {
        //  Safety:
        //  -   `handle` is assumed to be valid.
        let element = self.get(handle);

        //  Safety:
        //  -   `element` is valid.
        ptr::drop_in_place(element.as_ptr());

        self.forget(handle);
    }

    /// Deallocate the memory of the element associated to `handle`, without invoking its destructor.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handles` points to an allocated memory slot, makes no assumption about whether its value is valid.
    /// -   This invalidates the `handle`, and all of its copies.
    unsafe fn forget<T: ?Sized + Pointee>(&mut self, handle: Self::Handle<T>);

    /// Returns the Element associated to this `handle`.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid, and was issued by this instance.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get<T: ?Sized + Pointee>(&self, handle: Self::Handle<T>) -> Element<T>;

    /// Coerces the type of the handle.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid, and was issued by this instance.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn coerce<U: ?Sized + Pointee, T: ?Sized + Pointee + Unsize<U>>(&self, handle: Self::Handle<T>) -> Self::Handle<U>;
}

/// A single range storage.
///
/// Examples of use include: Vec, VecDeque.
pub trait SingleRangeStorage<T> {
    /// Gets a pointer to the storage to the range of elements.
    ///
    /// The pointer is only valid as long as the storage is not moved, or the range is not resized.
    fn get(&self) -> Range<T>;
}

/// A resizable single range storage.
///
/// Examples of use include: Vec, VecDeque.
pub trait SingleResizableRangeStorage<T> : SingleRangeStorage<T> {
    /// Attempts to grow the internal storage to accomodate at least `new_capacity` elements in total.
    fn try_grow<F>(&mut self, new_capacity: usize) -> Option<Range<T>>;

    /// Attempts to shrink the internal storage to accomodate at least `new_capacity` elements in total.
    fn try_shrink<F>(&mut self, new_capacity: usize) -> Option<Range<T>>;
}

//  Are MultiRangeStorage<T> and MultiResizableRangeStorage<T> necessary?
