//! This module defines the [`ComponentDenseStorage`] type, for use as a
//! SparseSet dense storage. (`SparseSet::<ComponentDenseStorage>`)
//!
//! Its role is to store a list of lists of components.

use crate::{
    component::Component,
    dyn_option::DynOption,
    sparse_set::{
        SparseSetDenseStorage, SparseSetDenseStorageInput,
    }, world::EntityIndex,
};

use std::iter::{ empty, once };

use utils::{ itertools::{ chain, zip_eq }, prelude::* };
use derive_more::From;
use dynvec::{ DynVec, OwnedDynVecValue };

pub struct StorageComponentsRef<'a> {
    idx: usize,
    storage: &'a ComponentDenseStorage,
}

impl<'a> StorageComponentsRef<'a> {
    pub fn typed<C: Component>(self, component_idx: usize) -> &'a C {
        self.storage.storages[component_idx]
            .typed::<C>().expect("Correct type")
            .as_slice().get(self.idx).expect("Valid index")
    }
}

pub struct StorageComponentsRefMut<'a> {
    idx: usize,
    storage: &'a mut ComponentDenseStorage,
}

impl<'a> StorageComponentsRefMut<'a> {
    pub fn typed<C: Component>(self, component_idx: usize) -> &'a mut C {
        self.storage.storages[component_idx]
            .typed_mut::<C>().expect("Correct ype")
            .as_mut_slice().get_mut(self.idx).expect("Valid index")
    }
}

#[derive_where::derive_where(Debug)]
#[derive(Default)]
pub struct ComponentDenseStorage {
    len: u32,
    #[derive_where(skip)]
    storages: Box<[DynVec]>,
}

impl ComponentDenseStorage {
    pub fn new(storages: Box<[DynVec]>) -> Self {
        Self {
            len: 0,
            storages,
        }
    }
}

impl SparseSetDenseStorage for ComponentDenseStorage {
    type SparseIdx = EntityIndex;
    type DenseIdx = u32;
    type PrimitiveDenseIdx = u32;

    type OwnedOutput<'a> = impl Iterator<Item = OwnedDynVecValue<'a>>
        where Self: 'a;
    type RefItem<'a> = StorageComponentsRef<'a>
        where Self: 'a;
    type RefMutItem<'a> = StorageComponentsRefMut<'a>
        where Self: 'a;

    fn len(&self) -> u32 {
        debug_assert!(
            chain(once(ix!(self.len)), self.storages.iter().map(|storage| storage.len()))
                .all_equal()
        );
        self.len
    }

    fn swap_remove(&mut self, idx: u32) -> Self::OwnedOutput<'_> {
        self.len -= 1;
        self.storages.iter_mut()
            .map(move |storage| storage.swap_remove(ix!(idx)).expect("Valid index"))
            .consume_on_drop()
    }

    fn get(&self, idx: u32) -> Option<Self::RefItem<'_>> {
        (idx < self.len)
            .then(|| StorageComponentsRef {
                idx: ix!(idx),
                storage: self,
            })
    }

    fn get_mut(&mut self, idx: u32) -> Option<Self::RefMutItem<'_>> {
        (idx < self.len)
            .then(|| StorageComponentsRefMut {
                idx: ix!(idx),
                storage: self,
            })
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::RefItem<'a>> + DoubleEndedIterator + ExactSizeIterator + Clone {
        (0..self.len)
            .map(|idx| self.get(idx).expect("in bound"))
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = Self::RefMutItem<'a>> + DoubleEndedIterator + ExactSizeIterator {
        // TODO: Implement (do i need unsafe ?? x( )
        empty()
    }
}

#[derive(From)]
pub enum ComponentDenseStorageInput<'a> {
    DynVecValue(OwnedDynVecValue<'a>),
    DynOption(&'a mut dyn DynOption),
}

impl<'a, T, I> SparseSetDenseStorageInput<I> for ComponentDenseStorage
    where I: IntoIterator<Item = T>,
          T: Into<ComponentDenseStorageInput<'a>>,
{
    fn push(&mut self, comps: I) {
        self.len += 1;
        for (storage, comp) in zip_eq(self.storages.iter_mut(), comps) {
            match comp.into() {
                ComponentDenseStorageInput::DynVecValue(dyn_vec_value) => {
                    dyn_vec_value.push_into(storage).expect("Correct type");
                },
                ComponentDenseStorageInput::DynOption(dyn_option) => {
                    dyn_option.take_and_push_into(storage)
                        .expect("Not alredy taken")
                        .expect("Correct type");
                },
            }
        }
    }

    fn set(&mut self, idx: u32, comps: I) {
        assert!(idx < self.len);
        for (storage, comp) in zip_eq(self.storages.iter_mut(), comps) {
            match comp.into() {
                ComponentDenseStorageInput::DynVecValue(dyn_vec_value) => {
                    dyn_vec_value.set_into(storage, ix!(idx)).expect("Correct type");
                },
                ComponentDenseStorageInput::DynOption(dyn_option) => {
                    dyn_option.take_and_set_into(ix!(idx), storage)
                        .expect("Not alredy taken")
                        .expect("Correct type");
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ ComponentDenseStorage, StorageComponentsRefMut };
    use dynvec::DynVec;
    use crate::sparse_set::{ SparseSetDenseStorage, SparseSetDenseStorageInput };

    #[test]
    fn push_get_set_iter_and_swap_remove_cover_paths() {
        // Two component columns: i32 and &str
        let mut storage = ComponentDenseStorage::new(vec![
            DynVec::new::<i32>(),
            DynVec::new::<&'static str>(),
        ].into_boxed_slice());

        // Push one row via DynOption inputs
        let mut c0 = Some(1);
        let mut c1 = Some("a");
        storage.push([
            &mut c0 as &mut dyn crate::dyn_option::DynOption,
            &mut c1 as &mut dyn crate::dyn_option::DynOption,
        ]);
        assert_eq!(storage.len(), 1);

        // get and get_mut accessors
        assert_eq!(*storage.get(0).unwrap().typed::<i32>(0), 1);
        assert_eq!(*storage.get(0).unwrap().typed::<&'static str>(1), "a");

        let row0m: StorageComponentsRefMut<'_> = storage.get_mut(0).unwrap();
        *row0m.typed::<i32>(0) = 2;
        assert_eq!(storage.get(0).unwrap().typed::<i32>(0), &2);

        // iter returns exactly one item
        let collected: Vec<_> = storage.iter().collect();
        assert_eq!(collected.len(), 1);

        // iter_mut is currently unimplemented and returns empty iterator
        assert_eq!(storage.iter_mut().count(), 0);

        // set overwrites values
        let mut d0 = Some(3);
        let mut d1 = Some("b");
        storage.set(0, [
            &mut d0 as &mut dyn crate::dyn_option::DynOption,
            &mut d1 as &mut dyn crate::dyn_option::DynOption,
        ]);
        assert_eq!(storage.get(0).unwrap().typed::<i32>(0), &3);
        assert_eq!(storage.get(0).unwrap().typed::<&'static str>(1), &"b");

        // swap_remove drops row and decreases len
        let _ = storage.swap_remove(0);
        assert_eq!(storage.len(), 0);
        assert!(storage.get(0).is_none());
    }

    #[test]
    fn set_with_owned_dynvec_values_path() {
        // build owned values by using temporary DynVecs and swap_remove
        let mut s0 = DynVec::new::<i64>();
        let mut s1 = DynVec::new::<&'static str>();
        s0.typed_mut::<i64>().unwrap().push(9);
        s1.typed_mut::<&'static str>().unwrap().push("x");
        let v0 = s0.swap_remove(0).unwrap();
        let v1 = s1.swap_remove(0).unwrap();

        let mut storage = ComponentDenseStorage::new(vec![
            DynVec::new::<i64>(),
            DynVec::new::<&'static str>(),
        ].into_boxed_slice());
        // push a placeholder row first so that set operates in-bounds
        let mut p0 = Some(0i64);
        let mut p1 = Some("_");
        storage.push([
            &mut p0 as &mut dyn crate::dyn_option::DynOption,
            &mut p1 as &mut dyn crate::dyn_option::DynOption,
        ]);

        // Now set using OwnedDynVecValue branch
        storage.set(0, [v0, v1]);
        assert_eq!(*storage.get(0).unwrap().typed::<i64>(0), 9);
        assert_eq!(*storage.get(0).unwrap().typed::<&'static str>(1), "x");
    }
}
