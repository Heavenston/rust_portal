use utils::{ default, handle_map };
use pgk::{
    color::{ Srgb, Srgba },
    BindGroupLayoutHandle,
    Renderer, BufferHandle, BindGroupHandle,
};
use crate::*;

use glam::{ Affine3A, Mat4, Vec3 };
use derive_more::{ From, Into };

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
    pub fn delete_buffers(&self, renderer: &mut Renderer) {
        renderer.delete_buffer(self.positions_buffer);
        renderer.delete_buffer(self.texcoords_buffer);
        renderer.delete_buffer(self.index_buffer);
    }
}

#[derive(Debug)]
pub(super) struct StaticMeshData {
    pub mesh: StaticMesh,
    pub object_bind_group: BindGroupHandle,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Into, From)]
pub struct StaticMeshHandle(handle_map::Handle<StaticMeshData>);

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub intensity: f32,
    pub color: Srgb,
}

#[derive(Debug)]
pub(super) struct DirectionalLightData {
    pub directional_light: DirectionalLight,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Into, From)]
pub struct DirectionalLightHandle(handle_map::Handle<DirectionalLightData>);

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
pub(super) struct SpotLightData {
    pub spot_light: SpotLight,
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Into, From)]
pub struct SpotLightHandle(handle_map::Handle<SpotLightData>);

#[derive(Debug)]
pub struct EngineState {
    pub renderer: Renderer,

    pub camera: Option<Camera>,
    /// immutable other than for the public mut methods
    pub(super) static_meshes: handle_map::HandleMap<StaticMeshData>,
    pub(super) directional_lights: handle_map::HandleMap<DirectionalLightData>,
    pub(super) spot_lights: handle_map::HandleMap<SpotLightData>,

    pub(super) world_bind_group_layout: BindGroupLayoutHandle,
    pub(super) object_bind_group_layout: BindGroupLayoutHandle,

    pub materials: MaterialsStore,
}

impl EngineState {
    pub fn new(mut renderer: Renderer) -> Self {
        let world_bind_group_layout = renderer.create_bind_group_layout()
            // world uniforms
            .entry().binding(0).uniform_buffer().add()
            // light array
            .entry().binding(1).uniform_buffer().add()
            .create();
        let object_bind_group_layout = renderer.create_bind_group_layout()
            .entry().binding(0).uniform_buffer().add()
            .create();

        let mut this = Self {
            renderer,

            camera: default(),
            static_meshes: default(),
            directional_lights: default(),
            spot_lights: default(),

            world_bind_group_layout,
            object_bind_group_layout,

            materials: default(),
        };

        let factory = pbr_material::create_factory(&mut this);
        this.materials.register(factory);
        let factory = hdr_tonemapper_material::create_factory(&mut this);
        this.materials.register(factory);

        this
    }

    pub fn static_meshes(&self) -> impl Iterator<Item = (StaticMeshHandle, &StaticMesh)> + ExactSizeIterator {
        self.static_meshes.iter().map(|(handle, data)| (handle.into(), &data.mesh))
    }

    pub fn directional_lights(&self) -> impl Iterator<Item = (DirectionalLightHandle, &DirectionalLight)> + ExactSizeIterator {
        self.directional_lights.iter().map(|(handle, data)| (handle.into(), &data.directional_light))
    }

    pub fn spot_lights(&self) -> impl Iterator<Item = (SpotLightHandle, &SpotLight)> + ExactSizeIterator {
        self.spot_lights.iter().map(|(handle, data)| (handle.into(), &data.spot_light))
    }

    pub fn insert_static_mesh(&mut self, mesh: StaticMesh) -> StaticMeshHandle {
        let renderer = &mut self.renderer;

        let transform: Mat4 = mesh.transform.into();
        let uniform_buffer = renderer.create_buffer()
            .size(size_of::<Mat4>() as u64)
            .data(bytemuck::bytes_of(&transform))
            .create();

        let object_bind_group = renderer.create_bind_group()
            .layout(self.object_bind_group_layout)
            .entry(0, uniform_buffer)
            .create();

        self.static_meshes.insert(StaticMeshData {
            mesh,
            object_bind_group,
        }).into()
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
        self.directional_lights.insert(DirectionalLightData {
            directional_light,
        }).into()
    }

    pub fn remove_directional_light(
        &mut self, handle: DirectionalLightHandle,
    ) -> Option<DirectionalLight> {
        self.directional_lights.remove(handle.into()).map(|data| data.directional_light)
    }

    pub fn insert_spot_light(
        &mut self, spot_light: SpotLight
    ) -> SpotLightHandle {
        self.spot_lights.insert(SpotLightData {
            spot_light,
        }).into()
    }

    pub fn remove_spot_light(
        &mut self, handle: SpotLightHandle,
    ) -> Option<SpotLight> {
        self.spot_lights.remove(handle.into()).map(|data| data.spot_light)
    }
}
