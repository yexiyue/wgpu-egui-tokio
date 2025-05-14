mod app;
mod egui_utils;
mod page;
mod state;
pub use app::App;
pub use egui;
pub use page::Page;
pub use state::WgpuState;
pub use wgpu;
use wgpu::{CommandEncoder, TextureView};
pub use winit;

pub trait Render {
    fn ui_draw(&mut self, ctx: &egui::Context) {
        let _ = ctx;
    }

    fn handle_event(&mut self, event: winit::event::WindowEvent, state: &WgpuState) {
        let _ = event;
        let _ = state;
    }

    fn render(
        &self,
        state: &WgpuState,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) -> anyhow::Result<()> {
        let _ = state;
        let _ = view;
        let _ = encoder;
        Ok(())
    }
}
