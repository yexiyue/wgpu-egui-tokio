use wgpu_egui_tokio::{
    Page, Render, WgpuState,
    wgpu::{self, include_wgsl},
};

pub struct Simple {
    pub pipeline: wgpu::RenderPipeline,
}

impl Page for Simple {
    type Message = ();
    fn new(
        WgpuState { device, config, .. }: &wgpu_egui_tokio::WgpuState,
        _sender: tokio::sync::mpsc::Sender<Self::Message>,
    ) -> Self
    where
        Self: Sized,
    {
        let shader = device.create_shader_module(include_wgsl!("trangle.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            multisample: Default::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });
        Self { pipeline }
    }
}

impl Render for Simple {
    fn render(
        &self,
        _state: &WgpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> anyhow::Result<()> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
