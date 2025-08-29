#[cfg(test)]
mod tests;

mod entity_storage;
pub use entity_storage::{ Entity, EntityIndex, EntityGeneration };
mod entity_set;
pub use entity_set::{ EntitySet, ComponentSet };
pub mod component;
use component::*;
mod query;
pub use query::*;
mod bundle;
pub use bundle::*;

use crate::{
    dyn_option::DynOption,
    index_map::IndexMap,
    sparse_set::SparseSet,
};

use std::{
    any::{ type_name, TypeId },
    borrow::Cow,
    collections::HashMap,
    iter::{ empty, once },
};
use utils::prelude::*;
use derive_more::{ From, Into, IsVariant };
use dynvec::{ DynVec, DynVecMetadata };

const RESERVED_ENTITY_COUNT: u32 = 100;

macro_rules! create_id {
    ($struct_vis: vis $name: ident($in_vis:vis $ty: ty)) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into)]
        $struct_vis struct $name($in_vis $ty);

        /// Provide a more-than-surely invalid default value
        impl Default for $name {
            fn default() -> Self {
                let t: $ty = 0;
                Self(!t)
            }
        }

        impl TryFrom<usize> for $name {
            type Error = <$ty as TryFrom<usize>>::Error;

            fn try_from(from: usize) -> Result<$name, Self::Error> {
                Ok($name(<$ty>::try_from(from)?))
            }
        }

        impl TryFrom<$name> for usize {
            type Error = <usize as TryFrom<$ty>>::Error;

            fn try_from(from: $name) -> Result<usize, Self::Error> {
                Ok(usize::try_from(from.0)?)
            }
        }
    };
}

create_id!(ArchetypId(u32));

#[derive(Default, Debug, Clone)]
struct Archetyp {
    components: ComponentSet,
    /// Table used for storing the table components of this archetyp.
    /// Multiple Archtyps could point to the same tables if they have the same
    /// (table) components.
    table_id: TableId,
}

create_id!(TableId(u32));

#[derive(Default, Debug)]
struct Table {
    table_components: ComponentSet,
    sparse_set: SparseSet<ComponentDenseStorage>,
}

pub struct AddComponent<'a, C> {
    pub r#ref: &'a mut C,
    pub was_added: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IsVariant)]
pub enum HasComponent {
    /// The entity is dead
    EntityIsNotAlive,
    /// The component was never registred
    UnknownComponent,
    /// The component is not present in this entity's archetyp
    NotPresent,
    /// The component *is* present in this entity's archtyp
    Present,
}

impl HasComponent {
    pub fn bool(self) -> bool {
        self.is_present()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetComponentError {
    #[error("Tried to get component of dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Component from type '{type_name}' was never registred")]
    UnknownComponent {
        type_name: &'static str,
    },
    #[error("Component from type '{type_name}' is not present in the entity {entity}")]
    ComponentNotPresent {
        type_name: &'static str,
        entity: Entity,
    },
}

#[derive(Debug)]
pub struct World {
    entity_storage: entity_storage::EntityStorage,

    /// Maps entity index to archetyp id
    entities_archetypes: IndexMap<ArchetypId, EntityIndex>,

    /// List of all archetypes indexed by their ids
    archetypes: IndexMap<Archetyp, ArchetypId>,
    /// List of tables indexed by their ids
    tables: IndexMap<Table, TableId>,

    components_typeid_to_entity: HashMap<TypeId, ComponentEntity>,
    components_set_to_archetyp: HashMap<ComponentSet, ArchetypId>,
    components_set_to_table: HashMap<ComponentSet, TableId>,
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
        let empty_archetyp = this.archtyp_for(Cow::Owned(EntitySet::default()));
        for reserved in this.entity_storage.reserved_entities() {
            this.entities_archetypes.set_or_push(reserved.index(), empty_archetyp);
            this.tables[this.archetypes[empty_archetyp].table_id]
                .sparse_set.insert(reserved.index(), empty::<ComponentDenseStorageInput>());
        }

        // Hard-code the first table for the ComponentComponent's component entity
        let cc_entity = this.entity_storage.take_next_reserved()
            .unwrap_or_else(|| this.entity_storage.spawn());
        let cc_entity = ComponentEntity(cc_entity);
        this.components_typeid_to_entity.insert(TypeId::of::<ComponentComponent>(), cc_entity);

        let cc_set = EntitySet::from(&[cc_entity][..]);
        let cc_table_id = this.tables.push(Table {
            table_components: cc_set.clone(),
            sparse_set: SparseSet::new(ComponentDenseStorage::new(
                vec![DynVec::new::<ComponentComponent>()]
                    .into_boxed_slice()
            )),
        });
        this.components_set_to_table.insert(cc_set.clone(), cc_table_id);

        let cc_archetype_id = this.archetypes.push(Archetyp {
            components: cc_set.clone(),
            table_id: cc_table_id,
        });
        this.components_set_to_archetyp.insert(cc_set.clone(), cc_archetype_id);

        // Move the entity from the empty table to the ComponentComponent table and seed its own ComponentComponent
        let empty_table_id = this.archetypes[empty_archetyp].table_id;
        let _ = this.tables[empty_table_id].sparse_set.remove(cc_entity.index());
        this.entities_archetypes.set_or_push(cc_entity.index(), cc_archetype_id);
        this.tables[cc_table_id].sparse_set.insert(cc_entity.index(), vec![
            &mut Some(ComponentComponent {
                dynvec_meta: DynVecMetadata::new::<DynVecMetadata>(),
            }) as &mut dyn DynOption
        ]);

        this
    }

    pub fn generation_at_index(&self, index: EntityIndex) -> EntityGeneration {
        self.entity_storage.generation_at_index(index)
    }

    pub fn alive(&self, entity: impl Into<Entity>) -> bool {
        self.entity_storage.alive(entity.into())
    }

    pub fn spawn(&mut self) -> Entity {
        let empty_archetyp = self.archtyp_for(Cow::Owned(EntitySet::default()));
        let entity = self.entity_storage.spawn();
        self.entities_archetypes.set_or_push(entity.index(), empty_archetyp);
        self.tables[self.archetypes[empty_archetyp].table_id]
            .sparse_set.insert(entity.index(), empty::<ComponentDenseStorageInput>());
        entity
    }

    /// Returns false if the entity was already dead.
    pub fn dispawn(&mut self, entity: impl Into<Entity>) -> bool {
        self.entity_storage.dispawn(entity.into())
    }

    /// Returns the entity for the given component type_id, or None if it was never
    /// registred.
    ///
    /// NOTE: Components cannot yet be registred through type_ids yet
    pub fn try_component_entity(&self, type_id: TypeId) -> Option<ComponentEntity> {
        self.components_typeid_to_entity.get(&type_id)
            .copied()
    }

    /// Returns the entity for the given component, or None if it was never
    /// registred.
    pub fn try_component<C: Component>(&self) -> Option<ComponentEntity> {
        self.try_component_entity(TypeId::of::<C>())
    }

    /// Returns the entity of the given component type.
    /// Registering it if not already done.
    pub fn component<C: Component>(&mut self) -> ComponentEntity {
        let type_id = TypeId::of::<C>();

        use std::collections::hash_map::Entry;
        let created_entity = match self.components_typeid_to_entity.entry(type_id) {
            Entry::Occupied(o) => return *o.get(),
            Entry::Vacant(vacant) => {
                let entity = self.entity_storage.take_next_reserved()
                    .unwrap_or_else(|| self.entity_storage.spawn());
                let entity = ComponentEntity(entity);
                vacant.insert(entity);
                entity
            },
        };

        self.add(created_entity, ComponentComponent {
            dynvec_meta: DynVecMetadata::new::<C>(),
        });

        created_entity
    }

    fn table_for(&mut self, table_components: Cow<'_, ComponentSet>) -> TableId {
        match self.components_set_to_table.get(table_components.as_ref()) {
            Some(&table_id) => table_id,
            None => {
                let comp_set = table_components.into_owned();
                let table_id = self.tables.push(Table {
                    table_components: comp_set.clone(),
                    sparse_set: SparseSet::new(ComponentDenseStorage::new(
                        comp_set.iter()
                            .map(|component| {
                                let component_metadata = self.get::<ComponentComponent>(component)
                                    .expect("Entity is component");

                                DynVec::new_with_meta(component_metadata.dynvec_meta.clone())
                            })
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    )),
                });

                self.components_set_to_table.insert(comp_set, table_id);

                table_id
            },
        }
    }

    fn archtyp_for(&mut self, component_set: Cow<'_, ComponentSet>) -> ArchetypId {
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

    pub fn has<C: Component>(&self, entity: impl Into<Entity>) -> HasComponent {
        let entity = entity.into();

        if !self.entity_storage.alive(entity)
        { return HasComponent::EntityIsNotAlive; }

        let Some(component) = self.try_component::<C>()
        else { return HasComponent::UnknownComponent; };

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];

        if archetyp.components.has(component) {
            HasComponent::Present
        }
        else {
            HasComponent::NotPresent
        }
    }

    pub fn get<C: Component>(&self, entity: impl Into<Entity>) -> Result<&C, GetComponentError> {
        let entity = entity.into();

        if !self.alive(entity)
        { return Err(GetComponentError::EntityIsNotAlive { entity }); }

        let Some(component) = self.try_component::<C>()
        else { return Err(GetComponentError::UnknownComponent {
            type_name: type_name::<C>()
        })};

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];
        let table_id = archetyp.table_id;
        let table = &self.tables[table_id];

        let Some(component_idx) = table.table_components.index_of(component)
        else { return Err(GetComponentError::ComponentNotPresent { type_name: type_name::<C>(), entity }) };

        let component = table.sparse_set.get(entity.index())
            .expect("Entity is in this table")
            .typed(component_idx);

        Ok(component)
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Result<&mut C, GetComponentError> {
        if !self.alive(entity)
        { return Err(GetComponentError::EntityIsNotAlive { entity }); }

        let Some(component) = self.try_component::<C>()
        else { return Err(GetComponentError::UnknownComponent {
            type_name: type_name::<C>()
        })};

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];
        let table_id = archetyp.table_id;
        let table = &mut self.tables[table_id];

        let Some(component_idx) = table.table_components.index_of(component)
        else { return Err(GetComponentError::ComponentNotPresent { type_name: type_name::<C>(), entity }) };

        let component = table.sparse_set.get_mut(entity.index())
            .expect("Entity in this table")
            .typed(component_idx);

        Ok(component)
    }

    pub fn get_or_default<C: Component + Default>(&mut self, entity: Entity) -> AddComponent<'_, C> {
        self.add_with(entity, default)
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> AddComponent<'_, C> {
        self.add_with(entity, || component)
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: impl Into<Entity>, f: F) -> AddComponent<'_, C>
        where F: FnOnce() -> C,
              C: Component,
    {
        let entity = entity.into();

        if !self.alive(entity)
        { panic!("Tried to add component to dead entity '{entity}'"); }

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

        let mut new_component_value = Some(f());

        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let components = old_table.sparse_set.remove(entity.index())
            .expect("entity is in table")
            .map(ComponentDenseStorageInput::from)
            // inserts the component into the list at its index
            .chain_after(new_component_idx, once(
                &mut new_component_value as &mut dyn DynOption
            ).map(ComponentDenseStorageInput::from));
        new_table.sparse_set.insert(entity.index(), components);

        let r#ref = new_table.sparse_set.get_mut(entity.index()).expect("Entity just inserted")
            .typed(new_component_idx);

        self.entities_archetypes[entity.index()] = new_archtyp_id;

        AddComponent {
            r#ref,
            was_added: true,
        }
    }

    pub fn remove<C>(&mut self, entity: impl Into<Entity>) -> Option<C>
        where C: Component,
    {
        let entity = entity.into();

        if !self.alive(entity)
        { return None; }

        let component = self.component::<C>();

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];
        if !archetyp.components.has(component) {
            return None;
        }

        let (Some(component_idx), new_component_set) = archetyp.components.clone().without(component)
        else { unreachable!("Component is inside the set") };
        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_table_id = self.archetypes[old_archetyp_id].table_id;
        let new_archtyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        let new_table_id = self.archetypes[new_archtyp_id].table_id;

        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let mut extracted_component = None;
        let components = old_table.sparse_set.remove(entity.index())
            .expect("entity is in table")
            .extract_nth(component_idx, |comp| extracted_component = Some(comp));
        new_table.sparse_set.insert(entity.index(), components);
        let extracted_component = extracted_component.expect("Exist in iterator so should have been extracted");

        self.entities_archetypes[entity.index()] = new_archtyp_id;

        Some(extracted_component.into_typed().expect("Correct type"))
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
