//! Simple implementations of the fallback principle, allocating from one and fallbacking to another.
//!
//! This is an attempt at implementing a generic way to combining existing storages, to simplify implementating small
//! storages for example.
//!
//! It is simpler than alternative, however is heavier weight.

mod multi_element;
mod single_element;
mod single_range;

pub use multi_element::MultiElement;
pub use single_element::SingleElement;
pub use single_range::SingleRange;
