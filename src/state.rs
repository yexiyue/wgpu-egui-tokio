use anyhow::anyhow;
use std::sync::Arc;
use wgpu::{
    Device, DeviceDescriptor, Instance, InstanceDescriptor, Queue, RequestAdapterOptionsBase,
    Surface, SurfaceConfiguration,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::egui_utils::EguiRenderer;

pub struct WgpuState {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
    pub egui_renderer: EguiRenderer,
}

impl WgpuState {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let instance = Instance::new(&InstanceDescriptor::from_env_or_default());
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptionsBase {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or(anyhow!("Failed to find an appropriate adapter"))?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default(), None)
            .await?;
        let PhysicalSize { width, height } = window.inner_size();

        let config = surface
            .get_default_config(&adapter, width.max(1), height.max(1))
            .ok_or(anyhow!("Failed to find a surface configuration"))?;

        surface.configure(&device, &config);

        let egui_renderer = EguiRenderer::new(&device, config.format, None, 1, &window);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            egui_renderer,
        })
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        // 重新配置表面（更新尺寸）
        self.surface.configure(&self.device, &self.config);
    }
}
