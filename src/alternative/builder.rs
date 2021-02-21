//! Builder trait for storages.

/// A trait to build storages, and salvage their state.
pub trait Builder<S> {
    /// Creates an instance of `Self` from `storage`.
    fn from_storage(storage: S) -> Self;

    /// Creates an instance of `S` from `self.`
    fn into_storage(self) -> S;
}

/// An empty builder state, when storages can be default constructed.
#[derive(Debug, Default)]
pub struct DefaultBuilder;

impl<S: Default> Builder<S> for DefaultBuilder {
    fn from_storage(_: S) -> Self { Self::default() }

    fn into_storage(self) -> S { S::default() }
}
