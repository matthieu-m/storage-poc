//! Proof-of-Concept implementation of a LinkedList parameterized by a Storage.

use core::{fmt::{self, Debug}, marker::PhantomData, mem::MaybeUninit, ptr};

use rfc2580::Pointee;

use crate::traits::MultiElementStorage;

/// A PoC LinkedList.
pub struct RawLinkedList<T: Pointee, S: MultiElementStorage> {
    next: Option<S::Handle<RawLinkedListNode<T, S>>>,
    storage: S,
    _marker: PhantomData<T>,
}

impl<T: Pointee, S: MultiElementStorage> RawLinkedList<T, S> {
    /// Creates a new instance from `storage`.
    pub fn new(storage: S) -> Self { Self { next: None, storage, _marker: PhantomData } }

    /// Clears all the elements from the list, leading to an empty list.
    pub fn clear(&mut self) {
        while let Some(_) = self.pop() {}
    }

    /// Returns a reference to the front element of the list, if any.
    pub fn front(&self) -> Option<&T> {
        self.next.map(|handle| unsafe {
            let pointer = self.storage.get(handle).as_ptr();
            let node = &*pointer;
            &node.element
        })
    }

    /// Returns a reference to the front element of the list, if any.
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.next.map(|handle| unsafe {
            let pointer = self.storage.get(handle).as_ptr();
            let node = &mut *pointer;
            &mut node.element
        })
    }

    /// Pushes a new element to the front of the list.
    pub fn push(&mut self, value: T) -> Result<(), T> {
        let node = RawLinkedListNode { next: self.next, element: value };
        let handle = self.storage.create(node).map_err(|node| node.element)?;

        self.next = Some(handle);

        Ok(())
    }

    /// Pops the front element of the list, if any, and returns it if it succeeded.
    pub fn pop(&mut self) -> Option<T> {
        self.next.take().map(|handle| unsafe {
            let mut node = MaybeUninit::<RawLinkedListNode<T, S>>::uninit();
            ptr::copy_nonoverlapping(self.storage.get(handle).as_ptr() as *const _, node.as_mut_ptr(), 1);

            let node = node.assume_init();
            self.storage.release(handle);

            self.next = node.next;
            node.element
        })
    }
}

impl<T: Debug + Pointee, S: MultiElementStorage> Debug for RawLinkedList<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "[")?;

        let mut next = self.next;
        if let Some(handle) = next {
            unsafe {
                let element = self.storage.get(handle);
                let node = element.as_ref();

                write!(f, "{:?}", &node.element)?;
                next = node.next;
            }
        }

        while let Some(handle) = next {
            unsafe {
                let element = self.storage.get(handle);
                let node = element.as_ref();

                write!(f, ", {:?}", &node.element)?;
                next = node.next;
            }
        }

        write!(f, "]")
    }
}

impl<T: Pointee, S: Default + MultiElementStorage> Default for RawLinkedList<T, S> {
    fn default() -> Self { Self::new(S::default()) }
}

impl<T: Pointee, S: MultiElementStorage> Drop for RawLinkedList<T, S> {
    fn drop(&mut self) { self.clear(); }
}

/// A PoC LinkedList storage helper.
///
/// Reserves enough space for storing a list node containing `T`, for a handle of size similar to `H`.
pub struct RawLinkedListNodeStorage<T, H>(Option<H>, MaybeUninit<T>);


//
//  Implementation
//

struct RawLinkedListNode<T, S: MultiElementStorage> {
    next: Option<S::Handle<Self>>,
    element: T,
}

#[cfg(test)]
mod test_inline {

use crate::inline::MultiElement;

use super::*;

#[test]
fn smoke_test() {
    type NodeStorage = RawLinkedListNodeStorage<u8, usize>;
    type List = RawLinkedList<u8, MultiElement<NodeStorage, 4>>;

    let mut list = List::default();

    list.push(1).unwrap();
    list.push(2).unwrap();

    assert_eq!(Some(&2), list.front());

    *list.front_mut().unwrap() = 3;

    assert_eq!(Some(3), list.pop());
    assert_eq!(Some(&1), list.front());
}

} // mod test_inline

#[cfg(all(test, feature = "alloc"))]
mod test_allocator {

use crate::allocator::MultiElement;
use crate::utils::{NonAllocator, SpyAllocator};

use super::*;

#[test]
fn smoke_test() {
    type List = RawLinkedList<String, MultiElement<SpyAllocator>>;

    let allocator = SpyAllocator::default();
    let mut list = List::new(MultiElement::new(allocator.clone()));

    list.push("Hello".to_string()).unwrap();
    list.push("World".to_string()).unwrap();

    assert_eq!(2, allocator.allocated());
    assert_eq!(0, allocator.deallocated());

    assert_eq!(Some(&"World".to_string()), list.front());

    *list.front_mut().unwrap() = "All".to_string();

    assert_eq!(Some("All".to_string()), list.pop());
    assert_eq!(Some(&"Hello".to_string()), list.front());
    assert_eq!(2, allocator.allocated());
    assert_eq!(1, allocator.deallocated());
}

#[test]
fn allocation_failure() {
    type List = RawLinkedList<&'static str, MultiElement<NonAllocator>>;

    let mut list = List::default();

    list.push("Caramba").unwrap_err();
}

} // mod test_allocator
