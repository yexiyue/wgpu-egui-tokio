use image_texture::{Message, Simple, StudyImageTexture};
use wgpu_egui_tokio::{App, winit};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = App::new();
    app.register::<StudyImageTexture, Message>();
    app.register::<Simple, ()>();
    event_loop.run_app(&mut app)?;
    Ok(())
}
