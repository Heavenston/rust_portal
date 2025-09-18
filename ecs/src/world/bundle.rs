mod typed;
mod named;
pub use named::{ NamedBundle, NamedBundleItem };
mod lazy;
pub use lazy::LazyBundle;
mod lazy_named;
pub use lazy_named::{ LazyNamedBundle, LazyNamedBundleItem };
mod type_erased;
pub use type_erased::TypeErasedBundle;

use crate::dyn_option::{ CellDynOption, FunDynOption };
use super::{
    ComponentInputDefaultOrNot, World, Component, ComponentEntity
};

use std::cell::Cell;
use utils::prelude::*;

pub trait BundleValueIterable {
    type Item<'a>: Into<ComponentInputDefaultOrNot<'a>> = ComponentInputDefaultOrNot<'a>
        where Self: 'a;

    /// This should only be called once
    /// Any additional calls may panic or return a nonsense iterator and/or values
    fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>>;
}

impl BundleValueIterable for Option<ComponentInputDefaultOrNot<'_>> {
    fn bundle_values_iter<'s>(&'s mut self) -> std::option::IntoIter<Self::Item<'s>> {
        self.take().into_iter()
    }
}

/// A `bundle` of components
pub trait Bundle: Sized {
    /// The number of components in this bundle
    fn len(&self) -> usize;

    /// This registers and returns all components in this bundle.
    /// The slice is the same size as reported by `self.len()`
    fn accumulate_components(&self, world: &mut World) -> impl AsRef<[ComponentEntity]> + use<Self>;
    /// This returns a value that can be iterated and has the same amount of value
    /// Its iterator should have the same size as reported by `self.len()`
    fn into_component_values(self) -> impl BundleValueIterable;
}
