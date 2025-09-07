#[cfg(test)]
mod tests;

mod entity_storage;
pub use entity_storage::{ Entity, EntityIndex, EntityGeneration };
mod entity_set;
pub use entity_set::{ EntitySet, ComponentSet };
pub mod component;
use component::*;
pub mod query;
mod bundle;
pub use bundle::*;

use crate::{
    dyn_option::FunDynOption,
    index_map::IndexMap,
    sparse_set::SparseSet, world_utils::GetComponentTypedError,
};

use std::{
    any::{Any, TypeId}, assert_matches::debug_assert_matches, borrow::Cow, collections::HashMap, iter::{ empty, once }
};
use utils::{extract_nth::ExtractNthExt, itertools::Itertools, prelude::*};
use derive_more::{ IsVariant };
use dynvec::{ DynVec, DynVecMetadata, DynVecValueRef, DynVecValueRefMut, RemovedDynVecValue };

const RESERVED_ENTITY_COUNT: u32 = 100;

macro_rules! create_id {
    ($struct_vis: vis $name: ident($in_vis:vis $ty: ty)) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ::derive_more::From, ::derive_more::Into)]
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

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

mod ids {
    create_id!(pub ArchetypId(u32));
    create_id!(pub TableId(u32));
}
use ids::{ ArchetypId, TableId };

#[derive(Default, Debug, Clone)]
struct Archetyp {
    entities: BitSet<EntityIndex>,
    components: ComponentSet,
    /// Table used for storing the table components of this archetyp.
    /// Multiple Archtyps could point to the same tables if they have the same
    /// (table) components.
    table_id: TableId,
}

#[derive(Default, Debug)]
struct Table {
    /// NOTE: This is a subset of the components used to find this table,
    /// as only components that have table storage are stored in this set
    /// but components that have no storage or are stored in sparse sets
    /// may still 'fragment' tables.
    table_components: ComponentSet,
    sparse_set: SparseSet<ComponentDenseStorage>,
}

/// Ref to a component that may or may not have no storage attached in which
/// case a ref makes no sense. If this is returned this means that the component
/// **is** attached to the relevant entity.
#[derive(Debug)]
pub enum OptionalComponentRef<C> {
    HasStorage(C),
    NoStorage,
}

#[derive(Debug, PartialEq)]
pub struct AddComponent<C> {
    pub component_ref: C,
    pub was_added: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IsVariant)]
pub enum HasComponent {
    /// The entity is dead
    EntityIsNotAlive,
    /// The component's entity is dead
    ComponentIsNotAlive,
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

#[derive(Debug, Clone, IsVariant)]
pub enum ComponentStorageKind {
    /// The component has no storage and thus is stored nowhere
    None,
    /// The component has table storage and thus is stoired alongside
    /// other components with table storage for each archetyp.
    ///
    /// This is the only storage kind that always fragment tables
    Table {
        dynvec_meta: DynVecMetadata,
    },
    // TODO
    // Sparse,
}

#[derive(Debug, thiserror::Error)]
pub enum GetComponentError {
    #[error("Tried to get component of dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Entity of the component {component} is not alive")]
    ComponentIsNotAlive {
        component: ComponentEntity,
    },
    #[error("Component from entity {component} is not present in the entity {entity}")]
    ComponentNotPresent {
        component: ComponentEntity,
        entity: Entity,
    },
    /// Only returned if the Entity *has* the component but this component
    /// doesn't have storage so nothing can be returned.
    #[error("Component from entity {component} doesn't have any storage (doesn't have the ComponentStorageComponent)")]
    ComponentHasNoStorage {
        component: ComponentEntity,
        entity: Entity,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum AddComponentError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Entity of the component {component} is not alive")]
    ComponentIsNotAlive {
        component: ComponentEntity,
    },
    #[error("The component {component} needs a value when inserting (Has a storage attached with a type that does not implement Default)")]
    ComponentNeedsValue {
        component: ComponentEntity,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum AddComponentWithError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("The type given does not match the type used for storage of {component}. Expected: '{expected}', given: '{given}'")]
    TypeMismatched {
        component: ComponentEntity,
        given: &'static str,
        expected: &'static str,
    },
    #[error("{component} is not alive")]
    ComponentIsNotAlive {
        component: ComponentEntity,
    },
    #[error("{component} does NOT have storage. (Does not have a ComponentStorageComponent)")]
    ComponentDoesNotHaveStorage {
        component: ComponentEntity,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveComponentError {
    #[error("Tried to remove a component from a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Entity of the component {component} is not alive")]
    ComponentIsNotAlive {
        component: ComponentEntity,
    },
    #[error("Component from entity {component} is not present in the entity {entity}")]
    ComponentNotPresent {
        component: ComponentEntity,
        entity: Entity,
    },
    #[error("Removing this component from this entity is forbidden by the implementation: {reason}")]
    Forbidden {
        reason: &'static str,
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

    /// Maps components to the set of archetypes that have this component
    components_to_archetypes: IndexMap<BitSet<ArchetypId>, EntityIndex>,

    components_typeid_to_entity: HashMap<TypeId, ComponentEntity>,
    components_entity_to_typeid: HashMap<ComponentEntity, TypeId>,
    components_set_to_archetyp: HashMap<ComponentSet, ArchetypId>,
    /// The [`ComponentSet`] used as keys must only be components that
    /// fragments tables, and may contain components that do not have table
    /// storage, thus this is not the same as the corresponding
    /// [`Table::table_components`]
    components_set_to_table: HashMap<ComponentSet, TableId>,
}

impl World {
    pub fn new() -> Self {
        let mut this = Self {
            entity_storage: entity_storage::EntityStorage::new(RESERVED_ENTITY_COUNT),

            entities_archetypes: default(),

            archetypes: default(),
            tables: default(),

            components_to_archetypes: default(),

            components_typeid_to_entity: default(),
            components_entity_to_typeid: default(),
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

        // Register this component first as it is required in the code paths
        // for other components
        this.component::<ComponentStorageComponent>();

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
        let archetyp = &mut self.archetypes[empty_archetyp];

        archetyp.entities.insert(entity.index());
        let table = &mut self.tables[archetyp.table_id];
        table.sparse_set.insert(entity.index(), empty::<ComponentDenseStorageInput>());

        entity
    }

    /// Spawn a new entity from the reserved entity index space for entities.
    /// Should only be used when creating components at runtime, though
    /// nothing prevents using this as a normal entity and using normal entities
    /// as components.
    pub fn spawn_component(&mut self) -> ComponentEntity {
        let entity = self.entity_storage.take_next_reserved()
            .unwrap_or_else(|| self.entity_storage.spawn());

        ComponentEntity(entity)
    }

    /// Moves all entity from this archetype to a new one that doesn't have this component
    /// deleting the given archetyp and table
    /// Used by [`World::dispawn`] for removing a component being unregistered
    fn remove_component_from_archetype_internal(&mut self, old_archetyp_id: ArchetypId, component: ComponentEntity) {
        let component_storage = self.component_storage(component)
            .expect("Component should be still be alive");

        // FIXME: Recycle the id somehow
        //        Kinda annoying because the id is 'dead' but no way of knowing it
        //        so its a logical error to use the id again but its very silent
        let old_archetyp = std::mem::take(&mut self.archetypes[old_archetyp_id]);
        let old_table_id = old_archetyp.table_id;

        let (was_in_set, new_component_set) = old_archetyp.components
            .without(component);
        debug_assert_matches!(was_in_set, Some(_));

        let new_archetyp_id = self.archtyp_for(Cow::Owned(new_component_set));
        let new_archetyp = &mut self.archetypes[new_archetyp_id];
        let new_table_id = new_archetyp.table_id;

        /*
         * Move all entities from the old archetyp to the new one
         */

        for entity_index in &old_archetyp.entities {
            debug_assert_eq!(self.entities_archetypes[entity_index], old_archetyp_id);
            self.entities_archetypes[entity_index] = new_archetyp_id;
        }
        new_archetyp.entities.extend(old_archetyp.entities.iter());

        /*
         * Now move their components from the old table to the new table
         */

        let old_table = std::mem::take(&mut self.tables[old_table_id]);
        if old_table.sparse_set.len() == 0 {
            // To this is a bit weird, because we are running this on all
            // archetypes with the given component, some way share the same
            // table_id, which mean we may reach this point on multiple table_ids
            // So to know if the table has already been deleted we check its len
            //
            // Note that for an empty table we would have nothing to do anyway
            return;
        }

        let component_idx = old_table.table_components.index_of(component);

        println!("{component_idx:?}");
        if !self.component_fragments_tables(component) {
            debug_assert_eq!(old_table_id, new_table_id);
            debug_assert_matches!(component_idx, None);
            return;
        }
        debug_assert_ne!(old_table_id, new_table_id);

        match component_storage {
            ComponentStorageKind::None => debug_assert_matches!(component_idx, None),
            ComponentStorageKind::Table { .. } => debug_assert_matches!(component_idx, Some(_)),
        }

        // now we remove everything (drain) the old table and insert everything
        // but the component we are unregistering into the new table

        let mut deconstructed_table = old_table.sparse_set.into_deconstructed();
        let mut column_drains = deconstructed_table.dense_values.drain()
            .collect_vec();

        let new_table = &mut self.tables[new_table_id];
        for entity in deconstructed_table.dense_to_sparse_indices {
            println!("{entity:?} - {component_idx:?}");
            let components = column_drains.iter_mut()
                .map(|column| column.next().expect("Should be the same length as the entities"));
            let components = zip_right(components, component_idx)
                .map_right(|(components, component_idx)| {
                    components.extract_nth(component_idx, |_| {
                        /* we just drop the component value we just removed */
                    })
                });

            new_table.sparse_set.insert(entity, components);
        }
    }

    /// Returns false if the entity was already dead.
    pub fn dispawn(&mut self, entity: impl Into<Entity>) -> bool {
        let entity = entity.into();
        let centity = ComponentEntity(entity);

        if !self.alive(entity) {
            return false;
        }

        if let Some(type_id) = self.components_entity_to_typeid.remove(&centity) {
            let val = self.components_typeid_to_entity.remove(&type_id);
            debug_assert_eq!(val, Some(centity));
        }

        // Set of all archetypes that have this component
        if let Some(component_archetypes) = self.components_to_archetypes.get_mut(entity.index()).map(std::mem::take) {
            for archetyp_id in component_archetypes {
                self.remove_component_from_archetype_internal(archetyp_id, centity);
            }
            // There must not be any archetype with this component anymore
            debug_assert!(self.archetypes.values().all(|archetyp| !archetyp.components.has(centity)));
        }

        self.entity_storage.dispawn(entity.into());

        let archetyp: ArchetypId = self.entities_archetypes[entity.index()];
        let archetyp: &mut Archetyp = &mut self.archetypes[archetyp];

        archetyp.entities.remove(entity.index());

        let table_id: TableId = archetyp.table_id;

        self.tables[table_id].sparse_set.remove(entity.index());

        true
    }

    /// Returns the entity for the given component type_id, or None if it was never
    /// registred.
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
                let new_entity = *vacant.insert(ComponentEntity(
                    // cannot call self.spawn_component because self is partially-borrowed
                    self.entity_storage.take_next_reserved()
                        .unwrap_or_else(|| self.entity_storage.spawn())
                ));
                let previous = self.components_entity_to_typeid.insert(new_entity, type_id);
                debug_assert_eq!(previous, None);
                new_entity
            },
        };

        self.add(created_entity, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<C>(),
        }).expect("Entity is alive");

        created_entity
    }

    pub fn component_storage(&self, component: ComponentEntity) -> Option<ComponentStorageKind> {
        // fixes infinite recursion
        // NOTE: This does not mean the component entity for ComponentStorageComponent
        // does not have itself as component.
        if component == self.components_typeid_to_entity[&TypeId::of::<ComponentStorageComponent>()] {
            return Some(ComponentStorageKind::Table {
                dynvec_meta: DynVecMetadata::new::<ComponentStorageComponent>(),
            });
        }

        match self.get::<ComponentStorageComponent>(component) {
            Ok(storage)
                => Some(ComponentStorageKind::Table {
                    dynvec_meta: storage.dynvec_meta.clone(),
                }),
            Err(GetComponentTypedError::EntityIsNotAlive { .. })
                => None,
            Err(GetComponentTypedError::UnknownComponent { .. })
                => unreachable!("This component is always registred"),
            Err(GetComponentTypedError::ComponentNotPresent { .. })
                => Some(ComponentStorageKind::None),
        }
    }

    /// Dummy function for now just to mark everywhere this will change things
    fn component_fragments_tables(&self, component: ComponentEntity) -> bool {
        let _ = component;
        true
    }

    fn table_for(&mut self, components: Cow<'_, ComponentSet>) -> TableId {
        debug_assert!(
            components.iter().all(|comp| self.component_fragments_tables(comp)),
            "Components given to table_for should all fragment tables"
        );
        match self.components_set_to_table.get(components.as_ref()) {
            Some(&table_id) => table_id,
            None => {
                let components = components.into_owned();
                let table_components: ComponentSet = components.iter()
                    .filter(|&comp| {
                        self.component_storage(comp)
                            .is_some_and(|storage| storage.is_table())
                    })
                    .collect();
                let table_id = self.tables.push(Table {
                    table_components: table_components.clone(),
                    sparse_set: SparseSet::new(ComponentDenseStorage::new(
                        table_components.iter()
                            .map(|component| {
                                let Some(ComponentStorageKind::Table { dynvec_meta }) = self.component_storage(component)
                                else { unreachable!("Filtered before"); };

                                DynVec::new_with_meta(dynvec_meta)
                            })
                            .collect::<Vec<_>>()
                            .into_boxed_slice()
                    )),
                });

                self.components_set_to_table.insert(components, table_id);

                table_id
            },
        }
    }

    fn archtyp_for(&mut self, component_set: Cow<'_, ComponentSet>) -> ArchetypId {
        match self.components_set_to_archetyp.get(component_set.as_ref()) {
            Some(&id) => id,
            None => {
                let component_set = component_set.into_owned();
                let table_components: ComponentSet = component_set.iter()
                    .filter(|&comp| self.component_fragments_tables(comp))
                    .collect();
                let table_id = self.table_for(Cow::Borrowed(&table_components));
                let archetyp_id = self.archetypes.push(Archetyp {
                    entities: BitSet::new(),
                    components: component_set.clone(),
                    table_id,
                });

                for component in component_set.iter() {
                    self.components_to_archetypes.extend_until(component.index(), BitSet::new);
                    self.components_to_archetypes[component.index()].insert(archetyp_id);
                }

                self.components_set_to_archetyp.insert(component_set, archetyp_id);

                archetyp_id
            },
        }
    }

    fn get_in_table_internal(
        &self, table_id: TableId, component: ComponentEntity, entity_index: EntityIndex,
    ) -> DynVecValueRef<'_> {
        debug_assert_matches!(self.component_storage(component), Some(ComponentStorageKind::Table { .. }));

        let table = &self.tables[table_id];

        let component_idx = table.table_components.index_of(component)
            .expect("Tried to get component in table where it is not present");
        let component_ref = table.sparse_set.get(entity_index)
            .expect("Tried to get an entity's component in a table where it is not present")
            .for_component(component_idx);

        component_ref
    }

    fn get_mut_in_table_internal(
        &mut self, table_id: TableId, component: ComponentEntity, entity_index: EntityIndex
    ) -> DynVecValueRefMut<'_> {
        debug_assert_matches!(self.component_storage(component), Some(ComponentStorageKind::Table { .. }));

        let table = &mut self.tables[table_id];

        let component_idx = table.table_components.index_of(component)
            .expect("Tried to get component in table where it is not present");
        let component_ref = table.sparse_set.get_mut(entity_index)
            .expect("Tried to get an entity's component in a table where it is not present")
            .for_component(component_idx);

        component_ref
    }

    pub fn has_component(&self, entity: impl Into<Entity>, component: ComponentEntity) -> HasComponent {
        let entity = entity.into();

        if !self.alive(entity) {
            return HasComponent::EntityIsNotAlive;
        }
        if !self.alive(component) {
            return HasComponent::ComponentIsNotAlive;
        }

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];

        if archetyp.components.has(component) {
            HasComponent::Present
        }
        else {
            HasComponent::NotPresent
        }
    }

    pub fn get_component(&self, entity: impl Into<Entity>, component: ComponentEntity) -> Result<DynVecValueRef<'_>, GetComponentError> {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(GetComponentError::EntityIsNotAlive { entity });
        }
        let Some(component_storage) = self.component_storage(component)
        else {
            return Err(GetComponentError::ComponentIsNotAlive { component });
        };

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &self.archetypes[archetyp_id];

        if !archetyp.components.has(component) {
            return Err(GetComponentError::ComponentNotPresent { entity, component });
        }

        match component_storage {
            ComponentStorageKind::None => return Err(
                GetComponentError::ComponentHasNoStorage { component, entity }
            ),
            ComponentStorageKind::Table { .. } => (),
        }

        Ok(self.get_in_table_internal(
            archetyp.table_id, component, entity.index()
        ))
    }

    pub fn get_component_mut(&mut self, entity: impl Into<Entity>, component: ComponentEntity) -> Result<DynVecValueRefMut<'_>, GetComponentError> {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(GetComponentError::EntityIsNotAlive { entity });
        }
        let Some(component_storage) = self.component_storage(component)
        else {
            return Err(GetComponentError::ComponentIsNotAlive { component });
        };

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];

        if !archetyp.components.has(component) {
            return Err(GetComponentError::ComponentNotPresent { entity, component });
        }

        match component_storage {
            ComponentStorageKind::None => return Err(
                GetComponentError::ComponentHasNoStorage { component, entity }
            ),
            ComponentStorageKind::Table { .. } => (),
        }

        let table_id = archetyp.table_id;

        Ok(self.get_mut_in_table_internal(
            table_id, component, entity.index()
        ))
    }

    /// The [`input`] function must retrun an iterator with exactly one
    /// element.
    ///
    /// If it returns ComponentDenseStorageInput::Default the component should
    /// be checked before if it has a default constructor, otherwise there
    /// will be a panic internally.
    // TODO: Be able to insert mutliple components at once, what would be the
    // best api for this ? (something like bevy's bundles I guess)
    fn add_component_internal<'a, 'b>(
        &'a mut self,
        entity: Entity,
        component_storage: ComponentStorageKind,
        component: ComponentEntity,
        input: ComponentDenseStorageInput<'a, 'b>,
    ) -> AddComponent<OptionalComponentRef<DynVecValueRefMut<'a>>> {
        // things that should be checked before calling this function
        debug_assert!(self.alive(entity));
        debug_assert!(self.alive(component));
        // In theory (it does not implement Eq)
        // debug_assert_eq!(self.component_storage(component), Some(component_storage));

        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_archetyp = &mut self.archetypes[old_archetyp_id];
        let old_table_id = old_archetyp.table_id;

        if old_archetyp.components.has(component) {
            match component_storage {
                ComponentStorageKind::None => return AddComponent {
                    component_ref: OptionalComponentRef::NoStorage,
                    was_added: false,
                },
                ComponentStorageKind::Table { .. } => (),
            };

            let table_id = old_archetyp.table_id;
            return AddComponent {
                component_ref: OptionalComponentRef::HasStorage(
                    self.get_mut_in_table_internal(
                        table_id, component, entity.index()
                    )
                ),
                was_added: false,
            };

        }

        // we have to change the entity's archetyp

        old_archetyp.entities.remove(entity.index());
        // self.entities_archetypes[entity.index()] = undefined;

        let (_, new_component_set) = old_archetyp.components.clone().with(component);
        let new_archetyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        debug_assert_ne!(old_archetyp_id, new_archetyp_id);
        let new_archetyp = &mut self.archetypes[new_archetyp_id];
        let new_table_id = new_archetyp.table_id;

        new_archetyp.entities.insert(entity.index());
        self.entities_archetypes[entity.index()] = new_archetyp_id;

        // Add the component's value into its storage
        match component_storage {
            ComponentStorageKind::None => {
                // the value doesn't actually matter but lets check the caller
                // knew that
                debug_assert!(matches!(input, ComponentDenseStorageInput::Default));
                
                // Nothing to add anywhere
            },
            ComponentStorageKind::Table { .. } => {
                // This is done at the same time as changing the entity's table
            },
            // TODO: For sparse storage we just insert it now
        }

        if !self.component_fragments_tables(component) {
            assert_eq!(old_table_id, new_table_id);

            return match component_storage {
                ComponentStorageKind::None => {
                    AddComponent {
                        component_ref: OptionalComponentRef::NoStorage,
                        was_added: true,
                    }
                },
                ComponentStorageKind::Table { .. } => unreachable!("Table storage cannot not fragment tables"),
                // TODO: For sparse storage we just fetch the data
            };
        }

        // The component fragments tables so the tables must be different here
        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let components = old_table.sparse_set.remove(entity.index())
            .expect("Entity is in table")
            .map(ComponentDenseStorageInput::RemovedDynVecValue);

        let component_ref = match new_table.table_components.index_of(component) {
            Some(new_idx) => {
                debug_assert_matches!(component_storage, ComponentStorageKind::Table { .. });

                // inserts the component into the list at its index
                let components = components.chain_after(new_idx, once(input));

                new_table.sparse_set.insert(entity.index(), components);

                // then gets its reference
                let r#ref = new_table.sparse_set.get_mut(entity.index())
                    .expect("Entity just inserted")
                    .for_component(new_idx);

                OptionalComponentRef::HasStorage(r#ref)
            },
            None => {
                debug_assert_matches!(component_storage, ComponentStorageKind::None);

                new_table.sparse_set.insert(entity.index(), components);

                OptionalComponentRef::NoStorage
            },
        };

        AddComponent {
            component_ref,
            was_added: true,
        }
    }

    /// The component must have no storage or its storage type must implement Default.
    pub fn add_component(
        &mut self,
        entity: impl Into<Entity>,
        component: ComponentEntity,
    ) -> Result<AddComponent<OptionalComponentRef<DynVecValueRefMut<'_>>>, AddComponentError> {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(AddComponentError::EntityIsNotAlive { entity });
        }

        if !self.alive(component) {
            return Err(AddComponentError::ComponentIsNotAlive { component });
        }

        let component_storage = self.component_storage(component).expect("Entity is alive");
        match component_storage {
            ComponentStorageKind::None |
            ComponentStorageKind::Table { dynvec_meta: DynVecMetadata { default_fn: Some(_), .. }, .. } => (),

            ComponentStorageKind::Table { dynvec_meta: DynVecMetadata { default_fn: None, .. }, .. } => {
                return Err(AddComponentError::ComponentNeedsValue { component });
            },
        }

        Ok(self.add_component_internal(
            entity, component_storage, component,
            ComponentDenseStorageInput::Default,
        ))
    }

    pub fn add_component_with<V: Any>(
        &mut self,
        entity: impl Into<Entity>,
        component: ComponentEntity,
        f: impl FnOnce() -> V,
    ) -> Result<AddComponent<&'_ mut V>, AddComponentWithError> {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(AddComponentWithError::EntityIsNotAlive { entity });
        }

        let Some(component_storage) = self.component_storage(component)
        else {
            return Err(AddComponentWithError::ComponentIsNotAlive { component });
        };

        match component_storage {
            ComponentStorageKind::None =>
                return Err(AddComponentWithError::ComponentDoesNotHaveStorage {
                    component,
                }),
            ComponentStorageKind::Table {
                dynvec_meta: DynVecMetadata { type_id, type_name: expected, .. }, ..
            } if type_id != TypeId::of::<V>() =>
                return Err(AddComponentWithError::TypeMismatched {
                    component,
                    given: std::any::type_name::<V>(),
                    expected,
                }),
            _ => (),
        }

        let mut fun_dyn_option = FunDynOption::new(f);
        let result = self.add_component_internal(
            entity, component_storage, component,
            ComponentDenseStorageInput::DynOption(&mut fun_dyn_option),
        );

        Ok(AddComponent {
            component_ref: match result.component_ref {
                OptionalComponentRef::HasStorage(mut r) => r.as_typed::<V>()
                    .expect("Correctly typed"),
                OptionalComponentRef::NoStorage => unreachable!("Created from type so must have storage"),
            },
            was_added: result.was_added,
        })
    }

    pub fn remove_component(
        &mut self,
        entity: impl Into<Entity>,
        component: ComponentEntity
    ) -> Result<OptionalComponentRef<RemovedDynVecValue<'_>>, RemoveComponentError> {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(RemoveComponentError::EntityIsNotAlive { entity });
        }

        let Some(component_storage) = self.component_storage(component)
        else {
            return Err(RemoveComponentError::ComponentIsNotAlive { component });
        };

        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_archetyp = &mut self.archetypes[old_archetyp_id];
        let old_table_id = old_archetyp.table_id;

        if !old_archetyp.components.has(component) {
            return Err(RemoveComponentError::ComponentNotPresent { component, entity });
        }

        if component == self.components_typeid_to_entity[&TypeId::of::<ComponentStorageComponent>()] {
            if self.components_entity_to_typeid.contains_key(&ComponentEntity(entity)) {
                return Err(RemoveComponentError::Forbidden {
                    reason: "Cannot remove the componentStorageComponent from an internal component entity.",
                });
            }

            todo!("Remove the storage from all tables");
        }

        /*
         * We have to change the entity's archetyp
         */

        old_archetyp.entities.remove(entity.index());

        let (_, new_component_set) = old_archetyp.components.clone().without(component);
        let new_archetyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        debug_assert_ne!(old_archetyp_id, new_archetyp_id);
        let new_archetyp = &mut self.archetypes[new_archetyp_id];
        let new_table_id = new_archetyp.table_id;

        new_archetyp.entities.insert(entity.index());
        self.entities_archetypes[entity.index()] = new_archetyp_id;

        // Removes the component's value from its storage
        match component_storage {
            ComponentStorageKind::None => {
                // Nothing to remove anywhere
            },
            ComponentStorageKind::Table { .. } => {
                // This is done at the same time as changing the component's table
            },
            // TODO: For sparse storage we should remove it now
        }

        if !self.component_fragments_tables(component) {
            debug_assert_eq!(old_table_id, new_table_id);

            return Ok(match component_storage {
                ComponentStorageKind::None => {
                    OptionalComponentRef::NoStorage
                },
                ComponentStorageKind::Table { .. } => unreachable!("Table storage cannot not fragment tables"),
                // TODO: For sparse storage we should just return its value
            });
        }

        // The component fragments tables so the tables must be different here
        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let components = old_table.sparse_set.remove(entity.index())
            .expect("entity is in table");

        let component_ref = match old_table.table_components.index_of(component) {
            Some(component_idx) => {
                debug_assert_matches!(component_storage, ComponentStorageKind::Table { .. });

                let mut extracted_component = None;
                let components = components
                    .extract_nth(component_idx, |comp| extracted_component = Some(comp));

                new_table.sparse_set.insert(entity.index(), components);

                let extracted_component = extracted_component
                    .expect("Exist in iterator so should have been extracted");

                OptionalComponentRef::HasStorage(extracted_component)
            },
            None => {
                debug_assert_matches!(component_storage, ComponentStorageKind::None);

                new_table.sparse_set.insert(entity.index(), components);

                OptionalComponentRef::NoStorage
            },
        };

        Ok(component_ref)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
