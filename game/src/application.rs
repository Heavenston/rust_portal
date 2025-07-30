use std::{collections::HashMap, iter::repeat_n};

use engine::{ color::{ LinearRgb, LinearRgba, Srgb, Srgba }, utils::* };
use glam::{ Affine3A, Mat4, Vec2, Vec3 };
use itertools::Itertools;
use crevice::std140::AsStd140 as _;

static SCENE_BYTES: &[u8] = include_bytes!("../../resources/simple_scene.glb");
// static SCENE_BYTES: &[u8] = include_bytes!("../../resources/just-sun.glb");
// static SCENE_BYTES: &[u8] = include_bytes!("../../resources/outdoor_scene.glb");

#[derive(Debug, Clone, Copy)]
struct Camera {
    transform: Affine3A,
    fov: f32,
    znear: f32,
    zfar: Option<f32>,
}

impl Camera {
    fn get_projection(&self, aspect_ratio: f32) -> Mat4 {
        if let Some(zfar) = self.zfar {
            Mat4::perspective_rh(self.fov, aspect_ratio, self.znear, zfar)
        } else {
            Mat4::perspective_infinite_rh(self.fov, aspect_ratio, self.znear)
        }
    }
}

#[derive(Default)]
struct BatchingStaticMesh {
    positions: Vec<Vec3>,
    texcoords: Vec<Vec2>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

struct GltfLoadingData<'a> {
    state: &'a mut engine::EngineState,
    gltf_buffers: Vec<gltf::buffer::Data>,
    gltf_textures: Vec<gltf::image::Data>,
    created_materials: HashMap<Option<usize>, engine::MaterialInstance>,
    batched_static_meshes: HashMap<Option<usize>, BatchingStaticMesh>,
}

pub struct Application {
    camera: Option<Camera>,
}

impl Application {
    pub fn new(state: &mut engine::EngineState) -> Self {
        let mut this = Application {
            camera: None,
        };
        this.init(state);
        this
    }

    fn init(&mut self, state: &mut engine::EngineState) {
        println!("Loading scene...");
        let (document, gltf_buffers, gltf_textures) = gltf::import_slice(SCENE_BYTES).expect("Could not load scene");

        let mut data = GltfLoadingData {
            state,
            gltf_buffers,
            gltf_textures,

            created_materials: default(),
            batched_static_meshes: default(),
        };
        println!("Creating materials...");
        for material in document.materials() {
            self.load_gltf_material(&mut data, material);
        }
        println!("Creating meshes...");
        for scene in document.scenes() {
            for node in scene.nodes() {
                self.load_gltf_node(&mut data, Mat4::default(), node);
            }
        }
        for (material_index, batching_mesh) in &data.batched_static_meshes {
            let Some(&material_instance) = data.created_materials.get(material_index)
            else {
                println!("Skipped a mesh with not material\n");
                continue;
            };
            self.commit_batching_mesh(data.state, material_instance, batching_mesh);
        }
        println!("Done loading");

        println!("Loaded: {} static meshes", state.static_meshes().len());
        println!("Loaded: {} directional lights", state.directional_lights().len());
        println!("Loaded: {} spot lights", state.spot_lights().len());

        if self.camera.is_none() {
            panic!("No camera was added");
        }
    }

    fn upload_texture(
        &mut self,
        renderer: &mut engine::Renderer,
        image_data: &gltf::image::Data,
    ) -> engine::TextureHandle {
        let texture_handle = renderer.create_texture()
            .width(image_data.width).height(image_data.height)
            .format(engine::TextureFormat::Rgba8UnormSrgb)
            .create();

        let data = match image_data.format {
            gltf::image::Format::R8G8B8 => {
                image_data.pixels.iter().copied()
                    .array_chunks::<3>()
                    .map(|[r, g, b]| [r, g, b, 255])
                    .flatten()
                    .collect_vec()
            },
            gltf::image::Format::R8G8B8A8 => {
                image_data.pixels.clone()
            },

            _ => panic!("Unsupported image gltf format '{:?}'", image_data.format),
        };

        renderer.write_texture(texture_handle, &data);

        texture_handle
    }

    fn load_gltf_material(
        &mut self,
        data: &mut GltfLoadingData,
        material: gltf::Material,
    ) -> engine::MaterialInstance {
        let state = &mut data.state;

        let material_index = material.index();
        let bmr = material.pbr_metallic_roughness();

        if let Some(&instance) = data.created_materials.get(&material_index) {
            return instance;
        }

        let diffuse_texture = bmr.base_color_texture()
            .map(|diffuse_texture| self.upload_texture(&mut state.renderer, &data.gltf_textures[diffuse_texture.texture().index()]));

        let material = state.materials.get_handle(&mut state.renderer, &engine::pbr_material::Parameters {
            enable_diffuse_texture: bmr.base_color_texture().is_some(),
        });
        let material_uniform_buffer = state.renderer.create_buffer()
            .size(engine::pbr_material::MaterialUniforms::std140_size_static() as u64)
            .data(engine::pbr_material::MaterialUniforms {
                base_color: LinearRgba::from_array(bmr.base_color_factor()),
                metallic: bmr.metallic_factor(),
                roughness: bmr.roughness_factor(),
            }.as_std140().as_bytes())
            .create();
        let layout = state.renderer.get_pipeline_bind_group_layouts(state.materials.get(material).pipeline)[2];
        let mut bind_group = state.renderer.create_bind_group()
            .layout(layout)
            .entry(0, material_uniform_buffer);
        if let Some(diffuse_texture) = diffuse_texture {
            bind_group = bind_group.entry(1, diffuse_texture).sampler(2);
        }
        let bind_group = bind_group.create();
        let instance = engine::MaterialInstance { material, bind_group };
        data.created_materials.insert(material_index, instance);
        instance
    }

    fn load_gltf_primitive(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        primitive: gltf::Primitive,
    ) {
        let reader = primitive.reader(|buffer| data.gltf_buffers.get(buffer.index()).map(|p| &**p));
        let batched_static_mesh = data.batched_static_meshes.entry(primitive.material().index()).or_default();

        debug_assert_eq!(batched_static_mesh.positions.len(), batched_static_mesh.texcoords.len());
        debug_assert_eq!(batched_static_mesh.texcoords.len(), batched_static_mesh.normals.len());
        let indices_offset = batched_static_mesh.positions.len() as u32;

        let vertex_count = reader.read_positions().unwrap().count();

        batched_static_mesh.positions.extend(
            reader.read_positions().expect("Mesh without positions?")
            .map(Vec3::from_array)
            .map(|point| transform.transform_point3(point))
        );
        if let Some(tex_coords) = reader.read_tex_coords(0) {
            batched_static_mesh.texcoords.extend(
                tex_coords.into_f32().map(Vec2::from_array)
            );
        }
        else {
            batched_static_mesh.texcoords.extend(
                repeat_n(Vec2::ZERO, vertex_count)
            );
        }
        batched_static_mesh.normals.extend(
            reader.read_normals().expect("Mesh without normals?")
            .map(Vec3::from_array)
            .map(|point| transform.transform_vector3(point))
        );
        batched_static_mesh.indices.extend(
            reader.read_indices().expect("Mesh without indices?")
            .into_u32()
            .map(|idx| idx + indices_offset)
        );
    }

    fn load_gltf_mesh(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        mesh: gltf::Mesh,
    ) {
        for primitive in mesh.primitives() {
            self.load_gltf_primitive(data, transform, primitive);
        }
    }

    fn load_gltf_light(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        light: gltf::khr_lights_punctual::Light,
    ) {
        let position: Vec3 = transform.translation.into();
        let direction = transform.transform_vector3(Vec3::NEG_Z);
        // FIXME: Should not be hard coded ?
        // let intensity = 1.;
        let intensity = light.intensity() * 0.001;
        let color: Srgb = LinearRgb::from_array(light.color()).into();

        use gltf::khr_lights_punctual::Kind;
        match light.kind() {
            Kind::Directional => {
                data.state.insert_directional_light(engine::DirectionalLight {
                    direction,
                    intensity,
                    color,
                });
            },
            Kind::Point => {
                panic!("Unsupported point light!");
            },
            Kind::Spot { inner_cone_angle, outer_cone_angle } => {
                data.state.insert_spot_light(engine::SpotLight {
                    position,
                    direction,
                    intensity,
                    color,
                    inner_cone_angle,
                    outer_cone_angle,
                });
            },
        }

    }

    fn commit_batching_mesh(
        &mut self,
        state: &mut engine::EngineState,
        material_instance: engine::MaterialInstance,
        batching_mesh: &BatchingStaticMesh,
    ) {
        let positions_bytes = bytemuck::cast_slice::<_, u8>(batching_mesh.positions.as_slice());
        let texcoords_bytes = bytemuck::cast_slice::<_, u8>(batching_mesh.texcoords.as_slice());
        let normals_bytes = bytemuck::cast_slice::<_, u8>(batching_mesh.normals.as_slice());
        let indices_bytes = bytemuck::cast_slice::<_, u8>(batching_mesh.indices.as_slice());

        let positions_buffer = state.renderer.create_buffer()
            .size(positions_bytes.len().try_into().expect("no overflow"))
            .data(&positions_bytes)
            .create();
        let texcoords_buffer = state.renderer.create_buffer()
            .size(texcoords_bytes.len().try_into().expect("no overflow"))
            .data(&texcoords_bytes)
            .create();
        let normals_buffer = state.renderer.create_buffer()
            .size(normals_bytes.len().try_into().expect("no overflow"))
            .data(&normals_bytes)
            .create();
        let index_buffer = state.renderer.create_buffer()
            .size(indices_bytes.len().try_into().expect("no overflow"))
            .data(&indices_bytes)
            .create();

        state.insert_static_mesh(engine::StaticMesh {
            transform: default(),
            positions_buffer,
            texcoords_buffer,
            normals_buffer,
            index_buffer,
            vertex_count: batching_mesh.indices.len().try_into().expect("No overflow"),
            material_instance,
        }.into());
    }

    fn load_gltf_node(
        &mut self,
        data: &mut GltfLoadingData,
        parent_transform: Mat4,
        node: gltf::Node,
    ) {
        let local_transform = Mat4::from_cols_array(
            &flatten_array::<f32, 4, 4>(node.transform().matrix())
        );
        let global_transform = parent_transform * local_transform;
        let global_affine = Affine3A::from_mat4(global_transform);

        if let Some(light) = node.light() {
            self.load_gltf_light(data, global_affine, light);
        }

        if let Some(mesh) = node.mesh() {
            self.load_gltf_mesh(data, global_affine, mesh);
        }

        if self.camera.is_none() &&
            let Some(camera) = node.camera() &&
            let gltf::camera::Projection::Perspective(perspective) = camera.projection()
        {
            self.camera = Some(Camera {
                transform: global_affine,
                fov: perspective.yfov(),
                znear: perspective.znear(),
                zfar: perspective.zfar(),
            });
        }

        for child_node in node.children() {
            self.load_gltf_node(data, global_transform, child_node);
        }
    }
}

impl engine::Application for Application {
    fn pre_frame(&mut self, state: &mut engine::EngineState, dt: f32) {
        println!("Frame {dt}s (~{}fps)!", 1. / dt);

        if let Some(camera) = &mut self.camera {
            // let rotation = Affine3A::from_rotation_y(dt);
            // camera.transform = rotation * camera.transform;

            state.camera = Some(engine::Camera {
                transform: camera.transform,
                projection: camera.get_projection(state.renderer.aspect_ration()),
                clear_color: Srgba::BLACK,
            });
        }

    }
}

