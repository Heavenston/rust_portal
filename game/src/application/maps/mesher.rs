use crevice::std140::AsStd140;
use glam::{Vec2, Vec3};
use pgk::color::LinearRgba;

use crate::Resources;

use super::*;

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
pub struct MapMaterial(MapCellMaterial, MapMaterialPart);

impl MapMaterial {
    fn from_cell_material(
        material: MapCellMaterial, direction: AxisDirection
    ) -> Self {
        MapMaterial(material, direction.into())
    }

    fn upload(&self, state: &mut engine::EngineState) -> engine::MaterialInstance {
        let material_uniform_buffer = state.kernel.create_buffer()
            .data(&engine::pbr_material::MaterialUniforms {
                base_color: LinearRgba::new(1., 1., 1., 1.),
                metallic: 0.5,
                roughness: 0.5,
            }.as_std140())
            .create();

        use MapCellMaterial::*;
        use MapMaterialPart::*;
        let texture_path = match (self.0, self.1) {
            (Metal, Wall) => "portal_1/metal/metalwall048b.png",
            (Metal, Floor | Ceiling) => "portal_1/metal/metal_modular_floor001.png",
            (Concrete, Wall) => "portal_1/concrete/concrete_modular_wall001a.png",
            (Concrete, Floor) => "portal_1/concrete/concrete_modular_floor001a.png",
            (Concrete, Ceiling) => "portal_1/concrete/concrete_modular_ceiling001a.png",
        };
        let texture_file = Resources::get(texture_path).expect("File exists");
        let texture_data = &*texture_file.data;
        let texture_image = image::load_from_memory(&texture_data).expect("Could not decode image")
            .to_rgba8();

        let texture_handle = state.kernel.create_texture()
            .usages(pgk::TextureUsages {
                copy_dst: true,
                texture_binding: true,
                ..default()
            })
            .format(pgk::TextureFormat::Rgba8UnormSrgb)
            .width(texture_image.width()).height(texture_image.height())
            .create();
        state.kernel.write_texture(texture_handle, &texture_image.as_raw());

        let material = state.materials.get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: true,
            unlit: false,
        });
        let material_data = state.materials.get(material);
        let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline)
            .expect("Pipeline exists");
        let bind_group_layout = pipeline_data.bind_group_layouts[2];
        let bind_group = state.kernel.create_bind_group()
            .layout(bind_group_layout)
            .entry(0, material_uniform_buffer)
            .entry(1, texture_handle).sampler(2)
            .create();

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
    
        let normal = match dir {
            AxisDirection::PosX => {
                let x = pos.x + size.x;
                self.positions.extend([
                    Vec3::new(x, pos.y,          pos.z),
                    Vec3::new(x, pos.y + size.y, pos.z),
                    Vec3::new(x, pos.y + size.y, pos.z + size.z),
                    Vec3::new(x, pos.y,          pos.z + size.z),
                ]);
                self.texcoords.extend([
                    Vec2::new(0.,     0.),
                    Vec2::new(0.,     size.y),
                    Vec2::new(size.z, size.y),
                    Vec2::new(size.z, 0.),
                ]);
                Vec3::NEG_X
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
                    Vec2::new(0.,     size.y),
                    Vec2::new(0.,     0.),
                    Vec2::new(size.z, 0.),
                    Vec2::new(size.z, size.y),
                ]);
                Vec3::X
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
                Vec3::NEG_Y
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
                Vec3::Y
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
                    Vec2::new(size.x, 0.),
                    Vec2::new(0.,     0.),
                    Vec2::new(0.,     size.y),
                    Vec2::new(size.x, size.y),
                ]);
                Vec3::NEG_Z
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
                    Vec2::new(size.x, size.y),
                    Vec2::new(0.,     size.y),
                    Vec2::new(0.,     0.),
                    Vec2::new(size.x, 0.),
                ]);
                Vec3::Z
            },
        };
    
        self.normals.extend([normal; 4]);
    
        // self.texcoords.extend([
        //     Vec2::new(0.0,         0.0),
        //     Vec2::new(tex_scale.x, 0.0),
        //     Vec2::new(tex_scale.x, tex_scale.y),
        //     Vec2::new(0.0,         tex_scale.y),
        // ]);
    
        self.indices.extend([
            base_index, base_index + 1, base_index + 2,
            base_index, base_index + 2, base_index + 3,
        ]);
    }

    fn upload(&self, state: &mut engine::EngineState) -> engine::MeshHandle {
        let positions_buffer = state.kernel.create_buffer()
            .slice(&self.positions)
            .create();
        let texcoords_buffer = state.kernel.create_buffer()
            .slice(&self.texcoords)
            .create();
        let normals_buffer = state.kernel.create_buffer()
            .slice(&self.normals)
            .create();
        let index_buffer = state.kernel.create_buffer()
            .slice(&self.indices)
            .create();

        let material_instance = self.material.upload(state);

        state.insert_mesh(engine::Mesh {
            transform: default(),
            positions_buffer,
            texcoords_buffer,
            normals_buffer,
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

    pub fn upload(&self, state: &mut engine::EngineState) {
        for mesh in &self.meshes {
            mesh.upload(state);
        }
    }
}

impl Map {
    fn mesh_cell_direction(&self, cell_pos: IVec3, dir: AxisDirection, model: &mut MapModel) {
        let MapCell::Filled { materials } = self.get_cell(cell_pos)
        else { return };

        let neighbor = self.get_cell(cell_pos + dir.as_diff());
        if !matches!(neighbor, MapCell::Air) {
            return;
        }

        let material = materials[dir.idx()];
        let mesh = model.for_material(MapMaterial::from_cell_material(material, dir));

        mesh.create_plane(cell_pos.as_vec3(), Vec3::splat(1.), dir);
    }

    fn mesh_cell(&self, cell_pos: IVec3, model: &mut MapModel) {
        for dir in AxisDirection::VARIANTS {
            self.mesh_cell_direction(cell_pos, dir, model);
        }
    }

    fn mesh_chunk(&self, chunk_pos: IVec3, model: &mut MapModel) {
        for dz in 0..CHUNK_SIZE.z {
            for dy in 0..CHUNK_SIZE.y {
                for dx in 0..CHUNK_SIZE.x {
                    let dpos = UVec3::new(dx, dy, dz);
                    let cell_pos = chunk_pos * CHUNK_SIZE.as_ivec3() + dpos.as_ivec3();
                    self.mesh_cell(cell_pos, model);
                }
            }
        }
    }

    pub fn mesh(&self) -> MapModel {
        let mut model = MapModel::default();

        for &chunk_pos in self.chunks.keys() {
            self.mesh_chunk(chunk_pos, &mut model);
            for dir in AxisDirection::VARIANTS {
                let neighbor_pos = chunk_pos + dir.as_diff();
                if !self.chunks.contains_key(&neighbor_pos) {
                    self.mesh_chunk(neighbor_pos, &mut model);
                }
            }
        }

        model
    }
}
