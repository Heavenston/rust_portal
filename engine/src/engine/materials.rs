use crate::{ *, utils::* };

use std::{ any::{ Any, TypeId }, collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData };

#[derive(Debug)]
pub struct MaterialData {
    pub pipeline: wgpu::RenderPipeline,
}

pub trait MaterialParameters: Debug + PartialEq + Eq + Hash + Clone + 'static { }
pub trait MaterialFactory<P> = (FnMut(&mut Renderer, &P) -> MaterialData) + 'static;

trait ObjectSafeFactory: 'static {
    fn create(&mut self, renderer: &mut Renderer, parameters: &dyn Any) -> MaterialData;
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
        (self.factory)(renderer, parameters.downcast_ref::<P>().expect("Correct type"))
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

#[derive_where::derive_where(Debug)]
#[derive(Default)]
pub struct MaterialsStore {
    materials: HashMap<MaterialHandle, MaterialData>,
    #[derive_where(skip)]
    factories: HashMap<TypeId, Box<dyn ObjectSafeFactory>>,
}

impl MaterialsStore {
    pub fn new() -> Self {
        Self::default()
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
        }

        handle
    }

    pub fn get(&self, handle: MaterialHandle) -> &MaterialData {
        &self.materials[&handle]
    }
}
