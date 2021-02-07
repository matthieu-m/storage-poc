//! Proof of Concept implementations of some collections, to demonstrate the use of Storages.

mod raw_box;
mod raw_linked_list;

pub use raw_box::RawBox;
pub use raw_linked_list::{RawLinkedList, RawLinkedListNodeStorage};
