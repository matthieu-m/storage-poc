//! Simple implementations of a composite storages, which aggregate two storages together.
//!
//! This is an attempt at implementing a generic way to combining existing storages, to simplify implementating small
//! storages for example.
//!
//! There are difficulties, though:
//!
//! -   Switching storages on the fly imply the ability to summon a storage from nothingness, hence the juggling of
//!     builders, and the Poisoned state in case user provided functions panic.
//! -   Switching handles, as storages switch, is easy for Single storages -- as the only handle is invalidated --
//!     however there doesn't seem to be an elegant solution for Multi storages, therefore they are not implemented.

mod builder;
mod inner;
mod single_element;
mod single_range;

pub use builder::{Builder, DefaultBuilder};
pub use single_element::SingleElement;
pub use single_range::SingleRange;

use inner::Inner;
