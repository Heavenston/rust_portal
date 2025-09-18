use super::*;

/// Private struct implementing `BundleValueIterable` for each tuple for `LazyNamedBundle`
struct BundleValueIterator<Tuple>(Tuple);

pub trait LazyNamedBundleItem {
    type T: 'static;
    type F: FnOnce() -> Self::T;
    type O: PartialOption<Self::F>;

    fn component_entity(&self, world: &mut World) -> ComponentEntity;
    /// If None is returned from this, then the Default value will be used
    /// as the value, otherwise the given function will be called to get
    /// the component's value.
    fn into_value(self) -> Self::O;
}

impl LazyNamedBundleItem for ComponentEntity {
    type T = ();
    type F = fn() -> ();
    type O = NeverOption;

    fn component_entity(&self, _world: &mut World) -> ComponentEntity {
        *self
    }

    fn into_value(self) -> NeverOption {
        NeverOption::None
    }
}

impl<T, F> LazyNamedBundleItem for (ComponentEntity, F)
    where T: 'static,
          F: FnOnce() -> T,
{
    type T = T;
    type F = F;
    type O = AlwaysOption<F>;

    fn component_entity(&self, _world: &mut World) -> ComponentEntity {
        self.0
    }

    fn into_value(self) -> AlwaysOption<F> {
        AlwaysOption::Some(self.1)
    }
}

pub struct LazyNamedBundle<T>(pub T);

macro_rules! impl_named_bundle {
    ($(($I: tt, $F: ident, $T: ident)),*) => {
        impl<$($T,)*> Bundle for LazyNamedBundle<($($T,)*)>
            where $($T: LazyNamedBundleItem,)*
        {
            fn len(&self) -> usize {
                count_args_literal!($($T),*)
            }

            fn accumulate_components(&self, world: &mut World) -> [ComponentEntity; count_args_literal!($($T),*)] {
                let _ = world;
                [$(self.0.$I.component_entity(world),)*]
            }

            fn into_component_values(self) -> impl BundleValueIterable {
                BundleValueIterator(($(self.0.$I.into_value().into_option().map(|v| Cell::new(FunDynOption::new(v))),)*))
            }
        }

        impl<$($F, $T,)*> BundleValueIterable for BundleValueIterator<($(Option<Cell<FunDynOption<$F>>>,)*)>
            where $($T: 'static,)*
                  $($F: FnOnce() -> $T,)*
        {
            type Item<'a> = ComponentInputDefaultOrNot<'a>
                where Self: 'a;

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
variadics_please::all_tuples_enumerated!(impl_named_bundle, 0, 16, F, T);
