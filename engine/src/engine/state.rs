use std::ops::Deref;

use crevice::std140::AsStd140;
use utils::prelude::*;
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

#[derive(Debug, Clone)]
pub struct Mesh {
    pub transform: Affine3A,
    pub vertex_buffers: Box<[(u32, BufferHandle)]>,
    /// list of u32
    pub index_buffer: BufferHandle,
    /// At most the size of the indices buffer
    pub vertex_count: u32,

    pub material_instance: MaterialInstance,
}

impl Mesh {
    pub fn delete_buffers(&self, kernel: &mut GraphicsKernel) {
        for &(_, buf) in &self.vertex_buffers {
            kernel.delete_buffer(buf);
        }
        kernel.delete_buffer(self.index_buffer);
    }
}

#[derive(Debug)]
pub struct MeshData {
    pub mesh: Mesh,
    pub object_bind_group: BindGroupHandle,
    pub uniform_buffer: BufferHandle,
}
pub type MeshHandle = handle_map::Handle<MeshData>;

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

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    pub position: Vec3,
    pub intensity: f32,
    pub color: Srgb,
}

#[derive(Debug)]
pub struct PointLightData {
    pub point_light: PointLight,
}
pub type PointLightHandle = handle_map::Handle<PointLightData>;

// Basically the readonly (for users) parts of the EngineState,
// separated beacause the readonly crate prevents destructuring and partial borrows
// of the struct for users but this is the indented usage of EngineState
#[derive(Default, Debug)]
#[utils::readonly::make]
pub struct EngineStateResources {
    pub meshes_map: handle_map::HandleMap<MeshData>,
    pub directional_lights_map: handle_map::HandleMap<DirectionalLightData>,
    pub spot_lights_map: handle_map::HandleMap<SpotLightData>,
    pub point_lights_map: handle_map::HandleMap<PointLightData>,

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

    pub input: input::EngineInputState,
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

            input: default(),
        };

        let factory = pbr_material::create_factory(&mut this);
        this.materials.register(factory);
        let factory = hdr_tonemapper_material::create_factory(&mut this);
        this.materials.register(factory);

        this
    }

    pub fn meshes(&self) -> impl Iterator<Item = (MeshHandle, &Mesh)> + ExactSizeIterator {
        self.resources.meshes_map.iter().map(|(handle, data)| (handle.into(), &data.mesh))
    }

    pub fn directional_lights(&self) -> impl Iterator<Item = (DirectionalLightHandle, &DirectionalLight)> + ExactSizeIterator {
        self.resources.directional_lights_map.iter().map(|(handle, data)| (handle.into(), &data.directional_light))
    }

    pub fn spot_lights(&self) -> impl Iterator<Item = (SpotLightHandle, &SpotLight)> + ExactSizeIterator {
        self.resources.spot_lights_map.iter().map(|(handle, data)| (handle.into(), &data.spot_light))
    }

    pub fn point_lights(&self) -> impl Iterator<Item = (PointLightHandle, &PointLight)> + ExactSizeIterator {
        self.resources.point_lights_map.iter().map(|(handle, data)| (handle.into(), &data.point_light))
    }

    pub fn insert_mesh(&mut self, mesh: Mesh) -> MeshHandle {
        let kernel = &mut self.kernel;

        let uniform_buffer = kernel.create_buffer()
            .data(&pbr_material::ObjectUniforms {
                model: mesh.transform.into(),
            }.as_std140())
            .create();

        let object_bind_group = kernel.create_bind_group()
            .layout(self.resources.object_bind_group_layout)
            .entry(0, uniform_buffer)
            .create();

        self.resources.meshes_map.insert(MeshData {
            mesh,
            uniform_buffer,
            object_bind_group,
        })
    }

    pub fn remove_mesh(&mut self, handle: MeshHandle) -> Option<Mesh> {
        let data = self.resources.meshes_map.remove(handle)?;

        data.mesh.delete_buffers(&mut self.kernel);
        self.kernel.delete_buffer(data.uniform_buffer);
        self.kernel.delete_bind_group(data.object_bind_group);

        Some(data.mesh)
    }

    /// Sets and writes the transform of a mesh to the GPU
    pub fn set_mesh_transform(&mut self, handle: MeshHandle, transform: Affine3A) {
        let data = self.resources.meshes_map.get_mut(handle).expect("Valid Mesh handle");

        data.mesh.transform = transform;
        self.kernel.write_buffer(data.uniform_buffer, 0, pbr_material::ObjectUniforms {
            model: transform.into(),
        }.as_std140().as_bytes());
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

    pub fn insert_point_light(
        &mut self, point_light: PointLight
    ) -> PointLightHandle {
        self.resources.point_lights_map.insert(PointLightData {
            point_light,
        })
    }

    pub fn remove_point_light(
        &mut self, handle: PointLightHandle,
    ) -> Option<PointLight> {
        self.resources.point_lights_map.remove(handle.into()).map(|data| data.point_light)
    }
}

impl Deref for EngineState {
    type Target = EngineStateResources;

    fn deref(&self) -> &Self::Target {
        &self.resources
    }
}
