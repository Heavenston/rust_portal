use crevice::std140::AsStd140;
use pgk::color::LinearRgb;
use utils::{default, itertools::Itertools};

use std::{ collections::HashMap, iter::{ repeat, repeat_n, zip } };

use glam::{ Vec2, Vec3 };


type Error = Box<dyn std::error::Error>;
type Result<R> = std::result::Result<R, Error>;

#[derive(Default)]
struct BspConstructingMesh {
    vertex_positions: Vec<Vec3>,
    vertex_normals: Vec<Vec3>,
    vertex_texcoords: Vec<Vec2>,
    indices: Vec<u32>,
}

#[derive(Default)]
struct Ctx {
    per_material: HashMap<i16, BspConstructingMesh>,
}

fn add_face_to_mesh(
    face: vbsp::Handle<'_, vbsp::Face>,
    mesh: &mut BspConstructingMesh,
    model_origin: Vec3,
) {
    let positions_start = mesh.vertex_positions.len();
    let first_index = positions_start as u32;
    mesh.vertex_positions.extend(
        face.vertices()
            .map(|v| v.position)
            .map(|pos| Vec3::new(pos.x, pos.y, pos.z))
            .map(|pos| pos + model_origin)
    );
    let positions_end = mesh.vertex_positions.len();
    let last_index = positions_end as u32;
    let vertex_count = positions_end - positions_start;

    assert!(vertex_count >= 3);

    let texture = face.texture();
    mesh.vertex_texcoords.extend(
        face.vertices()
            .map(|vertex| texture.uv(vertex.position))
            .map(Vec2::from_array)
    );

    let normal = face.normal();
    let normal = Vec3::new(normal.x, normal.y, normal.z);
    mesh.vertex_normals.extend(
        repeat_n(normal, vertex_count)
    );

    // triangulation by triangle fan using the indices
    mesh.indices.extend(
        (first_index + 1..last_index).tuple_windows::<(_, _)>()
        .map(|(b, c)| [c, b, first_index])
        .flatten()
    );
}

pub fn create_bsp_meshes(state: &mut engine::EngineState, bsp: &vbsp::Bsp) -> Result<()> {
    let mut ctx = Ctx::default();

    for model in bsp.models() {
        for face in model.faces() {
            let mesh = ctx.per_material.entry(face.texture_info).or_default();
            add_face_to_mesh(face, mesh, Vec3::new(model.origin.x, model.origin.y, model.origin.z));
        }
    }

    for (&texture_info, mesh) in &ctx.per_material {
        let texture_info = bsp.texture_info(texture_info as usize).expect("valid texture info");
        if texture_info.flags.intersects(vbsp::TextureFlags::SKIP | vbsp::TextureFlags::NODRAW) {
            continue;
        }

        let material = state.materials
            .get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
                enable_base_color_texture: false,
            });
        let material_data = state.materials.get(material);
        let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline)
            .expect("Valid pipeline handle");
        let bind_group_layout = pipeline_data.bind_group_layouts[2];
        let material_buffer = state.kernel.create_buffer()
            .data(&engine::pbr_material::MaterialUniforms {
                base_color: LinearRgb::from_array_u8(texture_info.debug_color())
                    .with_alpha(1.),
                metallic: 0.,
                roughness: 1.,
            }.as_std140())
            .create();
        let bind_group = state.kernel.create_bind_group()
            .layout(bind_group_layout)
            .entry(0, material_buffer)
            .create();

        let positions_buffer = state.kernel.create_buffer()
            .slice(&mesh.vertex_positions).create();
        let texcoords_buffer = state.kernel.create_buffer()
            .slice(&mesh.vertex_texcoords).create();
        let normals_buffer = state.kernel.create_buffer()
            .slice(&mesh.vertex_normals).create();
        let index_buffer = state.kernel.create_buffer()
            .slice(&mesh.indices).create();

        state.insert_mesh(engine::Mesh {
            transform: default(),
            positions_buffer,
            texcoords_buffer,
            normals_buffer,
            index_buffer,
            vertex_count: u32::try_from(mesh.vertex_positions.len()).expect("no overflow"),
            material_instance: engine::MaterialInstance {
                material,
                bind_group,
            },
        });
    }

    Ok(())
}
