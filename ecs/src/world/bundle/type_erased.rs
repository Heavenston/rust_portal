use super::*;

pub struct TypeErasedBundle<T>(pub T);

impl<const N: usize> Bundle for TypeErasedBundle<[(ComponentEntity, ComponentInputDefaultOrNot<'_>); N]> {
    fn len(&self) -> usize {
        N
    }

    fn accumulate_components(&self, _world: &mut World) -> impl AsRef<[ComponentEntity]> + use<N> {
        self.0.each_ref().map(|&(e, _)| e)
    }

    fn into_component_values(self) -> impl BundleValueIterable {
        struct Test<'a, const N: usize> {
            values: Option<[ComponentInputDefaultOrNot<'a>; N]>,
        }

        impl<const N: usize> BundleValueIterable for Test<'_, N> {
            type Item<'a> = ComponentInputDefaultOrNot<'a>
                where Self: 'a;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                self.values.take().unwrap().into_iter()
            }
        }

        Test {
            values: Some(self.0.map(|(_, a)| a))
        }
    }
}

impl Bundle for TypeErasedBundle<Vec<(ComponentEntity, ComponentInputDefaultOrNot<'_>)>> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn accumulate_components(&self, _world: &mut World) -> impl AsRef<[ComponentEntity]> + use<> {
        self.0.iter().map(|&(e, _)| e).collect_vec()
    }

    fn into_component_values(self) -> impl BundleValueIterable {
        struct Test<'a> {
            values: Vec<(ComponentEntity, ComponentInputDefaultOrNot<'a>)>,
        }

        impl BundleValueIterable for Test<'_> {
            type Item<'a> = ComponentInputDefaultOrNot<'a>
                where Self: 'a;

            fn bundle_values_iter(&mut self) -> impl Iterator<Item = Self::Item<'_>> {
                std::mem::take(&mut self.values).into_iter().map(|(_, val)| val)
            }
        }

        Test {
            values: self.0,
        }
    }
}
