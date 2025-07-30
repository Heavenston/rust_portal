use crate::{ *, utils::* };

use std::{
    any::{ Any, TypeId },
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    time::SystemTime,
};

#[derive(Debug)]
pub struct MaterialData {
    pub created_at: SystemTime,
    pub pipeline: PipelineHandle,
}

pub trait MaterialParameters: Debug + PartialEq + Eq + Hash + Clone + 'static { }
pub trait MaterialFactory<P>: 'static {
    fn create(&mut self, renderer: &mut Renderer, parameters: &P) -> MaterialData;
    fn is_outdated(&mut self, data: &MaterialData) -> bool;
}

impl<F, P> MaterialFactory<P> for F
    where F: (FnMut(&mut Renderer, &P) -> MaterialData) + 'static
{
    fn create(&mut self, renderer: &mut Renderer, parameters: &P) -> MaterialData {
        self(renderer, parameters)
    }

    fn is_outdated(&mut self, _data: &MaterialData) -> bool {
        false
    }
}

trait ObjectSafeFactory: 'static {
    fn create(&mut self, renderer: &mut Renderer, parameters: &dyn Any) -> MaterialData;
    fn is_outdated(&mut self, data: &MaterialData) -> bool;
}

struct FactoryWrapper<P, F>
    where P: MaterialParameters,
          F: MaterialFactory<P>
{
    parameters: PhantomData<*const P>,
    factory: F,
}

impl<P, F> ObjectSafeFactory for FactoryWrapper<P, F>
    where P: MaterialParameters,
          F: MaterialFactory<P>
{
    fn create(&mut self, renderer: &mut Renderer, parameters: &dyn Any) -> MaterialData {
        self.factory.create(renderer, parameters.downcast_ref::<P>().expect("Correct type"))
    }

    fn is_outdated(&mut self, data: &MaterialData) -> bool {
        self.factory.is_outdated(data)
    }
}

impl<P, F> From<F> for FactoryWrapper<P, F>
    where P: MaterialParameters,
          F: MaterialFactory<P>
{
    fn from(factory: F) -> Self {
        Self {
            parameters: default(),
            factory,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct MaterialHandle {
    parameters_type_id: TypeId,
    parameters_hash: u64,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct MaterialInstance {
    pub material: MaterialHandle,
    pub bind_group: BindGroupHandle,
}

#[derive_where::derive_where(Debug)]
#[derive(Default)]
pub struct MaterialsStore {
    original_parameters: HashMap<u64, Box<dyn Any>>,
    materials: HashMap<MaterialHandle, MaterialData>,
    #[derive_where(skip)]
    factories: HashMap<TypeId, Box<dyn ObjectSafeFactory>>,
}

impl MaterialsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recreate_outdated(&mut self, renderer: &mut Renderer) {
        for (handle, data) in &mut self.materials {
            let factory = self.factories.get_mut(&handle.parameters_type_id)
                .expect("Present");
            if factory.is_outdated(data) {
                let parameters = &self.original_parameters[&handle.parameters_hash];
                *data = factory.create(renderer, &**parameters);
            }
        }
    }

    pub fn register<P: MaterialParameters, F: MaterialFactory<P>>(
        &mut self, factory: F
    ) {
        let tid = TypeId::of::<P>();

        use std::collections::hash_map::Entry;
        match self.factories.entry(tid) {
            Entry::Occupied(_) => panic!("Material already registered"),
            Entry::Vacant(entry) => {
                entry.insert(Box::new(FactoryWrapper::from(factory)));
            },
        }
    }

    pub fn get_handle<P: MaterialParameters>(
        &mut self, renderer: &mut Renderer, parameters: &P
    ) -> MaterialHandle {
        let parameters_hash = hash_value(&parameters);
        let parameters_type_id = TypeId::of::<P>();

        let handle = MaterialHandle {
            parameters_type_id,
            parameters_hash,
        };

        if !self.materials.contains_key(&handle) {
            let tid = TypeId::of::<P>();
            let factory = self.factories.get_mut(&tid)
                .expect("Unknown material parameters type given");
            let data = factory.create(renderer, parameters);
            self.materials.insert(handle, data);
            self.original_parameters.insert(parameters_hash, Box::new(parameters.clone()));
        }

        handle
    }

    pub fn get(&self, handle: MaterialHandle) -> &MaterialData {
        &self.materials[&handle]
    }
}
