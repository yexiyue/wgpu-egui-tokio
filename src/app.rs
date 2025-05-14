use crate::{
    Render,
    page::{Page, Pages},
    state::WgpuState,
};
use egui_wgpu::ScreenDescriptor;
use std::sync::{Arc, Mutex};
use wgpu::{CommandEncoderDescriptor, TextureViewDescriptor};
use winit::{application::ApplicationHandler, event::WindowEvent, window::WindowAttributes};

pub struct App {
    pub state: Arc<Mutex<Option<WgpuState>>>,
    pub pages: Pages,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
            pages: Pages::new(),
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T, M>(&mut self)
    where
        T: Page<Message = M> + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        self.pages.register::<T, M>();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let res = (|| {
            let window = event_loop.create_window(WindowAttributes::default())?;
            let window = Arc::new(window);
            let state = pollster::block_on(WgpuState::new(window))?;
            Ok::<WgpuState, anyhow::Error>(state)
        })();
        match res {
            Ok(state) => {
                self.state.lock().unwrap().replace(state);
                self.pages.create(self.state.clone());
            }
            Err(err) => {
                tracing::error!("Failed to create WgpuState: {}", err);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(mut state) = self.state.lock().unwrap().as_mut() {
            state.egui_renderer.handle_input(&state.window, &event);

            match event {
                // 关闭窗口请求
                WindowEvent::CloseRequested => {
                    event_loop.exit(); // 退出事件循环
                }

                // 重绘请求（驱动渲染循环）
                WindowEvent::RedrawRequested => {
                    // 执行窗口预呈现通知
                    state.window.pre_present_notify();

                    // 执行实际渲染操作
                    if let Err(r) = (|| {
                        ui_render(&mut state, &mut self.pages)?;
                        Ok::<(), anyhow::Error>(())
                    })() {
                        tracing::error!("Render error: {}", r);
                    }

                    // 请求下一帧重绘（维持持续渲染）
                    state.window.request_redraw();
                }

                // 窗口大小变化事件
                WindowEvent::Resized(size) => {
                    // 更新WGPU表面配置
                    state.resize(size);
                    tracing::info!("Window resized to {:?}", size);
                }

                // 其他未处理事件
                _ => {}
            }

            self.pages.handle_event(event, state);
        }
    }
}

fn ui_render(state: &mut WgpuState, ui: &mut dyn Render) -> anyhow::Result<()> {
    let surface_texture = state.surface.get_current_texture()?;
    let view = surface_texture
        .texture
        .create_view(&TextureViewDescriptor::default());
    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [state.config.width, state.config.height],
        pixels_per_point: state.window.scale_factor() as f32,
    };
    let mut encoder = state
        .device
        .create_command_encoder(&CommandEncoderDescriptor::default());

    ui.render(&state, &view, &mut encoder)?;

    {
        state.egui_renderer.begin_frame(&state.window);

        ui.ui_draw(state.egui_renderer.context());

        state.egui_renderer.end_frame_and_draw(
            &state.device,
            &state.queue,
            &mut encoder,
            &state.window,
            &view,
            screen_descriptor,
        );
    }

    // 7. 提交命令到队列
    let command_buffer = encoder.finish();
    state.queue.submit(std::iter::once(command_buffer));

    // 8. 呈现渲染结果
    surface_texture.present();
    Ok(())
}
