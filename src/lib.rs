#![cfg_attr(not(test), no_std)]

//  Language Features
#![feature(coerce_unsized)]
#![feature(generic_associated_types)]
#![feature(ptr_metadata)]
#![feature(unsize)]
#![feature(untagged_unions)]

//  Library Features
#![feature(allocator_api)]
#![feature(layout_for_ptr)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(slice_ptr_get)]
#![feature(slice_ptr_len)]

//  Lints
#![deny(missing_docs)]

//! TODO

pub mod allocator;
pub mod alternative;
pub mod collections;
pub mod fallback;
pub mod inline;
pub mod small;
pub mod traits;

mod utils;
