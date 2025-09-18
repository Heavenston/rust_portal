use super::*;

/// Private struct implementing `BundleValueIterable` for each tuple for `LazyBundle`
struct BundleValueIterator<Tuple>(Tuple);

pub struct LazyBundle<T>(pub T);

macro_rules! impl_named_bundle {
    ($(($I: tt, $F: ident, $T: ident)),*) => {
        impl<$($F, $T,)*> Bundle for LazyBundle<($($F,)*)>
            where $($T: 'static,)*
                  $($F: FnOnce() -> $T,)*
        {
            fn len(&self) -> usize {
                count_args_literal!($($T),*)
            }

            fn accumulate_components(&self, world: &mut World) -> [ComponentEntity; count_args_literal!($($T),*)] {
                let _ = world;
                [$(world.component::<$T>(),)*]
            }

            fn into_component_values(self) -> impl BundleValueIterable {
                BundleValueIterator(($(Cell::new(FunDynOption::new(self.0.$I)),)*))
            }
        }

        impl<$($F, $T,)*> BundleValueIterable for BundleValueIterator<($(Cell<FunDynOption<$F>>,)*)>
            where $($T: 'static,)*
                  $($F: FnOnce() -> $T,)*
        {
            type Item<'a> = &'a dyn CellDynOption
                where Self: 'a;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                [$(&self.0.$I as &dyn CellDynOption,)*].into_iter()
            }
        }
    };
}
variadics_please::all_tuples_enumerated!(impl_named_bundle, 0, 16, F, T);
