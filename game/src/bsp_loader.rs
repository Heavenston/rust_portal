use crevice::std140::AsStd140;
use pgk::color::{LinearRgb, LinearRgba};
use utils::{default, itertools::Itertools as _};

use std::{ collections::HashMap, iter::repeat_n };

use glam::{ Affine3A, Mat3A, Vec2, Vec3, Vec3A };

type Error = Box<dyn std::error::Error>;
type Result<R, E = Error> = std::result::Result<R, E>;

pub const FROM_BSP_TRANSFORM: Affine3A = Affine3A {
    matrix3: Mat3A::from_cols_array(&[
        1., 0., 0.,
        0., 0., 1.,
        0., 1., 0.,
    ]),
    translation: Vec3A::new(0., 0., 0.),
};
pub const TO_BSP_TRANSFORM: Affine3A = Affine3A {
    matrix3: Mat3A::from_cols_array(&[
        1., 0., 0.,
        0., 0., 1.,
        0., 1., 0.,
    ]),
    translation: Vec3A::new(0., 0., 0.),
};

fn get_constant_color_material(
    state: &mut engine::EngineState,
    color: LinearRgba,
) -> engine::MaterialInstance {
    let material_handle = state.materials
        .get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: false,
            unlit: true,
        });
    let material_data = state.materials.get(material_handle);
    let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline)
        .expect("Valid pipeline handle");
    let bind_group_layout = pipeline_data.bind_group_layouts[2];
    let material_buffer = state.kernel.create_buffer()
        .data(&engine::pbr_material::MaterialUniforms {
            base_color: color,
            metallic: 0.,
            roughness: 1.,
        }.as_std140())
        .create();
    let bind_group = state.kernel.create_bind_group()
        .layout(bind_group_layout)
        .entry(0, material_buffer)
        .create();

    engine::MaterialInstance {
        material: material_handle,
        bind_group,
    }
}

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
        face.vertices().map(|v| v.position)
            .map(|vbsp::Vector { x, y, z }| Vec3 { x, y, z })
            .map(|pos| pos + model_origin)
            .map(|v| FROM_BSP_TRANSFORM.transform_point3(v))
    );
    let positions_end = mesh.vertex_positions.len();
    let last_index = positions_end as u32;
    let vertex_count = positions_end - positions_start;

    assert!(vertex_count >= 3);

    let texture = face.texture();
    mesh.vertex_texcoords.extend(
        mesh.vertex_positions[positions_start..positions_end].iter().copied()
            .map(|v| TO_BSP_TRANSFORM.transform_point3(v))
            .map(|Vec3 { x, y, z }| vbsp::Vector { x, y, z })
            .map(|vertex| texture.uv(vertex))
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
        .map(|(b, c)| [first_index, b, c])
        .flatten()
    );
}

fn load_vtf_texture(
    state: &mut engine::EngineState,
    vtf: &vtf::vtf::VTF,
) -> Result<pgk::TextureHandle> {
    // println!("{} of {}x{}", vtf.highres_image.format, vtf.highres_image.width, vtf.highres_image.height);

    let rgba8_data = match vtf.highres_image.format {
        // the `vtf` lib seems to break this conversion somehow
        vtf::ImageFormat::Bgr888 => vtf.highres_image.get_frame(0)?.iter().copied()
            .array_chunks()
            .flat_map(|[b, g, r]| [r, g, b, 255])
            .collect(),
        _ => vtf.highres_image.decode(0)?.into_rgba8().into_vec(),
    };

    let texture_handle = state.kernel.create_texture()
        .width(vtf.highres_image.width.into())
        .height(vtf.highres_image.height.into())
        .usages(pgk::TextureUsages {
            copy_dst: true,
            texture_binding: true,
            ..default()
        })
        .format(pgk::TextureFormat::Rgba8UnormSrgb)
        .create()
    ;
    state.kernel.write_texture(texture_handle, &rgba8_data);

    Ok(texture_handle)
}

fn load_light_mapped_generic_material(
    state: &mut engine::EngineState,
    _bsp: &vbsp::Bsp,
    vpk: &valve_pak::VPK,
    material: &vmt_parser::material::LightMappedGenericMaterial,
) -> Result<Option<engine::MaterialInstance>> {
    let color_texture_path = format!("materials/{}.vtf", material.base_texture);
    let color_texture_bytes = vpk.get_file(&color_texture_path)?
        .read_all()?;
    // println!("{}",
    //     vpk.list_files().iter().filter(|path| path.contains(&material.base_texture) && path.ends_with(".vtf"))
    //     .join(", ")
    // );
    let vtf = vtf::from_bytes(&color_texture_bytes).unwrap();

    let texture = load_vtf_texture(state, &vtf)?;

    let material_handle = state.materials
        .get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: true,
            unlit: false,
        });
    let material_data = state.materials.get(material_handle);
    let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline)
        .expect("Valid pipeline handle");
    let bind_group_layout = pipeline_data.bind_group_layouts[2];
    let material_buffer = state.kernel.create_buffer()
        .data(&engine::pbr_material::MaterialUniforms {
            base_color: LinearRgb::from_array(material.color.0)
                .with_alpha(1.),
            metallic: 0.,
            roughness: 1.,
        }.as_std140())
        .create();
    let bind_group = state.kernel.create_bind_group()
        .layout(bind_group_layout)
        .entry(0, material_buffer)
        .entry(1, texture).sampler(2)
        .create();

    Ok(Some(engine::MaterialInstance {
        material: material_handle,
        bind_group,
    }))
}

fn load_material(
    state: &mut engine::EngineState,
    bsp: &vbsp::Bsp,
    vpk: &valve_pak::VPK,
    texture_info: &vbsp::Handle<'_, vbsp::TextureInfo>,
) -> Result<Option<engine::MaterialInstance>> {
    let name = texture_info.name();
    let vmt_path = format!("materials/{name}.vmt").to_lowercase();
    let vmt_bytes = if let Some(vmt_bytes) = bsp.pack.get(&vmt_path)? {
        vmt_bytes
    } else if let Ok(mut file) = vpk.get_file(&vmt_path) {
        file.read_all()?
    } else {
        println!("Could not find .vmt at path {vmt_path}");
        return Ok(None);
    };

    let material = vmt_parser::from_str(str::from_utf8(&vmt_bytes)?)?
        .resolve(|path| -> Result<String> {
            Ok(vpk.get_file(path)?.read_all_string()?)
        });
    let material = match material {
        Result::Ok(material) => material,
        Result::Err(e) => {
            println!("Could not resolve a material: {e}");
            return Ok(None);
        },
    };

    // print!("{name} is:\n\t");
    match material {
        vmt_parser::material::Material::LightMappedGeneric(material) => {
            load_light_mapped_generic_material(state, bsp, vpk, &material)
        },
        vmt_parser::material::Material::UnlitGeneric(unlit) => {
            println!("Unlit (todo)");
            return Ok(None);
        },
        vmt_parser::material::Material::Patch(_) => unreachable!(),
        m => {
            println!("Unknown material: {m:?}");
            return Ok(None);
        },
    }
}

pub fn create_bsp_meshes(
    state: &mut engine::EngineState,
    bsp: &vbsp::Bsp,
    vpk: &valve_pak::VPK,
) -> Result<()> {
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

        let material_instance = load_material(state, bsp, vpk, &texture_info)?
            .unwrap_or_else(|| get_constant_color_material(state, LinearRgb::from_array_u8(texture_info.debug_color())
                .with_alpha(1.)));

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
            vertex_count: u32::try_from(mesh.indices.len()).expect("no overflow"),
            material_instance,
        });
    }

    Ok(())
}
