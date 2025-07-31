mod map_id {
    #[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
    pub struct MapId {
        #[cfg(debug_assertions)]
        map_id: crate::uid::Uid,
        _private_field: (),
    }
}

use std::{ collections::HashMap, marker::PhantomData, ops::{ Index, IndexMut } };

use crate::*;
use map_id::*;
use derive_where::derive_where;

#[derive_where(Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Handle<T> {
    id: Uid,
    map_id: MapId,
    _phantom: PhantomData<*const T>,
}

#[derive(Debug, Clone)]
#[derive_where(Default)]
pub struct HandleMap<T> {
    map_id: MapId,
    // FIXME: Can and probably should be replaced with a sparse index map
    values: HashMap<Uid, T>,
}

impl<T> HandleMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn handle(&self, uid: Uid) -> Handle<T> {
        Handle {
            id: uid,
            map_id: self.map_id,
            _phantom: default(),
        }
    }

    fn assert_handle(&self, handle: Handle<T>) {
        assert_eq!(self.map_id, handle.map_id);
    }

    pub fn reserve(&self) -> Handle<T> {
        let uid = Uid::new();
        self.handle(uid)
    }

    pub fn insert(&mut self, val: T) -> Handle<T> {
        let uid = Uid::new();
        self.values.insert(uid, val);
        self.handle(uid)
    }

    pub fn replace(&mut self, handle: Handle<T>, val: T) -> Option<T> {
        self.assert_handle(handle);
        use std::collections::hash_map::Entry;
        match self.values.entry(handle.id) {
            Entry::Occupied(mut entry) => {
                Some(std::mem::replace(entry.get_mut(), val))
            },
            Entry::Vacant(entry) => {
                entry.insert(val);
                None
            },
        }
    }

    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        self.assert_handle(handle);
        self.values.remove(&handle.id)
    }

    pub fn contains(&self, handle: Handle<T>) -> bool {
        handle.map_id == self.map_id && self.values.contains_key(&handle.id)
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.assert_handle(handle);
        self.values.get(&handle.id)
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.assert_handle(handle);
        self.values.get_mut(&handle.id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> + ExactSizeIterator + '_ {
        self.values.iter()
            .map(|(id, val)| (self.handle(*id), val))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<T>, &mut T)> + '_ {
        let map_id = self.map_id;
        self.values.iter_mut()
            .map(move |(&id, val)| (Handle { id, map_id, _phantom: default() }, val))
    }

    pub fn retain(&mut self, mut f: impl FnMut(Handle<T>, &mut T) -> bool) {
        let map_id = self.map_id;
        self.values.retain(|&id, val| f(Handle { id, map_id, _phantom: default() }, val));
    }
}

impl<T> Index<Handle<T>> for HandleMap<T> {
    type Output = T;

    fn index(&self, handle: Handle<T>) -> &Self::Output {
        self.get(handle).expect("Unknown handle")
    }
}

impl<T> IndexMut<Handle<T>> for HandleMap<T> {
    fn index_mut(&mut self, handle: Handle<T>) -> &mut Self::Output {
        self.get_mut(handle).expect("Unknown handle")
    }
}
