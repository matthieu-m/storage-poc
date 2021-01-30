#![cfg_attr(not(test), no_std)]

#![feature(coerce_unsized)]
#![feature(option_unwrap_none)]
#![feature(set_ptr_value)]
#![feature(unsize)]

#![cfg_attr(feature = "alloc", feature(allocator_api))]

#![deny(missing_docs)]

//! TODO

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod collections;
pub mod inline;
pub mod traits;

#[cfg(feature = "alloc")]
pub mod allocator;

#[cfg(feature = "alloc")]
pub mod small;

mod utils;
