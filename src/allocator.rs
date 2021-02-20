//! Simple implementations of the various allocator adaptor storages.

mod builder;
mod multi_element;
mod single_element;
mod single_range;

pub use builder::AllocatorBuilder;
pub use multi_element::MultiElement;
pub use single_element::SingleElement;
pub use single_range::SingleRange;
