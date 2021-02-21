#![cfg_attr(not(test), no_std)]

//  Language Features
#![feature(coerce_unsized)]
#![feature(generic_associated_types)]
#![feature(unsize)]
#![feature(untagged_unions)]

//  Library Features
#![feature(allocator_api)]
#![feature(layout_for_ptr)]
#![feature(maybe_uninit_extra)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_uninit_array)]
#![feature(option_unwrap_none)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(slice_ptr_get)]
#![feature(slice_ptr_len)]

//  Lints
#![allow(incomplete_features)]
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
