#![cfg_attr(not(test), no_std)]

//  Language Features
#![feature(coerce_unsized)]
#![feature(generic_associated_types)]
#![feature(unsize)]
#![feature(untagged_unions)]

//  Library Features
#![feature(layout_for_ptr)]
#![feature(maybe_uninit_uninit_array)]
#![feature(option_unwrap_none)]

#![cfg_attr(feature = "alloc", feature(allocator_api))]

//  Lints
#![allow(incomplete_features)]
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
