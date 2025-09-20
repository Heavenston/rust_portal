use super::*;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum NamedBundleError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    ComponentIsNotAlive(#[from] ComponentIsNotAliveError),
    ComponentRequiresValue(#[from] ComponentRequiresValueError),
    ComponentDoesNotHaveStorage(#[from] ComponentDoesNotHaveStorageError),
    TypeMismatchedError(#[from] TypeMismatchedError),
    Forbidden(#[from] ForbiddenError),
}

/// Private struct implementing `BundleValueIterable` for each tuple for `NamedBundle`
struct BundleValueIterator<Tuple>(Tuple);

pub trait NamedBundleItem {
    type T: 'static;
    type O: PartialOption<Self::T>;

    fn component_entity(&self, world: &mut World) -> ComponentEntity;
    fn value_type_info(&self) -> Option<(TypeId, &'static str)>;
    /// If None is returned from this, then the Default value will be used
    /// as the value, otherwise the given value will be used.
    fn into_value(self) -> impl PartialOption<Self::T>;
}

impl NamedBundleItem for ComponentEntity {
    type T = ();
    type O = NeverOption;

    fn component_entity(&self, _world: &mut World) -> ComponentEntity {
        *self
    }

    fn value_type_info(&self) -> Option<(TypeId, &'static str)> {
        None
    }

    fn into_value(self) -> NeverOption {
        NeverOption::None
    }
}

impl<T: 'static> NamedBundleItem for (ComponentEntity, T) {
    type T = T;
    type O = AlwaysOption<T>;

    fn component_entity(&self, _world: &mut World) -> ComponentEntity {
        self.0
    }

    fn value_type_info(&self) -> Option<(TypeId, &'static str)> {
        Some((TypeId::of::<T>(), std::any::type_name::<T>()))
    }

    fn into_value(self) -> AlwaysOption<T> {
        AlwaysOption::Some(self.1)
    }
}

pub struct NamedBundle<T>(pub T);

macro_rules! impl_named_bundle {
    ($(($I: tt, $T: ident)),*) => {
        impl<$($T,)*> Bundle for NamedBundle<($($T,)*)>
            where $($T: NamedBundleItem,)*
        {
            type Error = NamedBundleError;

            fn len(&self) -> usize {
                count_args_literal!($($T),*)
            }

            fn accumulate_components(&self, world: &mut World) -> [ComponentEntity; count_args_literal!($($T),*)] {
                let _ = world;
                [$(self.0.$I.component_entity(world),)*]
            }

            fn components_value_types(&self) -> impl Iterator<Item = Option<(TypeId, &'static str)>> {
                [$(self.0.$I.value_type_info(),)*].into_iter()
            }

            fn into_component_values(self) -> impl BundleValueIterable {
                BundleValueIterator(($(self.0.$I.into_value().into_option().map(|v| Cell::new(Some(v))),)*))
            }
        }

        impl<$($T,)*> BundleValueIterable for BundleValueIterator<($(Option<Cell<Option<$T>>>,)*)>
            where $($T: 'static,)*
        {
            type Item<'a> = ComponentInputDefaultOrNot<'a>;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                [
                    $(match &mut self.0.$I {
                        Some(val) => ComponentInputDefaultOrNot::DynOption(val),
                        None => ComponentInputDefaultOrNot::Default,
                    }),*
                ].into_iter()
            }
        }
    };
}
variadics_please::all_tuples_enumerated!(impl_named_bundle, 0, 16, T);
