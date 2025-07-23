use std::ops::Range;

use crate::{uid::Uid, utils::default, BufferHandle, CameraBindGroupHandle, ObjectBindGroupHandle};

use super::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPassResourceHandle(Uid);

pub struct RenderPassOperation2<'a> {
    renderer: &'a Renderer,
    renderpass: wgpu::RenderPass<'a>,

    last_camera_bind_group: Option<CameraBindGroupHandle>,
}

#[bon::bon]
impl<'a> RenderPassOperation2<'a> {
    /// Just drop self
    pub fn finish(self) {
        // To be clear, this function just drops self
        drop(self);
    }

    #[builder(
        finish_fn = draw
    )]
    pub fn draw_call(
        &mut self,
        #[builder(finish_fn)]
        indices: Range<u32>,
        #[builder(default = 0)]
        base_vertex: i32,
        #[builder(name = camera_bind_group)]
        camera_bind_group_handle: CameraBindGroupHandle,
        #[builder(name = object_bind_group)]
        object_bind_group_handle: ObjectBindGroupHandle,
        #[builder(name = positions_buffer)]
        positions_buffer_handle: BufferHandle,
        #[builder(name = index_buffer)]
        index_buffer_handle: BufferHandle,
    ) {
        let camera_bind_group: &wgpu::BindGroup =
            &self.renderer.resources.camera_bind_groups.get(camera_bind_group_handle)
            .expect("Invalid camera_bind_group_handle given").bind_group;
        let object_bind_group: &wgpu::BindGroup =
            &self.renderer.resources.object_bind_groups.get(object_bind_group_handle)
            .expect("Invalid object_bind_group_handle given").bind_group;
        let vertex_buffer: &wgpu::Buffer =
            &self.renderer.resources.buffers.get(positions_buffer_handle)
            .expect("Invalid vertex buffer handle given").buffer;
        let index_buffer: &wgpu::Buffer =
            &self.renderer.resources.buffers.get(index_buffer_handle)
            .expect("Invalid index buffer handle given").buffer;

        // pipeline is set on this object creation
        if self.last_camera_bind_group != Some(camera_bind_group_handle) {
            self.renderpass.set_bind_group(0, camera_bind_group, &[]);
            self.last_camera_bind_group = Some(camera_bind_group_handle);
        }
        self.renderpass.set_bind_group(1, object_bind_group, &[]);
        self.renderpass.set_vertex_buffer(0, vertex_buffer.slice(..));
        self.renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.renderpass.draw_indexed(indices, base_vertex, 0..1);
    }
}

struct RenderPassColorAttachmentInfo {
    texture_view_handle: RenderPassResourceHandle,
    color_clear: Option<wgpu::Color>,
    discard: bool,
}

#[bon::builder(
    builder_type(name = "AddRenderPassAttachment", vis = "pub"),
    start_fn(name = "start_add_color_attachment", vis = ""),
    finish_fn(name = "finish", vis = "pub")
)]
fn add_color_attachment<'a, 'b, PS>(
    #[builder(start_fn)]
    mut builder: RenderPassOperationBuilder<'a, 'b, PS>,
    texture_view_handle: RenderPassResourceHandle,
    color_clear: Option<wgpu::Color>,
    #[builder(default = false)]
    discard: bool,
) -> RenderPassOperationBuilder<'a, 'b, PS>
where PS: render_pass_operation_builder::State {
    builder.color_attachments.push(RenderPassColorAttachmentInfo {
        texture_view_handle,
        color_clear,
        discard,
    });

    builder
}

struct RenderPassDepthStencilAttachmentInfo {
    texture_view_handle: RenderPassResourceHandle,
    depth_ops: Option<wgpu::Operations<f32>>,
    stencil_ops: Option<wgpu::Operations<u32>>,
}

#[bon::builder(
    builder_type(name = "AddRenderPassDepthStencilAttachment", vis = "pub"),
    start_fn(name = "start_add_depth_stencil_attachment", vis = ""),
    finish_fn(name = "finish", vis = "pub")
)]
fn add_depth_stencil_attachment<'a, 'b, PS>(
    #[builder(start_fn)]
    mut builder: RenderPassOperationBuilder<'a, 'b, PS>,
    texture_view_handle: RenderPassResourceHandle,
    depth_ops: Option<wgpu::Operations<f32>>,
    stencil_ops: Option<wgpu::Operations<u32>>,
) -> RenderPassOperationBuilder<'a, 'b, PS>
where PS: render_pass_operation_builder::State {
    builder.depth_stencil_attachment = Some(RenderPassDepthStencilAttachmentInfo {
        texture_view_handle,
        depth_ops,
        stencil_ops,
    });

    builder
}

#[bon::builder(
    builder_type(name = "RenderPassOperationBuilder", vis = "pub"),
    start_fn(name = "start_build_render_pass_operation", vis = ""),
    finish_fn(name = "build", vis = "pub")
)]
fn build_render_pass_operation<'a, 'b>(
    #[builder(start_fn)]
    data: &'a mut RenderPassOperationData<'b>,
    #[builder(field)]
    color_attachments: Vec<RenderPassColorAttachmentInfo>,
    #[builder(field)]
    depth_stencil_attachment: Option<RenderPassDepthStencilAttachmentInfo>,
) -> RenderPassOperation2<'a> {
    let RenderPassOperationData { renderer, encoder, resources } = data;

    let color_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = color_attachments.iter()
        .map(|info| {
            wgpu::RenderPassColorAttachment {
                view: resources.get_texture_view(info.texture_view_handle)
                    .expect("Invalid color attachment texture view handle given"),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: info.color_clear.map(wgpu::LoadOp::Clear).unwrap_or(wgpu::LoadOp::Load),
                    store: if info.discard { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store },
                },
            }
        })
        .map(Some)
        .collect();
    
    let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &color_attachments,
        depth_stencil_attachment: depth_stencil_attachment
            .map(|info| wgpu::RenderPassDepthStencilAttachment {
                view: resources.get_texture_view(info.texture_view_handle)
                    .expect("Invalid color attachment texture view handle given"),
                depth_ops: info.depth_ops,
                stencil_ops: info.stencil_ops,
            }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    renderpass.set_pipeline(&renderer.default_3d_render_pipeline);

    RenderPassOperation2 {
        renderer,
        renderpass,

        last_camera_bind_group: None,
    }
}

impl<'a, 'b, S> RenderPassOperationBuilder<'a, 'b, S>
    where S: render_pass_operation_builder::State,
{
    pub fn color_attachment(self) -> AddRenderPassAttachment<'a, 'b, S> {
        start_add_color_attachment(self)
    }
}

struct PresentSurfaceResource {
    handle: RenderPassResourceHandle,
    texture: wgpu::SurfaceTexture,
    texture_view: wgpu::TextureView,
}

#[derive(Default)]
struct RenderPassResources {
    present_surface: Option<PresentSurfaceResource>,
}

impl RenderPassResources {
    /// Works with a texture or texture view handle
    pub fn get_texture_view(&self, handle: RenderPassResourceHandle) -> Option<&wgpu::TextureView> {
        if let Some(present_surface) = &self.present_surface && present_surface.handle == handle {
            return Some(&present_surface.texture_view);
        }

        None
    }
}

struct RenderPassOperationData<'a> {
    renderer: &'a mut Renderer,
    encoder: wgpu::CommandEncoder,
    resources: RenderPassResources,
}

pub struct RenderOperation<'a> {
    data: Option<RenderPassOperationData<'a>>,
}

impl<'a> RenderOperation<'a> {
    pub(super) fn new(renderer: &'a mut Renderer) -> Self {
        let encoder = renderer.device.create_command_encoder(&Default::default());

        Self {
            data: Some(RenderPassOperationData {
                renderer,
                encoder,
                resources: default(),
            }),
        }
    }

    fn submit(&mut self) {
        let Some(data) = self.data.take()
        else { unreachable!("Already submitted?") };
        if let Some(present_surface) = data.resources.present_surface {
            data.renderer.queue.submit([data.encoder.finish()]);
            data.renderer.window.pre_present_notify();
            present_surface.texture.present();
        }
    }

    /// Just drop self
    pub fn finish(self) {
        // To be clear, this function just drops self
        drop(self);
    }

    fn create_present_surface(&mut self) -> &mut PresentSurfaceResource {
        let data = self.data.as_mut().expect("Already submitted??");
        if data.resources.present_surface.is_none() {
            let texture = data.renderer
                .surface
                .get_current_texture()
                .expect("failed to acquire next swapchain texture");
            let texture_view = texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(data.renderer.surface_format.add_srgb_suffix()),
                    ..Default::default()
                });

            data.resources.present_surface = Some(PresentSurfaceResource {
                handle: RenderPassResourceHandle(default()),
                texture,
                texture_view,
            });
        }
        data.resources.present_surface.as_mut().unwrap()
    }

    pub fn render_pass(&mut self) -> RenderPassOperationBuilder<'_, 'a> {
        start_build_render_pass_operation(
            self.data.as_mut().expect("Already submitted??")
        )
    }

    pub fn using_present_texture(&mut self) -> RenderPassResourceHandle {
        self.create_present_surface().handle
    }
}

impl<'a> Drop for RenderOperation<'a> {
    fn drop(&mut self) {
        self.submit();
    }
}

