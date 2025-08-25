mod entity_storage;
pub use entity_storage::{ Entity, EntityIndexType, EntityGenerationType };
mod entity_set;
pub use entity_set::{ EntitySet };
mod component_vec;
pub use component_vec::{ ComponentVec };

use std::{ alloc::Layout, any::{Any, TypeId}, borrow::Cow, collections::HashMap, iter::{empty, once, zip}, marker::PhantomData, ops::{Deref, DerefMut} };
use derive_more::{ From, Into };

use utils::{ itertools::chain, prelude::* };

use crate::{
    index_map::{ IndexMap, IndexMapIndex },
    sparse_set::{ SparseSet, SparseSetDenseStorage }, world::component_vec::ComponentVecFactory
};

pub trait Component: 'static + Any { }
impl<T: 'static> Component for T { }

const RESERVED_ENTITY_COUNT: u32 = 100;

macro_rules! create_id {
    ($name: ident($ty: ty)) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into)]
        struct $name(pub $ty);

        /// Provide a more-than-surely invalid default value
        impl Default for $name {
            fn default() -> Self {
                let t: $ty = 0;
                Self(!t)
            }
        }

        impl IndexMapIndex for $name {
            fn from_usize(usize: usize) -> Self {
                Self(usize.try_into().expect("No overflow"))
            }

            fn to_usize(&self) -> usize {
                ix!(self.0)
            }
        }
    };
}

/// Component given to all entities of components
#[derive(Clone)]
pub struct ComponentComponent {
    pub type_id: TypeId,
    pub layout: Layout,
    pub componentvec_factory: Box<dyn ComponentVecFactory>,
}

create_id!(ArchetypId(u32));

#[derive(Default, Debug, Clone)]
struct Archetyp {
    components: EntitySet,
    /// Table used for storing the table components of this archetyp.
    /// Multiple Archtyps could point to the same tables if they have the same
    /// (table) components.
    table_id: TableId,
}

struct StorageComponentsRef<'a> {
    idx: usize,
    storage: &'a ComponentDenseStorage,
}

impl<'a> StorageComponentsRef<'a> {
    fn typed<C>(self, component_idx: usize) -> TypedComponentRef<'a, C> {
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
        let dyn_ref = self.storage_ref.storage.storages[self.component_idx]
            .dyn_get(self.storage_ref.idx).expect("Valid idx");
        (dyn_ref as &dyn Any).downcast_ref().expect("Correct type")
    }
}

struct StorageComponentsRefMut<'a> {
    idx: usize,
    storage: &'a mut ComponentDenseStorage,
}

impl<'a> StorageComponentsRefMut<'a> {
    fn typed<C>(self, component_idx: usize) -> TypedComponentRefMut<'a, C> {
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
        let dyn_ref = self.storage_ref.storage.storages[self.component_idx]
            .dyn_get(self.storage_ref.idx).expect("Valid idx");
        (dyn_ref as &dyn Any).downcast_ref().expect("Correct type")
    }
}

impl<'a, C: Component> DerefMut for TypedComponentRefMut<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let dyn_ref = self.storage_ref.storage.storages[self.component_idx]
            .dyn_get_mut(self.storage_ref.idx).expect("Valid idx");
        (dyn_ref as &mut dyn Any).downcast_mut().expect("Correct type")
    }
}

#[derive_where::derive_where(Debug)]
#[derive(Default)]
struct ComponentDenseStorage {
    len: usize,
    #[derive_where(skip)]
    storages: Box<[Box<dyn ComponentVec>]>,
}

impl SparseSetDenseStorage for ComponentDenseStorage {
    type OwnedItem = Box<[Box<dyn Component>]>;
    type RefItem<'a> = StorageComponentsRef<'a>
        where Self: 'a;
    type RefMutItem<'a> = StorageComponentsRefMut<'a>
        where Self: 'a;

    fn len(&self) -> usize {
        debug_assert!(
            chain(once(self.len), self.storages.iter().map(|storage| storage.dyn_len()))
                .all_equal()
        );
        self.len
    }

    fn push(&mut self, val: Self::OwnedItem) {
        for (storage, comp) in zip(self.storages.iter_mut(), val) {
            storage.dyn_push(comp);
        }
    }

    fn swap_remove(&mut self, idx: usize) -> Self::OwnedItem {
        self.storages.iter_mut()
            .map(|storage| storage.dyn_swap_remove(idx))
            .collect_vec()
            .into_boxed_slice()
    }

    fn set(&mut self, idx: usize, value: Self::OwnedItem) {
        assert!(idx < self.len);
        for (storage, comp) in zip(self.storages.iter_mut(), value) {
            storage.dyn_set(idx, comp);
        }
    }

    fn get(&self, idx: usize) -> Option<Self::RefItem<'_>> {
        (idx >= self.len)
            .then(|| StorageComponentsRef {
                idx,
                storage: self,
            })
    }

    fn get_mut(&mut self, idx: usize) -> Option<Self::RefMutItem<'_>> {
        (idx >= self.len)
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

create_id!(TableId(u32));

#[derive(Default, Debug)]
struct Table {
    table_components: EntitySet,
    sparse_set: SparseSet<ComponentDenseStorage>,
}

pub struct AddComponent<'a, C> {
    pub r#ref: TypedComponentRefMut<'a, C>,
    pub was_added: bool,
}

#[derive(Debug)]
pub struct World {
    entity_storage: entity_storage::EntityStorage,

    /// Maps entity index to archetyp id
    entities_archetypes: IndexMap<ArchetypId, EntityIndexType>,

    /// List of all archetypes indexed by their ids
    archetypes: IndexMap<Archetyp, ArchetypId>,
    /// List of tables indexed by their ids
    tables: IndexMap<Table, TableId>,

    components_typeid_to_entity: HashMap<TypeId, Entity>,
    components_set_to_archetyp: HashMap<EntitySet, ArchetypId>,
    components_set_to_table: HashMap<EntitySet, TableId>,
}

impl World {
    pub fn new() -> Self {
        let mut this = Self {
            entity_storage: entity_storage::EntityStorage::new(RESERVED_ENTITY_COUNT),

            entities_archetypes: default(),

            archetypes: default(),
            tables: default(),

            components_typeid_to_entity: default(),
            components_set_to_archetyp: default(),
            components_set_to_table: default(),
        };

        // make sure all reserved entities will immediately be valid
        let empty_archtyp = this.archtyp_for(Cow::Owned(EntitySet::default()));
        for reserved in this.entity_storage.reserved_entities() {
            this.entities_archetypes.set_or_push(reserved.index(), empty_archtyp);
        }

        this
    }

    pub fn alive(&self, entity: Entity) -> bool {
        self.entity_storage.alive(entity)
    }

    pub fn spawn(&mut self) -> Entity {
        let empty_archetype = self.archtyp_for(Cow::Owned(EntitySet::default()));
        let entity = self.entity_storage.spawn();
        self.entities_archetypes.set_or_push(entity.index(), empty_archetype);
        entity
    }

    pub fn spawn_many(&mut self, amount: u32) -> impl Iterator<Item = Entity> {
        self.entity_storage.spawn_many(amount)
    }

    /// Returns false if the entity wasn't alive already
    pub fn dispawn(&mut self, entity: Entity) -> bool {
        self.entity_storage.dispawn(entity)
    }

    pub fn try_component<C: Component>(&self) -> Option<Entity> {
        let type_id = TypeId::of::<C>();
        self.components_typeid_to_entity.get(&type_id)
            .copied()
    }

    /// Returns the entity of the given component type.
    /// Registering it if not already done.
    pub fn component<C: Component>(&mut self) -> Entity {
        let type_id = TypeId::of::<C>();

        use std::collections::hash_map::Entry;
        let created_entity = match self.components_typeid_to_entity.entry(type_id) {
            Entry::Occupied(o) => return *o.get(),
            Entry::Vacant(vacant) => {
                let entity = self.entity_storage.take_next_reserved()
                    .unwrap_or_else(|| self.entity_storage.spawn());
                vacant.insert(entity);
                entity
            },
        };

        self.add(created_entity, ComponentComponent {
            type_id,
            layout: Layout::new::<C>(),
            componentvec_factory: Box::new(|| -> Box<dyn ComponentVec> { Box::new(Vec::<C>::new()) }) as Box<_>,
        });

        created_entity
    }

    fn table_for(&mut self, table_components: Cow<'_, EntitySet>) -> TableId {
        match self.components_set_to_table.get(table_components.as_ref()) {
            Some(&table_id) => table_id,
            None => {
                let comp_set = table_components.into_owned();
                let table_id = self.tables.push(Table {
                    table_components: comp_set.clone(),
                    sparse_set: SparseSet::new(ComponentDenseStorage::default()),
                });

                self.components_set_to_table.insert(comp_set, table_id);

                table_id
            },
        }
    }

    fn archtyp_for(&mut self, component_set: Cow<'_, EntitySet>) -> ArchetypId {
        match self.components_set_to_archetyp.get(component_set.as_ref()) {
            Some(&id) => id,
            None => {
                let component_set = component_set.into_owned();
                // FIXME: When table components are added this should filters 
                // to only the table components
                let table_components = component_set.clone();
                let table_id = self.table_for(Cow::Borrowed(&table_components));
                let archetyp_id = self.archetypes.push(Archetyp {
                    components: component_set.clone(),
                    table_id,
                });

                self.components_set_to_archetyp.insert(component_set, archetyp_id);

                archetyp_id
            },
        }
    }

    pub fn has<C: Component>(&self, entity: Entity) -> bool {
        if !self.entity_storage.alive(entity)
        { return false; }
        let Some(component) = self.try_component::<C>()
        else { return false };
        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];
        archetyp.components.has(component)
    }

    pub fn get<C: Component>(&self, entity: Entity) -> Option<TypedComponentRef<'_, C>> {
        let component = self.try_component::<C>()?;

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];
        let table_id = archetyp.table_id;
        let table = &self.tables[table_id];
        let component_idx = table.table_components.index_of(component)?;
        table.sparse_set.get(entity.index())
            .map(|p| p.typed(component_idx))
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Option<TypedComponentRefMut<'_, C>> {
        let component = self.try_component::<C>()?;

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];
        let table_id = archetyp.table_id;
        let table = &mut self.tables[table_id];
        let component_idx = table.table_components.index_of(component)?;
        table.sparse_set.get_mut(entity.index())
            .map(|p| p.typed(component_idx))
    }

    pub fn get_or_default<C: Component + Default>(&mut self, entity: Entity) -> AddComponent<'_, C> {
        self.add_with(entity, default)
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: Entity, component: C) -> AddComponent<'_, C> {
        self.add_with(entity, || component)
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: Entity, f: F) -> AddComponent<'_, C>
        where F: FnOnce() -> C,
              C: Component,
    {
        let component = self.component::<C>();

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];
        if archetyp.components.has(component) {
            let table_id = archetyp.table_id;
            let table = &mut self.tables[table_id];
            let component_idx = table.table_components.index_of(component)
                .expect("is in table");
            let r#ref = table.sparse_set.get_mut(entity.index())
                .expect("entity is in table")
                .typed::<C>(component_idx);
            return AddComponent {
                r#ref,
                was_added: false,
            };
        }

        // we have to change the entity's archetyp

        let (new_component_idx, new_component_set) = archetyp.components.clone().with(component);
        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_table_id = self.archetypes[old_archetyp_id].table_id;
        let new_archtyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        let new_table_id = self.archetypes[new_archtyp_id].table_id;

        // FIXME: At the time of writing this only removing/adding entities from tables 
        // was required when changing archetyps, adding non-table components will
        // change this
        // (and both tables could be the same)

        let mut old_components = self.tables[old_table_id].sparse_set.remove(entity.index())
            .expect("entity is in table").into_vec();
        old_components.insert(new_component_idx, Box::new(f()) as Box<dyn Component>);

        let new_table = &mut self.tables[new_table_id];
        new_table.sparse_set.insert(
            entity.index(), old_components.into_boxed_slice()
        );

        let r#ref = new_table.sparse_set.get_mut(entity.index()).expect("Entity just inserted")
            .typed(new_component_idx);

        AddComponent {
            r#ref,
            was_added: true,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
