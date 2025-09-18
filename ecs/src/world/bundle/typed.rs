//! Module where the implementation of the Bundle trait for all tuples
//! of N components (up to 16) lies

use super::*;

/// Private struct implementing `BundleValueIterable` for each tuple bundle 
struct BundleValueIterator<Tuple> {
    tuple: Tuple,
    #[cfg(debug_assertions)]
    double_drain_check: bool,
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
                    tuple: ($(Cell::new(Some(self.$I)),)*),
                    #[cfg(debug_assertions)]
                    double_drain_check: false,
                }
            }
        }

        impl<$($T),*> BundleValueIterable for BundleValueIterator<($(Cell<Option<$T>>,)*)>
            where $($T: Component,)*
        {
            type Item<'a> = &'a dyn CellDynOption;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                #[cfg(debug_assertions)]
                { assert!(!self.double_drain_check, "Called BundleValueIterable::bundle_values_iter twice");
                  self.double_drain_check = true; }
                [$(&self.tuple.$I as &dyn CellDynOption,)*].into_iter()
            }
        }
    };
}
variadics_please::all_tuples_enumerated!(bundle_impl, 0, 16, T);

