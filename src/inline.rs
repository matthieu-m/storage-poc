//! Simple implementations of the various inline storages.

mod multi_element;
mod single_element;

pub use multi_element::{MultiElement, MultiElementHandle};
pub use single_element::SingleElement;
