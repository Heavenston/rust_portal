mod mesher;
pub use mesher::*;
mod items;
pub use items::*;

use utils::prelude::*;

use std::collections::HashMap;

use glam::{IVec3, UVec3};

pub const WORLD_SCALE: f32 = 2.0;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapCellMaterial {
    #[default]
    Metal,
    Concrete,
    UVCheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCell {
    Filled {
        materials: [MapCellMaterial; AxisDirection::VARIANT_COUNT],
    },
    Air,
}

impl MapCell {
    pub const DEFAULT: MapCell = MapCell::Filled {
        materials: [MapCellMaterial::Metal; AxisDirection::VARIANT_COUNT],
    };
}

impl Default for MapCell {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub const CHUNK_SIZE: UVec3 = UVec3::splat(8);
pub const CHUNK_LEN: usize = (CHUNK_SIZE.x * CHUNK_SIZE.y * CHUNK_SIZE.z) as usize;

pub fn chunk_of_cell(cell_pos: IVec3) -> IVec3 {
    cell_pos.div_euclid(CHUNK_SIZE.as_ivec3())
}

pub fn cell_pos_inside_chunk(cell_pos: IVec3) -> UVec3 {
    cell_pos.rem_euclid(CHUNK_SIZE.as_ivec3()).as_uvec3()
}

pub fn cell_idx_inside_chunk(cell_pos: UVec3) -> usize {
    (cell_pos.x +
     cell_pos.y * CHUNK_SIZE.x +
     cell_pos.z * CHUNK_SIZE.x * CHUNK_SIZE.y) as usize
}

#[derive(Debug, Clone)]
pub struct MapChunk<C = MapCell> {
    pub cells: [C; CHUNK_LEN],
}

impl<C> MapChunk<C> {
    pub fn cell_from_global_pos(&self, pos: IVec3) -> &C {
        self.cell(cell_pos_inside_chunk(pos))
    }

    pub fn cell_from_global_pos_mut(&mut self, pos: IVec3) -> &mut C {
        self.cell_mut(cell_pos_inside_chunk(pos))
    }

    pub fn cell(&self, pos: UVec3) -> &C {
        debug_assert!(pos.x < CHUNK_SIZE.x && pos.y < CHUNK_SIZE.y && pos.z < CHUNK_SIZE.z);
        &self.cells[cell_idx_inside_chunk(pos)]
    }

    pub fn cell_mut(&mut self, pos: UVec3) -> &mut C {
        debug_assert!(pos.x < CHUNK_SIZE.x && pos.y < CHUNK_SIZE.y && pos.z < CHUNK_SIZE.z);
        &mut self.cells[cell_idx_inside_chunk(pos)]
    }
}

impl<C: Default> Default for MapChunk<C> {
    fn default() -> Self {
        Self {
            cells: std::array::from_fn(|_| default()),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Map {
    pub chunks: HashMap<IVec3, MapChunk>,
    pub items: Vec<MapItem>,
}

impl Map {
    pub fn has_cell(&self, pos: IVec3) -> bool {
        self.chunks.contains_key(&chunk_of_cell(pos))
    }

    pub fn get_cell(&self, pos: IVec3) -> &MapCell {
        let Some(chunk) = self.chunks.get(&chunk_of_cell(pos))
        else { return &MapCell::DEFAULT };

        chunk.cell_from_global_pos(pos)
    }

    pub fn get_cell_mut(&mut self, pos: IVec3) -> &mut MapCell {
        let chunk = self.chunks.entry(chunk_of_cell(pos))
            .or_default();

        chunk.cell_from_global_pos_mut(pos)
    }

    /// Sets the wall on the given direction to the given material
    /// (if the wall exists)
    pub fn set_air_cell_wall(&mut self, pos: IVec3, wall: AxisDirection, material: MapCellMaterial) {
        if !matches!(self.get_cell(pos), MapCell::Air) {
            return;
        }

        let neighbor_pos = pos + wall.as_ivec3();
        if !self.has_cell(neighbor_pos) && material == MapCellMaterial::default() {
            return;
        }
        let neighbor_cell = self.get_cell_mut(neighbor_pos);

        match neighbor_cell {
            // nothing to do here, the selected wall does not exist
            MapCell::Air => (),
            MapCell::Filled { materials } => {
                materials[wall.opposit().idx()] = material;
            },
        }
    }

    pub fn set_air_cell_walls(&mut self, pos: IVec3, directions: u32, material: MapCellMaterial) {
        for dir in AxisDirection::iter_in_bitfield(directions) {
            self.set_air_cell_wall(pos, dir, material);
        }
    }

    pub fn iter_aabb(from: IVec3, to: IVec3, mut with: impl FnMut(IVec3)) {
        for x in from.x..=to.x {
            for y in from.y..=to.y {
                for z in from.z..=to.z {
                    with(IVec3::new(x, y, z));
                }
            }
        }
    }

    pub fn fill(&mut self, from: IVec3, to: IVec3, with: MapCell) {
        Self::iter_aabb(from, to, |pos| *self.get_cell_mut(pos) = with);
    }

    pub fn clear(&mut self, from: IVec3, to: IVec3, wall_directions: u32, material: MapCellMaterial) {
        self.fill(from, to, MapCell::Air);
        Self::iter_aabb(from, to, |pos| self.set_air_cell_walls(pos, wall_directions, material));
    }

    pub fn fill_material(&mut self, from: IVec3, to: IVec3, with: MapCellMaterial) {
        Self::iter_aabb(from, to, |pos| match self.get_cell_mut(pos) {
            MapCell::Filled { materials } => *materials = std::array::repeat(with),
            MapCell::Air => (),
        });
    }
}
