use crate::dyn_option::DynOption;
use super::{
    ComponentDenseStorageInput, World, Component, ComponentEntity
};

use utils::prelude::*;

pub trait BundleValueIterable {
    type Item<'a>: for<'w> Into<ComponentDenseStorageInput<'w, 'a>>
        where Self: 'a;

    /// This should only be called once
    /// Any additional calls may panic or return a nonsense iterator and/or values
    fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>>;
}

/// Private struct implementing `BundleValueIterable` for each tuple bundle 
struct BundleValueIterator<Tuple> {
    tuple: Tuple,
    #[cfg(debug_assertions)]
    double_drain_check: bool,
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

macro_rules! bundle_impl {
    ($(($I: tt, $T: ident)),*) => {
        impl<$($T),*> Bundle for ($($T,)*)
            where $($T: Component,)*
        {
            fn len(&self) -> usize {
                count_args_literal!($($T),*)
            }

            fn accumulate_components(&self, world: &mut World) -> [ComponentEntity; count_args_literal!($($T),*)] {
                let _ = world;

                [$(world.component::<$T>()),*]
            }

            fn into_component_values(self) -> impl BundleValueIterable {
                BundleValueIterator {
                    tuple: ($(Some(self.$I),)*),
                    #[cfg(debug_assertions)]
                    double_drain_check: false,
                }
            }
        }

        impl<$($T),*> BundleValueIterable for BundleValueIterator<($(Option<$T>,)*)>
            where $($T: Component,)*
        {
            type Item<'a> = &'a mut dyn DynOption;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                #[cfg(debug_assertions)]
                { assert!(!self.double_drain_check, "Called BundleValueIterable::bundle_values_iter twice");
                  self.double_drain_check = true; }
                [$(&mut self.tuple.$I as &mut dyn DynOption,)*].into_iter()
            }
        }
    };
}
variadics_please::all_tuples_enumerated!(bundle_impl, 0, 16, T);
