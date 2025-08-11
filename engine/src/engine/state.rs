use std::ops::Deref;

use utils::{ default, handle_map };
use pgk::{
    color::{ Srgb, Srgba },
    BindGroupLayoutHandle,
    GraphicsKernel, BufferHandle, BindGroupHandle,
};
use crate::*;

use glam::{ Affine3A, Mat4, Vec3 };

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub transform: Affine3A,
    pub projection: Mat4,

    pub clear_color: Srgba,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticMesh {
    pub transform: Affine3A,
    /// list of vec3
    pub positions_buffer: BufferHandle,
    /// list of vec2
    pub texcoords_buffer: BufferHandle,
    /// list of vec2
    pub normals_buffer: BufferHandle,
    /// list of u32
    pub index_buffer: BufferHandle,
    /// At most the size of the indices buffer
    pub vertex_count: u32,

    pub material_instance: MaterialInstance,
}

impl StaticMesh {
    pub fn delete_buffers(&self, kernel: &mut GraphicsKernel) {
        kernel.delete_buffer(self.positions_buffer);
        kernel.delete_buffer(self.texcoords_buffer);
        kernel.delete_buffer(self.index_buffer);
    }
}

#[derive(Debug)]
pub struct StaticMeshData {
    pub mesh: StaticMesh,
    pub object_bind_group: BindGroupHandle,
}
pub type StaticMeshHandle = handle_map::Handle<StaticMeshData>;

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Srgb,
}

#[derive(Debug)]
pub struct DirectionalLightData {
    pub directional_light: DirectionalLight,
}
pub type DirectionalLightHandle = handle_map::Handle<DirectionalLightData>;

#[derive(Debug, Clone, Copy)]
pub struct SpotLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Srgb,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
}

#[derive(Debug)]
pub struct SpotLightData {
    pub spot_light: SpotLight,
}
pub type SpotLightHandle = handle_map::Handle<SpotLightData>;

// Basically the readonly (for users) parts of the EngineState,
// separated beacause the readonly crate prevents destructuring and partial borrows
// of the struct for users but this is the indented usage of EngineState
#[derive(Default, Debug)]
#[utils::readonly::make]
pub struct EngineStateResources {
    pub static_meshes_map: handle_map::HandleMap<StaticMeshData>,
    pub directional_lights_map: handle_map::HandleMap<DirectionalLightData>,
    pub spot_lights_map: handle_map::HandleMap<SpotLightData>,

    pub world_bind_group_layout: BindGroupLayoutHandle,
    pub object_bind_group_layout: BindGroupLayoutHandle,
}

impl EngineStateResources {
    
}

#[derive(Debug)]
pub struct EngineState {
    pub kernel: GraphicsKernel,
    pub materials: MaterialsStore,
    pub camera: Option<Camera>,

    pub resources: EngineStateResources,
}

impl EngineState {
    pub fn new(mut kernel: GraphicsKernel) -> Self {
        let world_bind_group_layout = kernel.create_bind_group_layout()
            .label("World Bind Group Layout")
            // world uniforms
            .entry().binding(0).uniform_buffer().add()
            // light array
            .entry().binding(1).uniform_buffer().add()
            .create();
        let object_bind_group_layout = kernel.create_bind_group_layout()
            .label("Object Bind Group Layout")
            .entry().binding(0).uniform_buffer().add()
            .create();

        let mut this = Self {
            kernel,
            materials: default(),
            camera: default(),

            resources: EngineStateResources {
                world_bind_group_layout,
                object_bind_group_layout,

                ..default()
            },
        };

        let factory = pbr_material::create_factory(&mut this);
        this.materials.register(factory);
        let factory = hdr_tonemapper_material::create_factory(&mut this);
        this.materials.register(factory);

        this
    }

    pub fn static_meshes(&self) -> impl Iterator<Item = (StaticMeshHandle, &StaticMesh)> + ExactSizeIterator {
        self.resources.static_meshes_map.iter().map(|(handle, data)| (handle.into(), &data.mesh))
    }

    pub fn directional_lights(&self) -> impl Iterator<Item = (DirectionalLightHandle, &DirectionalLight)> + ExactSizeIterator {
        self.resources.directional_lights_map.iter().map(|(handle, data)| (handle.into(), &data.directional_light))
    }

    pub fn spot_lights(&self) -> impl Iterator<Item = (SpotLightHandle, &SpotLight)> + ExactSizeIterator {
        self.resources.spot_lights_map.iter().map(|(handle, data)| (handle.into(), &data.spot_light))
    }

    pub fn insert_static_mesh(&mut self, mesh: StaticMesh) -> StaticMeshHandle {
        let kernel = &mut self.kernel;

        let transform: Mat4 = mesh.transform.into();
        let uniform_buffer = kernel.create_buffer()
            .data(&transform)
            .create();

        let object_bind_group = kernel.create_bind_group()
            .layout(self.resources.object_bind_group_layout)
            .entry(0, uniform_buffer)
            .create();

        self.resources.static_meshes_map.insert(StaticMeshData {
            mesh,
            object_bind_group,
        })
    }

    pub fn remove_static_mesh(&mut self, handle: StaticMeshHandle) -> Option<StaticMesh> {
        let _ = handle;
        // This leads to leaks (the uniform buffer) which without reference counted handles i dont
        // know how to fix (other than just included the uniform buffer in StaticMeshData)
        unimplemented!()
    }

    pub fn insert_directional_light(
        &mut self, directional_light: DirectionalLight
    ) -> DirectionalLightHandle {
        self.resources.directional_lights_map.insert(DirectionalLightData {
            directional_light,
        })
    }

    pub fn remove_directional_light(
        &mut self, handle: DirectionalLightHandle,
    ) -> Option<DirectionalLight> {
        self.resources.directional_lights_map.remove(handle.into()).map(|data| data.directional_light)
    }

    pub fn insert_spot_light(
        &mut self, spot_light: SpotLight
    ) -> SpotLightHandle {
        self.resources.spot_lights_map.insert(SpotLightData {
            spot_light,
        })
    }

    pub fn remove_spot_light(
        &mut self, handle: SpotLightHandle,
    ) -> Option<SpotLight> {
        self.resources.spot_lights_map.remove(handle.into()).map(|data| data.spot_light)
    }
}

impl Deref for EngineState {
    type Target = EngineStateResources;

    fn deref(&self) -> &Self::Target {
        &self.resources
    }
}
