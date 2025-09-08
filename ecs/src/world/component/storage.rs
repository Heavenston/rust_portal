//! This module defines the [`ComponentDenseStorage`] type, for use as a
//! SparseSet dense storage. (`SparseSet::<ComponentDenseStorage>`)
//!
//! Its role is to store a list of lists of components.

use crate::{
    dyn_option::DynOption,
    sparse_set::{
        SparseSetDenseStorage, SparseSetDenseStorageInput,
    },
    world::EntityIndex,
};

use std::iter::{ empty, once };

use utils::{ itertools::{ chain, zip_eq }, prelude::* };
use derive_more::From;
use dynvec::{ DrainedDynVecValue, DynVec, DynVecDrain, IncorrectTypeError, InsertionError, RemovedDynVecValue };

pub struct StorageComponentsRef<'a> {
    idx: usize,
    storage: &'a ComponentDenseStorage,
}

impl<'a> StorageComponentsRef<'a> {
    pub fn for_component(self, component_idx: usize) -> dynvec::DynVecValueRef<'a> {
        self.storage.storages[component_idx]
            .get(self.idx).expect("Valid index")
    }
}

pub struct StorageComponentsRefMut<'a> {
    idx: usize,
    storage: &'a mut ComponentDenseStorage,
}

impl<'a> StorageComponentsRefMut<'a> {
    pub fn for_component(self, component_idx: usize) -> dynvec::DynVecValueRefMut<'a> {
        self.storage.storages[component_idx]
            .get_mut(self.idx).expect("Valid index")
    }
}

#[derive_where::derive_where(Debug)]
#[derive(TryClone, Default)]
pub struct ComponentDenseStorage {
    #[try_clone(use_clone)]
    len: u32,
    #[derive_where(skip)]
    #[try_clone(error_type = "dynvec::NoCloneError", clone_with = try_clone_boxed_slice)]
    storages: Box<[DynVec]>,
}

impl ComponentDenseStorage {
    pub fn new(storages: Box<[DynVec]>) -> Self {
        Self {
            len: 0,
            storages,
        }
    }

    pub fn debug_types(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for storage in &self.storages {
            list.entry(&storage.metadata().type_name);
        }
        list.finish()?;
        Ok(())
    }

    pub fn drain(&mut self) -> impl Iterator<Item = DynVecDrain<'_>> {
        self.storages.iter_mut()
            .map(move |storage| storage.drain())
            .consume_on_drop()
    }

    pub fn remove_column(&mut self, component_idx: usize) {
        let mut storages = std::mem::take(&mut self.storages).into_vec();
        storages.remove(component_idx);
        self.storages = storages.into_boxed_slice();
    }
}

impl SparseSetDenseStorage for ComponentDenseStorage {
    type SparseIdx = EntityIndex;
    type DenseIdx = u32;
    type PrimitiveDenseIdx = u32;

    type RemovedOutput<'a> = impl Iterator<Item = RemovedDynVecValue<'a>>
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

    fn swap_remove(&mut self, idx: u32) -> Self::RemovedOutput<'_> {
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
        todo!();
        empty()
    }
}

#[derive(From)]
pub enum ComponentDenseStorageInput<'a, 'b> {
    RemovedDynVecValue(RemovedDynVecValue<'a>),
    DrainedDynVecValue(DrainedDynVecValue<'a>),
    DynOption(&'b mut dyn DynOption),
    Default,
}

impl<'a, 'b, T, I> SparseSetDenseStorageInput<I> for ComponentDenseStorage
    where I: IntoIterator<Item = T>,
          T: Into<ComponentDenseStorageInput<'a, 'b>>,
{
    fn push(&mut self, comps: I) {
        self.len += 1;
        for (i, (storage, comp)) in zip_eq(self.storages.iter_mut(), comps).enumerate() {
            let type_result: Result<(), IncorrectTypeError> = match comp.into() {
                ComponentDenseStorageInput::RemovedDynVecValue(dyn_vec_value) => {
                    dyn_vec_value.push_into(storage)
                },
                ComponentDenseStorageInput::DrainedDynVecValue(dyn_vec_value) => {
                    dyn_vec_value.push_into(storage)
                },
                ComponentDenseStorageInput::DynOption(dyn_option) => {
                    dyn_option.take_and_push_into(storage)
                        .expect("Not alredy taken")
                },
                ComponentDenseStorageInput::Default => {
                    storage.push_default()
                        .expect("Type should have a default constructor if ComponentDenseStorageInput::Default is used");
                    Ok(())
                },
            };

            if let Err(err) = type_result {
                panic!("Error pushing component {i} (with {:?}): {err}", std::fmt::from_fn(|f| self.debug_types(f)));
            }
        }
    }

    fn set(&mut self, idx: u32, comps: I) {
        assert!(idx < self.len);
        for (i, (storage, comp)) in zip_eq(self.storages.iter_mut(), comps).enumerate() {
            let type_result: Result<(), InsertionError> = match comp.into() {
                ComponentDenseStorageInput::RemovedDynVecValue(dyn_vec_value) => {
                    dyn_vec_value.set_into(storage, ix!(idx))
                },
                ComponentDenseStorageInput::DrainedDynVecValue(dyn_vec_value) => {
                    dyn_vec_value.set_into(storage, ix!(idx))
                },
                ComponentDenseStorageInput::DynOption(dyn_option) => {
                    dyn_option.take_and_set_into(ix!(idx), storage)
                        .expect("Not alredy taken")
                },
                ComponentDenseStorageInput::Default => {
                    storage.set_default(ix!(idx))
                        .expect("Type should have a default constructor if ComponentDenseStorageInput::Default is used");
                    Ok(())
                }
            };

            if let Err(err) = type_result {
                panic!("Error setting component {i} (with {:?}): {err}", std::fmt::from_fn(|f| self.debug_types(f)));
            }
        }
    }
}
