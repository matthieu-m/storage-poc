//! Simple implementations of the various inline storages.

mod multi_element;
mod single_element;
mod single_range;

pub use multi_element::{MultiElement, MultiElementHandle};
pub use single_element::SingleElement;
pub use single_range::SingleRange;
