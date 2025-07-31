use std::ops::Range;

use crate::{ color::Srgba, * };
use utils::*;

use super::Renderer;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPassResourceHandle(Uid);

pub struct RenderPassOperation2<'a> {
    renderer: &'a Renderer,
    renderpass: wgpu::RenderPass<'a>,
}

impl<'a> RenderPassOperation2<'a> {
    /// Just drop self
    pub fn finish(self) {
        // To be clear, this function just drops self
        drop(self);
    }

    pub fn set_pipeline(
        &mut self,
        pipeline: PipelineHandle,
    ) {
        let pipeline = &self.renderer.resources.pipelines.get(pipeline)
            .expect("Invalid pipeline handle given").pipeline;
        self.renderpass.set_pipeline(pipeline);
    }

    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: BindGroupHandle,
    ) {
        let bind_group = &self.renderer.resources.bind_groups.get(bind_group)
            .expect("Invalid pipeline handle given").bind_group;
        self.renderpass.set_bind_group(index, bind_group, &[]);
    }

    pub fn set_vertex_buffer(
        &mut self,
        index: u32,
        buffer: BufferHandle,
    ) {
        let buffer = &self.renderer.resources.buffers.get(buffer)
            .expect("Invalid pipeline handle given").buffer;
        self.renderpass.set_vertex_buffer(index, buffer.slice(..));
    }

    pub fn set_index_buffer(
        &mut self,
        buffer: BufferHandle,
    ) {
        let buffer = &self.renderer.resources.buffers.get(buffer)
            .expect("Invalid pipeline handle given").buffer;
        self.renderpass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
    }

    pub fn draw_indexed(
        &mut self,
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    ) {
        self.renderpass.draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw(
        &mut self,
        vertices: Range<u32>,
        instances: Range<u32>,
    ) {
        self.renderpass.draw(vertices, instances);
    }
}

struct RenderPassColorAttachmentInfo {
    texture_view_handle: RenderPassResourceHandle,
    clear_color: Option<Srgba>,
    discard: bool,
}

#[bon::builder(
    builder_type(name = AddRenderPassColorAttachment, vis = "pub"),
    start_fn(name = start_add_color_attachment, vis = ""),
    finish_fn(name = finish, vis = "pub")
)]
fn add_color_attachment<'a, 'b, PS>(
    #[builder(start_fn)]
    mut builder: RenderPassOperationBuilder<'a, 'b, PS>,
    texture_view_handle: RenderPassResourceHandle,
    clear_color: Option<Srgba>,
    #[builder(default = false)]
    discard: bool,
) -> RenderPassOperationBuilder<'a, 'b, PS>
where PS: render_pass_operation_builder::State {
    builder.color_attachments.push(RenderPassColorAttachmentInfo {
        texture_view_handle,
        clear_color,
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
    builder_type(name = AddRenderPassDepthStencilAttachment, vis = "pub"),
    start_fn(name = start_add_depth_stencil_attachment, vis = ""),
    finish_fn(name = finish, vis = "pub")
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
                view: resources.get_texture_view(renderer, info.texture_view_handle)
                    .expect("Invalid color attachment texture view handle given"),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: info.clear_color.map(wgpu::Color::from).map(wgpu::LoadOp::Clear).unwrap_or(wgpu::LoadOp::Load),
                    store: if info.discard { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store },
                },
            }
        })
        .map(Some)
        .collect();
    
    let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &color_attachments,
        depth_stencil_attachment: depth_stencil_attachment
            .map(|info| wgpu::RenderPassDepthStencilAttachment {
                view: resources.get_texture_view(renderer, info.texture_view_handle)
                    .expect("Invalid color attachment texture view handle given"),
                depth_ops: info.depth_ops,
                stencil_ops: info.stencil_ops,
            }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    RenderPassOperation2 {
        renderer,
        renderpass,
    }
}

impl<'a, 'b, S> RenderPassOperationBuilder<'a, 'b, S>
    where S: render_pass_operation_builder::State,
{
    pub fn color_attachment(self) -> AddRenderPassColorAttachment<'a, 'b, S> {
        start_add_color_attachment(self)
    }

    pub fn depth_stencil_attachment(self) -> AddRenderPassDepthStencilAttachment<'a, 'b, S> {
        start_add_depth_stencil_attachment(self)
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
    depth_buffer_resource_handle: RenderPassResourceHandle,
    render_target_resource_handle: RenderPassResourceHandle,
}

impl RenderPassResources {
    /// Works with a texture or texture view handle
    pub fn get_texture_view<'a>(&'a self, renderer: &'a Renderer, handle: RenderPassResourceHandle) -> Option<&'a wgpu::TextureView> {
        if let Some(present_surface) = &self.present_surface && present_surface.handle == handle {
            return Some(&present_surface.texture_view);
        }

        if handle == self.depth_buffer_resource_handle {
            return Some(&renderer.resources.textures
                .get(renderer.depth_buffer_handle)
                .expect("Depth buffer is present")
                .view);
        }

        if handle == self.render_target_resource_handle {
            return Some(&renderer.resources.textures
                .get(renderer.render_target_handle)
                .expect("Depth buffer is present")
                .view);
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

    /// called by the Drop impl
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

    pub fn using_depth_buffer(&mut self) -> RenderPassResourceHandle {
        self.data.as_mut().expect("Already submitted??").resources.depth_buffer_resource_handle
    }

    pub fn using_render_target(&mut self) -> RenderPassResourceHandle {
        self.data.as_mut().expect("Already submitted??").resources.render_target_resource_handle
    }
}

impl<'a> Drop for RenderOperation<'a> {
    fn drop(&mut self) {
        self.submit();
    }
}

