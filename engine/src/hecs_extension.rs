use smallvec::SmallVec;

pub struct ParentComponent(pub Option<hecs::Entity>);
pub struct ChildrenComponent(pub SmallVec<[hecs::Entity; 8]>);

pub trait WorldExt {
    fn add_child(
        &self, parent_entity: hecs::Entity, child_entity: hecs::Entity,
    ) -> Result<(), hecs::ComponentError>;
    fn remove_parent(&self, child_entity: hecs::Entity) -> Result<(), hecs::ComponentError>;
}
impl WorldExt for hecs::World {
    fn add_child(
        &self, parent_entity: hecs::Entity, child_entity: hecs::Entity,
    ) -> Result<(), hecs::ComponentError> {
        let mut children = self.get_mut::<ChildrenComponent>(parent_entity)?;
        let mut parent = self.get_mut::<ParentComponent>(child_entity)?;
        if let Some(old_parent_entity) = parent.0 {
            let mut old_children = self.get_mut::<ChildrenComponent>(old_parent_entity)?;
            let child_index = old_children
                .0
                .iter()
                .copied()
                .enumerate()
                .find_map(|(a, b)| {
                    if b == child_entity {
                        Some(a)
                    }
                    else {
                        None
                    }
                })
                .unwrap();
            old_children.0.swap_remove(child_index);
        }
        if children.0.contains(&child_entity) {
            return Ok(());
        }
        children.0.push(child_entity);
        parent.0 = Some(parent_entity);
        Ok(())
    }

    /// Alias: Orphaner
    fn remove_parent(&self, child_entity: hecs::Entity) -> Result<(), hecs::ComponentError> {
        let mut parent = self.get_mut::<ParentComponent>(child_entity)?;
        if let Some(old_parent_entity) = parent.0 {
            let mut old_children = self.get_mut::<ChildrenComponent>(old_parent_entity)?;
            let child_index = old_children
                .0
                .iter()
                .copied()
                .enumerate()
                .find_map(|(a, b)| {
                    if b == child_entity {
                        Some(a)
                    }
                    else {
                        None
                    }
                })
                .unwrap();
            old_children.0.swap_remove(child_index);
        }
        parent.0 = None;
        Ok(())
    }
}
