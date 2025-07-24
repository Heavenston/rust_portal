use engine::utils::*;
use glam::{Affine3A, Mat4, Vec2, Vec3, Vec4};
use itertools::Itertools;

static SCENE_BYTES: &[u8] = include_bytes!("../../resources/simple_scene.glb");

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

struct GltfLoadingData<'a> {
    state: &'a mut engine::EngineState,
    gltf_buffers: Vec<gltf::buffer::Data>,
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
        let (document, gltf_buffers, _) = gltf::import_slice(SCENE_BYTES).expect("Could not load scene");

        let mut data = GltfLoadingData {
            state,
            gltf_buffers,
        };
        for scene in document.scenes() {
            for node in scene.nodes() {
                self.load_gltf_node(&mut data, Mat4::default(), node);
            }
        }
    }

    fn load_gltf_mesh(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        mesh: gltf::Mesh,
    ) {
        let state = &mut *data.state;

        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| data.gltf_buffers.get(buffer.index()).map(|p| &**p));

            let positions = reader.read_positions().expect("Mesh without positions?")
                .map(Vec3::from_array)
                .collect_vec();
            let positions_bytes = bytemuck::cast_slice::<_, u8>(&positions);
            let texcoords = reader.read_tex_coords(0).expect("Mesh without texcoords?")
                .into_f32()
                .map(Vec2::from_array)
                .collect_vec();
            let texcoords_bytes = bytemuck::cast_slice::<_, u8>(&texcoords);
            let indices = reader.read_indices().expect("Mesh without indices is not supported")
                .into_u32()
                .collect_vec();
            let indices_bytes = bytemuck::cast_slice::<_, u8>(&indices);

            let positions_buffer = state.renderer.create_buffer()
                .size(positions_bytes.len().try_into().expect("no overflow"))
                .data(&positions_bytes)
                .create();
            let texcoords_buffer = state.renderer.create_buffer()
                .size(texcoords_bytes.len().try_into().expect("no overflow"))
                .data(&texcoords_bytes)
                .create();
            let index_buffer = state.renderer.create_buffer()
                .size(indices_bytes.len().try_into().expect("no overflow"))
                .data(&indices_bytes)
                .create();

            let material = state.materials.get_handle(&mut state.renderer, &engine::PbrMaterialParameters {
                enable_diffuse_texture: false,
            });
            let material_uniform_buffer = state.renderer.create_buffer()
                .size(size_of::<engine::PbrMaterialUniforms>() as u64)
                .data(bytemuck::bytes_of(&engine::PbrMaterialUniforms {
                    base_color: Vec4::new(1., 0., 0., 1.),
                }))
                .create();
            let layout = state.renderer.get_pipeline_bind_group_layouts(state.materials.get(material).pipeline)[2];
            let bind_group = state.renderer.create_bind_group()
                .layout(layout)
                .entry(0, material_uniform_buffer)
                .create();
            let material_instance = engine::MaterialInstance {
                material,
                bind_group,
            };

            state.insert_static_mesh(engine::StaticMesh {
                transform,
                positions_buffer,
                texcoords_buffer,
                index_buffer,
                vertex_count: indices.len().try_into().expect("No overflow"),
                material_instance,
            }.into());
        }
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
    fn update(&mut self, state: &mut engine::EngineState, _dt: f32) {
        state.camera = self.camera.map(|camera| {
            engine::Camera {
                transform: camera.transform,
                projection: camera.get_projection(state.renderer.aspect_ration()),
                clear_color: wgpu::Color::BLACK,
            }
        });
    }
}

