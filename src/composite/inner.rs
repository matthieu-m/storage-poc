//  The inner storage, to dispatch on both types.

use core::ops::{Deref, DerefMut};

use super::Builder;

//  Alternative type.
pub(crate) enum Inner<F, S, FB, SB> {
    First(InnerElement<F, SB>),
    Second(InnerElement<S, FB>),
    Poisoned,
}

impl<F, S, FB, SB> Inner<F, S, FB, SB> {
    pub(crate) fn first(value: F, builder: SB) -> Self {
        Self::First(InnerElement{ value, builder })
    }

    pub(crate) fn second(value: S, builder: FB) -> Self {
        Self::Second(InnerElement{ value, builder })
    }
}

impl<F: Default, S, FB, SB: Default> Default for Inner<F, S, FB, SB> {
    fn default() -> Self { Self::First(InnerElement::default()) }
}

//  Element of alternative type.
#[derive(Default)]
pub(crate) struct InnerElement<V, B> {
    value: V,
    builder: B,
}

impl<V, B> InnerElement<V, B> {
    pub(crate) fn transform<OV, OB, Fun, R>(self, fun: Fun) -> (InnerElement<OV, OB>, R)
        where
            B: Builder<OV>,
            OB: Builder<V>,
            Fun: FnOnce(&mut V, &mut OV) -> R,
    {
        let InnerElement { mut value, builder } = self;
        let mut other_value = B::into_storage(builder);
        let result = fun(&mut value, &mut other_value);
        let other_builder = OB::from_storage(value);

        (InnerElement { value: other_value, builder: other_builder }, result)
    }
}

impl<V, B> Deref for InnerElement<V, B> {
    type Target = V;

    fn deref(&self) -> &Self::Target { &self.value }
}

impl<V, B> DerefMut for InnerElement<V, B> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}
