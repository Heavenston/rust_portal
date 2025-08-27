//! This module defines the [`ComponentDenseStorage`] type, for use as a
//! SparseSet dense storage. (`SparseSet::<ComponentDenseStorage>`)
//!
//! Its role is to store a list of lists of components.

use super::{ Component };
use crate::{dyn_option::DynOption, sparse_set::{
    SparseSetDenseStorage, SparseSetDenseStorageInput,
}};

use std::{
    iter::{ empty, once },
    marker::PhantomData,
    ops::{ Deref, DerefMut }
};

use derive_more::From;
use dynvec::{ DynVec, OwnedDynVecValue };
use utils::{ itertools::{ chain, zip_eq }, prelude::* };

pub struct StorageComponentsRef<'a> {
    idx: usize,
    storage: &'a ComponentDenseStorage,
}

impl<'a> StorageComponentsRef<'a> {
    pub fn typed<C>(self, component_idx: usize) -> TypedComponentRef<'a, C> {
        TypedComponentRef {
            storage_ref: self,
            component_idx,
            _data: PhantomData,
        }
    }
}

pub struct TypedComponentRef<'a, C> {
    storage_ref: StorageComponentsRef<'a>,
    component_idx: usize,
    _data: PhantomData<*const C>,
}

impl<'a, C: Component> Deref for TypedComponentRef<'a, C> {
    type Target = C;

    fn deref(&self) -> &C {
        self.storage_ref.storage.storages[self.component_idx]
            .typed::<C>().expect("Correct type")
            .as_slice().get(self.storage_ref.idx).expect("Valid index")
    }
}

pub struct StorageComponentsRefMut<'a> {
    idx: usize,
    storage: &'a mut ComponentDenseStorage,
}

impl<'a> StorageComponentsRefMut<'a> {
    pub fn typed<C>(self, component_idx: usize) -> TypedComponentRefMut<'a, C> {
        TypedComponentRefMut {
            storage_ref: self,
            component_idx,
            _data: PhantomData,
        }
    }
}

pub struct TypedComponentRefMut<'a, C> {
    storage_ref: StorageComponentsRefMut<'a>,
    component_idx: usize,
    _data: PhantomData<*const C>,
}

impl<'a, C: Component> Deref for TypedComponentRefMut<'a, C> {
    type Target = C;

    fn deref(&self) -> &C {
        self.storage_ref.storage.storages[self.component_idx]
            .typed::<C>().expect("Correct type")
            .as_slice().get(self.storage_ref.idx).expect("Valid index")
    }
}

impl<'a, C: Component> DerefMut for TypedComponentRefMut<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.storage_ref.storage.storages[self.component_idx]
            .typed_mut::<C>().expect("Correct ype")
            .as_mut_slice().get_mut(self.storage_ref.idx).expect("Valid index")
    }
}

#[derive_where::derive_where(Debug)]
#[derive(Default)]
pub struct ComponentDenseStorage {
    len: usize,
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
    type OwnedOutput<'a> = impl Iterator<Item = OwnedDynVecValue<'a>>
        where Self: 'a;
    type RefItem<'a> = StorageComponentsRef<'a>
        where Self: 'a;
    type RefMutItem<'a> = StorageComponentsRefMut<'a>
        where Self: 'a;

    fn len(&self) -> usize {
        debug_assert!(
            chain(once(self.len), self.storages.iter().map(|storage| storage.len()))
                .all_equal()
        );
        self.len
    }

    fn swap_remove(&mut self, idx: usize) -> Self::OwnedOutput<'_> {
        self.len -= 1;
        self.storages.iter_mut()
            .map(move |storage| storage.swap_remove(idx).expect("Valid index"))
            .consume_on_drop()
    }

    fn get(&self, idx: usize) -> Option<Self::RefItem<'_>> {
        (idx < self.len)
            .then(|| StorageComponentsRef {
                idx,
                storage: self,
            })
    }

    fn get_mut(&mut self, idx: usize) -> Option<Self::RefMutItem<'_>> {
        (idx < self.len)
            .then(|| StorageComponentsRefMut {
                idx,
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

    fn set(&mut self, idx: usize, comps: I) {
        assert!(idx < self.len);
        for (storage, comp) in zip_eq(self.storages.iter_mut(), comps) {
            match comp.into() {
                ComponentDenseStorageInput::DynVecValue(dyn_vec_value) => {
                    dyn_vec_value.set_into(storage, idx).expect("Correct type");
                },
                ComponentDenseStorageInput::DynOption(dyn_option) => {
                    dyn_option.take_and_set_into(idx, storage)
                        .expect("Not alredy taken")
                        .expect("Correct type");
                },
            }
        }
    }
}
