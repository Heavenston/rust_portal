use std::any::TypeId;

use crate::component::Component;

pub trait Bundle {
    fn type_ids() -> impl Iterator<Item = TypeId> + ExactSizeIterator;

    fn len() -> usize {
        Self::type_ids().len()
    }
}

macro_rules! bundle_impl {
    ($($name: ident),*) => {
        impl<$($name),*> Bundle for ($($name,)*)
            where $($name: Component,)*
        {
            fn type_ids() -> impl Iterator<Item = TypeId> + ExactSizeIterator {
                [
                    $(TypeId::of::<$name>(),)*
                ].into_iter()
            }

            fn len() -> usize {
                ${count($name)}
            }
        }
    };
}

bundle_impl!(A);
bundle_impl!(A, B);
bundle_impl!(A, B, C);
bundle_impl!(A, B, C, D);
bundle_impl!(A, B, C, D, E);
bundle_impl!(A, B, C, D, E, F);
bundle_impl!(A, B, C, D, E, F, G);
bundle_impl!(A, B, C, D, E, F, G, H);
bundle_impl!(A, B, C, D, E, F, G, H, I);
bundle_impl!(A, B, C, D, E, F, G, H, I, J);
