use crevice::std140::AsStd140;
use glam::{Vec2, Vec3};
use pgk::color::LinearRgba;

use crate::upload_image_resource;

use super::*;

struct MeshCtx<'a> {
    override_material: Option<MapMaterial>,
    model: &'a mut MapModel,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapMaterialPart {
    #[default]
    Wall,
    Floor,
    Ceiling,
}

impl From<AxisDirection> for MapMaterialPart {
    fn from(value: AxisDirection) -> Self {
        match value {
            AxisDirection::PosY => Self::Ceiling,
            AxisDirection::NegY => Self::Floor,
            _ => Self::Wall,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapMaterial(pub MapCellMaterial, pub MapMaterialPart);

impl MapMaterial {
    fn from_cell_material(
        material: MapCellMaterial, direction: AxisDirection
    ) -> Self {
        MapMaterial(material, direction.into())
    }

    fn upload(&self, state: &mut engine::EngineState) -> engine::MaterialInstance {
        use MapCellMaterial::*;
        use MapMaterialPart::*;

        let material_uniform_buffer = state.kernel.create_buffer()
            .data(&engine::pbr_material::MaterialUniforms {
                base_color: LinearRgba::new(1., 1., 1., 1.),
                metallic: match self.0 {
                    Metal => 1.,
                    Concrete => 0.,
                    UVCheck => 0.01,
                },
                roughness: match self.0 {
                    Metal => 0.6,
                    Concrete => 0.9,
                    UVCheck => 1.,
                },
            }.as_std140())
            .create();

        let texture_path = match (self.0, self.1) {
            (Metal, Wall) => "portal_1/metal/metalwall048b",
            (Metal, Floor | Ceiling) => "portal_1/metal/metal_modular_floor001",
            (Concrete, Wall) => "portal_1/concrete/concrete_modular_wall001a",
            (Concrete, Floor) => "portal_1/concrete/concrete_modular_floor001a",
            (Concrete, Ceiling) => "portal_1/concrete/concrete_modular_ceiling001a",
            (UVCheck, _) => "UV_checker_Map_byValle",
        };
        println!("Loading texture '{texture_path}'...");
        let color_texture_handle: Option<pgk::TextureHandle> = upload_image_resource(
            state, &format!("{texture_path}.png"), true
        );
        let normal_texture_handle: Option<pgk::TextureHandle> = upload_image_resource(
            state, &format!("{texture_path}_normal.png"), false
        );

        let material = state.materials.get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: color_texture_handle.is_some(),
            enable_normal_map_texture: normal_texture_handle.is_some(),
            unlit: self.0 == UVCheck,
        });
        let material_data = state.materials.get(material);
        let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline)
            .expect("Pipeline exists");
        let bind_group_layout = pipeline_data.bind_group_layouts[2];
        let mut bind_group = state.kernel.create_bind_group()
            .layout(bind_group_layout)
            .entry(0, material_uniform_buffer);
        if let Some(color_texture_handle) = color_texture_handle {
            bind_group = bind_group.entry(1, color_texture_handle).sampler(2);
        }
        if let Some(normal_texture_handle) = normal_texture_handle {
            bind_group = bind_group.entry(3, normal_texture_handle).sampler(4);
        }
        let bind_group = bind_group.create();

        engine::MaterialInstance {
            material,
            bind_group,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct MapMesh {
    pub material: MapMaterial,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub indices: Vec<u32>,
}

impl MapMesh {
    /// NOTE: The size in the given axis is ignored (a plane cannot have a size along
    /// its normal)
    fn create_plane(&mut self, pos: Vec3, size: Vec3, dir: AxisDirection) {
        let base_index = self.positions.len() as u32;
    
        match dir {
            AxisDirection::PosX => {
                let x = pos.x + size.x;
                self.positions.extend([
                    Vec3::new(x, pos.y,          pos.z),
                    Vec3::new(x, pos.y + size.y, pos.z),
                    Vec3::new(x, pos.y + size.y, pos.z + size.z),
                    Vec3::new(x, pos.y,          pos.z + size.z),
                ]);
                self.texcoords.extend([
                    Vec2::new(size.z, size.y),
                    Vec2::new(size.z, 0.),
                    Vec2::new(0.,     0.),
                    Vec2::new(0.,     size.y),
                ]);
            },
            AxisDirection::NegX => {
                let x = pos.x;
                self.positions.extend([
                    Vec3::new(x, pos.y + size.y, pos.z),
                    Vec3::new(x, pos.y,          pos.z),
                    Vec3::new(x, pos.y,          pos.z + size.z),
                    Vec3::new(x, pos.y + size.y, pos.z + size.z),
                ]);
                self.texcoords.extend([
                    Vec2::new(0.,     0.),
                    Vec2::new(0.,     size.y),
                    Vec2::new(size.z, size.y),
                    Vec2::new(size.z, 0.),
                ]);
            },
            AxisDirection::PosY => {
                let y = pos.y + size.y;
                self.positions.extend([
                    Vec3::new(pos.x,          y, pos.z),
                    Vec3::new(pos.x,          y, pos.z + size.z),
                    Vec3::new(pos.x + size.x, y, pos.z + size.z),
                    Vec3::new(pos.x + size.x, y, pos.z),
                ]);
                self.texcoords.extend([
                    Vec2::new(0.,     0.    ),
                    Vec2::new(0.,     size.z),
                    Vec2::new(size.x, size.z),
                    Vec2::new(size.x, 0.    ),
                ]);
            },
            AxisDirection::NegY => {
                let y = pos.y;
                self.positions.extend([
                    Vec3::new(pos.x,          y, pos.z),
                    Vec3::new(pos.x + size.x, y, pos.z),
                    Vec3::new(pos.x + size.x, y, pos.z + size.z),
                    Vec3::new(pos.x,          y, pos.z + size.z),
                ]);
                self.texcoords.extend([
                    Vec2::new(0.,     0.),
                    Vec2::new(size.x, 0.),
                    Vec2::new(size.x, size.z),
                    Vec2::new(0.,     size.z),
                ]);
            },
            AxisDirection::PosZ => {
                let z = pos.z + size.z;
                self.positions.extend([
                    Vec3::new(pos.x,          pos.y,          z),
                    Vec3::new(pos.x + size.x, pos.y,          z),
                    Vec3::new(pos.x + size.x, pos.y + size.y, z),
                    Vec3::new(pos.x,          pos.y + size.y, z),
                ]);
                self.texcoords.extend([
                    Vec2::new(0.,     size.y),
                    Vec2::new(size.x, size.y),
                    Vec2::new(size.x, 0.),
                    Vec2::new(0.,     0.),
                ]);
            },
            AxisDirection::NegZ => {
                let z = pos.z;
                self.positions.extend([
                    Vec3::new(pos.x,          pos.y + size.y, z),
                    Vec3::new(pos.x + size.x, pos.y + size.y, z),
                    Vec3::new(pos.x + size.x, pos.y,          z),
                    Vec3::new(pos.x,          pos.y,          z),
                ]);
                self.texcoords.extend([
                    Vec2::new(size.x, 0.),
                    Vec2::new(0.,     0.),
                    Vec2::new(0.,     size.y),
                    Vec2::new(size.x, size.y),
                ]);
            },
        }
    
        self.normals.extend([dir.as_ivec3().as_vec3(); 4]);
    
        self.indices.extend([
            base_index, base_index + 1, base_index + 2,
            base_index, base_index + 2, base_index + 3,
        ]);
    }

    fn upload(&self, state: &mut engine::EngineState) -> engine::MeshHandle {
        let tangent_data = pgk::utils::compute_all_tangents(
            &self.positions, &self.texcoords, &self.indices
        );

        let positions_buffer = state.kernel.create_buffer()
            .slice(&self.positions)
            .create();
        let texcoords_buffer = state.kernel.create_buffer()
            .slice(&self.texcoords)
            .create();
        let normals_buffer = state.kernel.create_buffer()
            .slice(&self.normals)
            .create();
        let tangents_buffer = state.kernel.create_buffer()
            .slice(&tangent_data.tangents)
            .create();
        let bitangents_buffer = state.kernel.create_buffer()
            .slice(&tangent_data.bitangents)
            .create();
        let index_buffer = state.kernel.create_buffer()
            .slice(&self.indices)
            .create();

        let material_instance = self.material.upload(state);

        state.insert_mesh(engine::Mesh {
            transform: default(),
            vertex_buffers: vec![
                (0, positions_buffer),
                (1, texcoords_buffer),
                (2, normals_buffer),
                (3, tangents_buffer),
                (4, bitangents_buffer),
            ].into_boxed_slice(),
            index_buffer,
            vertex_count: self.indices.len().try_into().expect("no overflow"),
            material_instance,
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct MapModel {
    pub meshes: Vec<MapMesh>,
}

impl MapModel {
    fn for_material(&mut self, material: MapMaterial) -> &mut MapMesh {
        if let Some(pos) = self.meshes.iter().position(|mesh| mesh.material == material) {
            &mut self.meshes[pos]
        }
        else {
            self.meshes.push(MapMesh {
                material,
                ..Default::default()
            });
            self.meshes.last_mut().expect("Just pushed")
        }
    }

    pub fn upload(&self, state: &mut engine::EngineState) -> Vec<engine::MeshHandle> {
        self.meshes.iter()
            .map(|mesh| mesh.upload(state))
            .collect()
    }
}

impl Map {
    fn mesh_cell_direction(&self, ctx: &mut MeshCtx, cell_pos: IVec3, dir: AxisDirection) {
        let MapCell::Filled { materials } = self.get_cell(cell_pos)
        else { return };

        let neighbor = self.get_cell(cell_pos + dir.as_ivec3());
        if !matches!(neighbor, MapCell::Air) {
            return;
        }

        let material = ctx.override_material
            .unwrap_or_else(|| MapMaterial::from_cell_material(materials[dir.idx()], dir));
        let mesh = ctx.model.for_material(material);

        mesh.create_plane(cell_pos.as_vec3(), Vec3::splat(1.), dir);
    }

    fn mesh_cell(&self, ctx: &mut MeshCtx, cell_pos: IVec3) {
        for dir in AxisDirection::VARIANTS {
            self.mesh_cell_direction(ctx, cell_pos, dir);
        }
    }

    fn mesh_chunk(&self, ctx: &mut MeshCtx, chunk_pos: IVec3) {
        for dz in 0..CHUNK_SIZE.z {
            for dy in 0..CHUNK_SIZE.y {
                for dx in 0..CHUNK_SIZE.x {
                    let dpos = UVec3::new(dx, dy, dz);
                    let cell_pos = chunk_pos * CHUNK_SIZE.as_ivec3() + dpos.as_ivec3();
                    self.mesh_cell(ctx, cell_pos);
                }
            }
        }
    }

    pub fn mesh(&self, override_material: Option<MapMaterial>) -> MapModel {
        let mut model = MapModel::default();

        let mut ctx = MeshCtx {
            override_material,
            model: &mut model,
        };

        for &chunk_pos in self.chunks.keys() {
            self.mesh_chunk(&mut ctx, chunk_pos);
            for dir in AxisDirection::VARIANTS {
                let neighbor_pos = chunk_pos + dir.as_ivec3();
                if !self.chunks.contains_key(&neighbor_pos) {
                    self.mesh_chunk(&mut ctx, neighbor_pos);
                }
            }
        }

        model
    }
}
