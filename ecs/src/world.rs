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
use utils::prelude::*;
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
        self.tables[archetyp.table_id]
            .sparse_set.insert(entity.index(), empty::<ComponentDenseStorageInput>());
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

    /// Returns false if the entity was already dead.
    pub fn dispawn(&mut self, entity: impl Into<Entity>) -> bool {
        let entity = entity.into();

        let was_alive = self.entity_storage.dispawn(entity.into());
        if !was_alive {
            return false;
        }

        let archetyp: ArchetypId = self.entities_archetypes[entity.index()];
        let archetyp: &mut Archetyp = &mut self.archetypes[archetyp];

        archetyp.entities.remove(entity.index());

        let table_id: TableId = archetyp.table_id;

        self.tables[table_id].sparse_set.remove(entity.index());

        if let Some(type_id) = self.components_entity_to_typeid.remove(&ComponentEntity(entity)) {
            let val = self.components_typeid_to_entity.remove(&type_id);
            debug_assert_eq!(val, Some(ComponentEntity(entity)));
        }

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
            components.iter()
                .all(|comp| self.component_fragments_tables(comp)),
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
                    .filter(|&comp| {
                        self.component_storage(comp)
                            .is_some_and(|storage| storage.is_table())
                    })
                    .collect();
                let table_id = self.table_for(Cow::Borrowed(&table_components));
                let archetyp_id = self.archetypes.push(Archetyp {
                    entities: BitSet::new(),
                    components: component_set.clone(),
                    table_id,
                });

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

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];
        if archetyp.components.has(component) {
            match component_storage {
                ComponentStorageKind::None => return AddComponent {
                    component_ref: OptionalComponentRef::NoStorage,
                    was_added: false,
                },
                ComponentStorageKind::Table { .. } => (),
            };

            let table_id = archetyp.table_id;
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

        let (new_component_idx, new_component_set) = archetyp.components.clone().with(component);
        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_archetyp = &mut self.archetypes[old_archetyp_id];

        old_archetyp.entities.remove(entity.index());
        
        let old_table_id = self.archetypes[old_archetyp_id].table_id;

        let new_archtyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        let new_archtyp = &mut self.archetypes[new_archtyp_id];

        new_archtyp.entities.insert(entity.index());
        self.entities_archetypes[entity.index()] = new_archtyp_id;

        let new_table_id = self.archetypes[new_archtyp_id].table_id;

        match component_storage {
            ComponentStorageKind::None => {
                debug_assert_eq!(old_table_id, new_table_id);
                debug_assert!(
                    matches!(input, ComponentDenseStorageInput::Default),
                    "No need to actually provide a value",
                );
                return AddComponent {
                    component_ref: OptionalComponentRef::NoStorage,
                    was_added: true,
                }
            },
            ComponentStorageKind::Table { .. } => (),
        }

        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let components = old_table.sparse_set.remove(entity.index())
            .expect("Entity is in table")
            .map(ComponentDenseStorageInput::DynVecValue)
            // inserts the component into the list at its index
            .chain_after(new_component_idx, once(input));
        new_table.sparse_set.insert(entity.index(), components);

        let r#ref = new_table.sparse_set.get_mut(entity.index())
            .expect("Entity just inserted")
            .for_component(new_component_idx);

        AddComponent {
            component_ref: OptionalComponentRef::HasStorage(r#ref),
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

        if component == self.components_typeid_to_entity[&TypeId::of::<ComponentStorageComponent>()] {
            if self.components_entity_to_typeid.contains_key(&ComponentEntity(entity)) {
                return Err(RemoveComponentError::Forbidden {
                    reason: "Cannot remove the componentStorageComponent from an internal component entity.",
                });
            }

            todo!("Remove the storage from all tables");
        }

        let archetyp_id = self.entities_archetypes[entity.index()];
        let archetyp = &mut self.archetypes[archetyp_id];

        if !archetyp.components.has(component) {
            return Err(RemoveComponentError::ComponentNotPresent { component, entity });
        }

        let (Some(component_idx), new_component_set) = archetyp.components.clone()
            .without(component)
        else { unreachable!("Component is inside the set") };

        let old_archetyp_id = self.entities_archetypes[entity.index()];
        let old_archetyp = &mut self.archetypes[old_archetyp_id];

        old_archetyp.entities.remove(entity.index());
        
        let old_table_id = self.archetypes[old_archetyp_id].table_id;

        let new_archtyp_id = self.archtyp_for(Cow::Borrowed(&new_component_set));
        let new_archtyp = &mut self.archetypes[new_archtyp_id];

        new_archtyp.entities.insert(entity.index());
        self.entities_archetypes[entity.index()] = new_archtyp_id;
        
        let new_table_id = self.archetypes[new_archtyp_id].table_id;

        match component_storage {
            ComponentStorageKind::None => {
                debug_assert_eq!(old_table_id, new_table_id);
                return Ok(OptionalComponentRef::NoStorage);
            },
            ComponentStorageKind::Table { .. } => (),
        }

        let [old_table, new_table] = self.tables.get_disjoint_mut([
            old_table_id, new_table_id,
        ]).expect("Old table and new table are not equal");

        let mut extracted_component = None;
        let components = old_table.sparse_set.remove(entity.index())
            .expect("entity is in table")
            .extract_nth(component_idx, |comp| extracted_component = Some(comp));
        new_table.sparse_set.insert(entity.index(), components);
        let extracted_component = extracted_component.expect("Exist in iterator so should have been extracted");

        Ok(OptionalComponentRef::HasStorage(extracted_component))
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
