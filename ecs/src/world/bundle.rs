mod typed;
pub use typed::*;
mod named;
pub use named::*;
mod lazy;
pub use lazy::*;
mod lazy_named;
pub use lazy_named::*;
mod type_erased;
pub use type_erased::*;

use crate::{dyn_option::{ CellDynOption, FunDynOption }, world::TypeMismatchedError};
use super::{
    ComponentInputDefaultOrNot, World, Component, ComponentEntity,
    ComponentIsNotAliveError, ComponentRequiresValueError, EntityIsNotAliveError,
    ComponentDoesNotHaveStorageError, ForbiddenError,
};

use std::{ any::TypeId, cell::Cell };
use utils::prelude::*;

pub trait BundleValueIterable {
    type Item<'a>: Into<ComponentInputDefaultOrNot<'a>> = ComponentInputDefaultOrNot<'a>
        where Self: 'a;

    /// This should only be called once
    /// Any additional calls may panic or return a nonsense iterator and/or values
    fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>>;
}

#[doc(hidden)]
pub trait BundleErrorFrom<T> {
    const REACHABLE: bool;

    /// In contrast to [`From`], this can and is expected to panic if an error
    /// is not supposed to be possible for this Bundle
    fn bundle_from(value: T) -> Self;
}

impl<A, B> BundleErrorFrom<B> for A
    where A: From<B>
{
    const REACHABLE: bool = true;

    fn bundle_from(value: B) -> Self {
        A::from(value)
    }
}

trait_alias!(pub trait BundleError = BundleErrorFrom<EntityIsNotAliveError> +
    BundleErrorFrom<ComponentIsNotAliveError> +
    BundleErrorFrom<ComponentRequiresValueError> +
    BundleErrorFrom<ComponentDoesNotHaveStorageError> +
    BundleErrorFrom<TypeMismatchedError> +
    BundleErrorFrom<ForbiddenError>);

/// A `bundle` of components
pub trait Bundle: Sized {
    type Error: BundleError;

    /// The number of components in this bundle
    fn len(&self) -> usize;

    /// This registers and returns all components in this bundle.
    /// The slice is the same size as reported by `self.len()`
    fn accumulate_components(&self, world: &mut World) -> impl AsRef<[ComponentEntity]> + use<Self>;

    /// This should return the type id of the values of each component stored
    /// in this bundle.
    /// Should have the same size as reported by `self.len()`
    fn components_value_types(&self) -> impl Iterator<Item = Option<(TypeId, &'static str)>>;

    /// This returns a value that can be iterated and has the same amount of value
    /// Its iterator should have the same size as reported by `self.len()`
    fn into_component_values(self) -> impl BundleValueIterable;
}
