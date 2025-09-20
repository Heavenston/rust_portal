use super::*;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum LazyBundleError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    Forbidden(#[from] ForbiddenError),
}

impl BundleErrorFrom<ComponentRequiresValueError> for LazyBundleError {
    const REACHABLE: bool = false;
    fn bundle_from(_value: ComponentRequiresValueError) -> Self {
        unreachable!("Lazy bundles always provide a value for components")
    }
}

impl BundleErrorFrom<ComponentDoesNotHaveStorageError> for LazyBundleError {
    const REACHABLE: bool = false;
    fn bundle_from(_value: ComponentDoesNotHaveStorageError) -> Self {
        unreachable!("Lazy bundles use World::component to get components which should always return entities with the correct storages")
    }
}

impl BundleErrorFrom<TypeMismatchedError> for LazyBundleError {
    const REACHABLE: bool = false;
    fn bundle_from(_value: TypeMismatchedError) -> Self {
        unreachable!("Lazy bundles use World::component to get components which should always return entities with the correct storages")
    }
}

impl BundleErrorFrom<ComponentIsNotAliveError> for LazyBundleError {
    const REACHABLE: bool = false;
    fn bundle_from(_value: ComponentIsNotAliveError) -> Self {
        unreachable!("Lazy bundles use World::component to get components which should always return alive entities")
    }
}

/// Private struct implementing `BundleValueIterable` for each tuple for `LazyBundle`
struct BundleValueIterator<Tuple>(Tuple);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LazyBundle<T>(pub T);

macro_rules! impl_named_bundle {
    ($(($I: tt, $F: ident, $T: ident)),*) => {
        impl<$($F, $T,)*> Bundle for LazyBundle<($($F,)*)>
            where $($T: 'static,)*
                  $($F: FnOnce() -> $T,)*
        {
            type Error = LazyBundleError;

            fn len(&self) -> usize {
                count_args_literal!($($T),*)
            }

            fn accumulate_components(&self, world: &mut World) -> [ComponentEntity; count_args_literal!($($T),*)] {
                let _ = world;
                [$(world.component::<$T>(),)*]
            }

            fn components_value_types(&self) -> impl Iterator<Item = Option<(TypeId, &'static str)>> {
                [$(Some((TypeId::of::<$T>(), std::any::type_name::<$T>())),)*].into_iter()
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
