use utils::prelude::*;
use pgk::{ BindGroupHandle, GraphicsKernel, PipelineHandle };
use crate::*;

use std::{
    any::{ Any, TypeId },
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    time::{Duration, SystemTime},
};

#[derive(Debug)]
pub struct MaterialData {
    pub created_at: SystemTime,
    pub pipeline: PipelineHandle,
}

pub trait MaterialParameters: Debug + PartialEq + Eq + Hash + Clone + 'static { }
pub trait MaterialFactory<P>: 'static {
    fn create(&mut self, kernel: &mut GraphicsKernel, parameters: &P) -> MaterialData;
    fn is_outdated(&mut self, data: &MaterialData) -> bool;
}

impl<F, P> MaterialFactory<P> for F
    where F: (FnMut(&mut GraphicsKernel, &P) -> MaterialData) + 'static
{
    fn create(&mut self, kernel: &mut GraphicsKernel, parameters: &P) -> MaterialData {
        self(kernel, parameters)
    }

    fn is_outdated(&mut self, _data: &MaterialData) -> bool {
        false
    }
}

pub struct EmbeddedShaderFactoryHelper<F, P, E> {
    parameters: PhantomData<fn(P, E) -> (P, E)>,
    factory: F,
    file_name: String,
}

impl<F, P, E> MaterialFactory<P> for EmbeddedShaderFactoryHelper<F, P, E>
    where F: (FnMut(&mut GraphicsKernel, &str, &P) -> MaterialData) + 'static,
          P: MaterialParameters,
          E: rust_embed::Embed + 'static,
{
    fn create(&mut self, kernel: &mut GraphicsKernel, parameters: &P) -> MaterialData {
        let shader_source_file = E::get(&self.file_name)
            .expect("Could not find shader source file");
        let shader_source = str::from_utf8(&shader_source_file.data)
            .expect("Invalid utf8 in shader source file");

        (self.factory)(kernel, shader_source, parameters)
    }

    fn is_outdated(&mut self, data: &MaterialData) -> bool {
        let shader_source_file = builtin_shaders::BuiltinShaders::get(&self.file_name)
            .expect("Could not find shader source file");
        let Some(last_modified) = shader_source_file.metadata.last_modified()
        else {
            // FIXME: Change to warn! or something
            println!("Could not get last modified on shader source file");
            return false;
        };
        let last_modified_time =
            SystemTime::UNIX_EPOCH + Duration::from_secs(last_modified);

        data.created_at < last_modified_time
    }
}

pub fn embedded_shader_factory_helper<F, P, E>(
    fun: F,
    _embed: E,
    file_name: impl Into<String>,
) -> EmbeddedShaderFactoryHelper<F, P, E> {
    EmbeddedShaderFactoryHelper {
        parameters: PhantomData,
        factory: fun,
        file_name: file_name.into(),
    }
}

trait ObjectSafeFactory: 'static {
    fn create(&mut self, kernel: &mut GraphicsKernel, parameters: &dyn Any) -> MaterialData;
    fn is_outdated(&mut self, data: &MaterialData) -> bool;
}

struct FactoryWrapper<P, F>
    where P: MaterialParameters,
          F: MaterialFactory<P>
{
    parameters: PhantomData<fn(P) -> P>,
    factory: F,
}

impl<P, F> ObjectSafeFactory for FactoryWrapper<P, F>
    where P: MaterialParameters,
          F: MaterialFactory<P>
{
    fn create(&mut self, kernel: &mut GraphicsKernel, parameters: &dyn Any) -> MaterialData {
        self.factory.create(kernel, parameters.downcast_ref::<P>().expect("Correct type"))
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

    pub fn recreate_outdated(&mut self, kernel: &mut GraphicsKernel) {
        for (handle, data) in &mut self.materials {
            let factory = self.factories.get_mut(&handle.parameters_type_id)
                .expect("Present");
            if factory.is_outdated(data) {
                let parameters = &self.original_parameters[&handle.parameters_hash];
                *data = factory.create(kernel, &**parameters);
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
        &mut self, kernel: &mut GraphicsKernel, parameters: &P
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
            let data = factory.create(kernel, parameters);
            self.materials.insert(handle, data);
            self.original_parameters.insert(parameters_hash, Box::new(parameters.clone()));
        }

        handle
    }

    pub fn get(&self, handle: MaterialHandle) -> &MaterialData {
        &self.materials[&handle]
    }
}
