//! The various storages available.

use core::{marker::Unsize, ptr::NonNull};

/// A pointer to memory suitable for T.
pub type Element<T> = NonNull<T>;

/// A pointer to memory suitable for the indicated number of Ts.
pub type Range<T> = (Element<T>, usize);

/// A single element storage.
///
/// Examples of use include: Box.
pub trait SingleElementStorage<T: ?Sized> {
    /// Stores a `value` within the storage.
    ///
    /// If a value is already stored, it is overwritten and `drop` is not executed.
    fn create(&mut self, value: T) -> Result<Element<T>, T>
        where
            T: Sized;

    /// Stores a `value` within the storage.
    ///
    /// If a value is already stored, it is overwritten and `drop` is not executed.
    fn create_unsize<V: Unsize<T>>(&mut self, value: V) -> Result<Element<T>, V>;

    /// Destroys the value stored within the storage.
    ///
    /// #   Safety
    ///
    /// -   Assumes that there is a value stored.
    unsafe fn destroy(&mut self);

    /// Gets a pointer to the storage to the element.
    ///
    /// #   Safety
    ///
    /// -   Assumes that there is a value stored.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get(&self) -> Element<T>;
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
    ///
    /// On success, may invoke `relocator` with the new `Range` to give the opportunity to move the elements from the
    /// current range to the new range. The current and new ranges may overlap.
    fn try_grow<F>(&mut self, new_capacity: usize, relocator: F) -> Option<Range<T>>
        where
            F: FnOnce(Range<T>);

    /// Attempts to shrink the internal storage to accomodate at least `new_capacity` elements in total.
    ///
    /// On success, may invoke `relocator` with the new `Range` to give the opportunity to move the elements from the
    /// current range to the new range. The current and new ranges may overlap.
    fn try_shrink<F>(&mut self, new_capacity: usize, relocator: F) -> Option<Range<T>>
        where
            F: FnOnce(Range<T>);
}

/// A multi elements storage.
///
/// Examples of use include: BTreeMap, LinkedList, SkipList.
pub trait MultiElementStorage<T: ?Sized> {
    /// The Handle used to obtain the elements.
    type Handle;

    /// Attempts to store `value` in a newly allocated memory slot.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// #   Safety
    ///
    /// -   The Element obtained is only valid until `self` is moved, or `self.destroy` is invoked on its handle.
    /// -   This may relocate all existing elements, which should be re-acquired through their handles.
    fn create(&mut self, value: T) -> Option<(Self::Handle, Element<T>)>
        where
            T: Sized;

    /// Attempts to store `value` in a newly allocated memory slot.
    ///
    /// This may fail if memory cannot be allocated for it.
    ///
    /// #   Safety
    ///
    /// -   The Element obtained is only valid until `self` is moved, or `self.destroy` is invoked on its handle.
    /// -   This may relocate all existing elements, which should be re-acquired through their handles.
    fn create_unsize<V: ?Sized>(&mut self, value: V) -> Option<(Self::Handle, Element<T>)>
        where
            V: Unsize<T>;

    /// Destroys the element associated to `handle` and deallocates its memory slot.
    ///
    /// #   Safety
    ///
    /// -   Assumes `handles` points to an allocated memory slot containing a valid value.
    /// -   This invalidates the `handle`, and all of its copies.
    unsafe fn destroy(&mut self, handle: Self::Handle);

    /// Returns the Element associated to this `handle`.
    ///
    /// #   Safety
    ///
    /// -   Assumes that `handle` is valid, and was issued by this instance.
    /// -   The pointer is only valid as long as the storage is not moved.
    unsafe fn get(&self, handle: Self::Handle) -> Element<T>;
}

//  Are MultiRangeStorage<T> and MultiResizableRangeStorage<T> necessary?
