use crate::{Render, WgpuState};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::mpsc::{Sender, channel};

pub trait Page: Render {
    type Message;

    fn new(state: &WgpuState, sender: Sender<Self::Message>) -> Self
    where
        Self: Sized;

    fn update(&mut self, message: Self::Message, state: &WgpuState) {
        let _ = message;
        let _ = state;
    }
}

fn create_component<T, M>(state: Arc<Mutex<Option<WgpuState>>>) -> Arc<RwLock<dyn Render>>
where
    T: Page<Message = M> + Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    let (sender, mut receiver) = channel(1);
    let state_ref = state.clone();
    let state_ref = state_ref.lock().unwrap();
    let state_ref = state_ref.as_ref().expect("WgpuState is not initialized");

    let component = Arc::new(RwLock::new(T::new(state_ref, sender)));

    tokio::spawn({
        let component = component.clone();
        async move {
            while let Some(message) = receiver.recv().await {
                let state_ref = state.clone();
                let state_ref = state_ref.lock().unwrap();
                let state_ref = state_ref.as_ref().expect("WgpuState is not initialized");
                component.write().unwrap().update(message, state_ref);
            }
        }
    });

    component
}

pub struct Pages {
    pub current: String,
    pub pages: HashMap<String, Arc<RwLock<dyn Render>>>,
    pub registers: HashMap<
        String,
        Box<dyn Fn(Arc<Mutex<Option<WgpuState>>>) -> Arc<RwLock<dyn Render>> + Send + Sync>,
    >,
}

impl Pages {
    pub(crate) fn new() -> Self {
        Self {
            current: "".to_string(),
            pages: HashMap::new(),
            registers: HashMap::new(),
        }
    }

    pub fn register<T, M>(&mut self)
    where
        T: Page<Message = M> + Send + Sync + 'static,
        M: Send + Sync + 'static,
    {
        let name = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap()
            .to_string();
        // 默认为最后一个注册的页面
        self.current = name.clone();

        let res = Box::new(|state| create_component::<T, M>(state));
        self.registers.insert(name, res);
    }

    pub(crate) fn create(&mut self, state: Arc<Mutex<Option<WgpuState>>>) {
        for (name, register) in self.registers.iter() {
            let component = register(state.clone());
            self.pages.insert(name.clone(), component);
        }
    }
}

impl Render for Pages {
    fn ui_draw(&mut self, ctx: &egui::Context) {
        egui::Window::new("Select Page").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Page");
                egui::ComboBox::from_id_salt("page")
                    .selected_text(&self.current)
                    .show_ui(ui, |ui| {
                        for (name, _) in self.pages.iter() {
                            ui.selectable_value(&mut self.current, name.clone(), name);
                        }
                    })
            })
        });

        if let Some(page) = self.pages.get_mut(&self.current) {
            page.write().unwrap().ui_draw(ctx);
        }
    }

    fn render(
        &self,
        state: &WgpuState,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> anyhow::Result<()> {
        if let Some(page) = self.pages.get(&self.current) {
            page.read().unwrap().render(state, view, encoder)?;
        }
        Ok(())
    }

    fn handle_event(&mut self, event: winit::event::WindowEvent, state: &WgpuState) {
        if let Some(page) = self.pages.get_mut(&self.current) {
            page.write().unwrap().handle_event(event, state);
        }
    }
}
