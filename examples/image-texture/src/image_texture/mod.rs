use image::GenericImageView;
use wgpu_egui_tokio::{
    Page, Render, WgpuState, egui,
    wgpu::{self, Color, CommandEncoder, TextureView, include_wgsl, util::DeviceExt},
    winit::event::WindowEvent,
};

// 计算图像在屏幕上的缩放比例，返回一个包含宽度和高度缩放比例的数组
pub fn calc_scale(image: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    // 解构图像和屏幕的宽度和高度
    let [width, height] = image;
    let [screen_width, screen_height] = screen;

    // 计算图像和屏幕的宽高比
    let image_ratio = width / height;
    let screen_ratio = screen_width / screen_height;

    // 根据宽高比调整缩放比例
    if image_ratio > screen_ratio {
        [1.0, screen_ratio / image_ratio] // 图像宽度占满屏幕，高度按比例缩放
    } else {
        [image_ratio / screen_ratio, 1.0] // 图像高度占满屏幕，宽度按比例缩放
    }
}

fn gen_texture_data() -> Vec<u8> {
    let red = [255u8, 0, 0, 255]; // 红色
    let yellow = [255, 255, 0, 255]; // 黄色
    let blue = [0, 0, 255, 255]; // 蓝色

    // 定义二维纹理数据结构并翻转
    let rows = [
        [red, red, red, red, red],          // 第七行
        [red, yellow, red, red, red],       // 第六行
        [red, yellow, red, red, red],       // 第五行
        [red, yellow, yellow, red, red],    // 第四行
        [red, yellow, red, red, red],       // 第三行
        [red, yellow, yellow, yellow, red], // 第二行
        [blue, red, red, red, red],         // 第一行
    ];

    // 将二维数组展平为一维字节数组
    rows.iter().flatten().flatten().copied().collect()
}

pub enum Message {
    Sampler,
    Load,
    LoadedImage(image::DynamicImage),
}

pub struct StudyImageTexture {
    pub mag_filter: wgpu::FilterMode,      // 纹理放大过滤模式
    pub address_mode_u: wgpu::AddressMode, // 纹理 U 轴寻址模式
    pub address_mode_v: wgpu::AddressMode, // 纹理 V 轴寻址模式
    pub loading: bool,                     // 是否正在加载图像
    pub image_url: String,                 // 图像 URL
    sender: tokio::sync::mpsc::Sender<Message>,
    pub pipeline: wgpu::RenderPipeline, // 渲染管线（包含着色器、状态配置等）
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub scale_bind_group: wgpu::BindGroup,
    pub scale_buffer: wgpu::Buffer,
    pub image_dimensions: [f32; 2],
}

impl Page for StudyImageTexture {
    type Message = Message;
    fn new(
        state: &wgpu_egui_tokio::WgpuState,
        sender: tokio::sync::mpsc::Sender<Self::Message>,
    ) -> Self
    where
        Self: Sized,
    {
        let WgpuState {
            device,
            queue,
            config,
            ..
        } = state;
        // 6. 创建着色器模块（加载WGSL着色器）
        let shader = device.create_shader_module(include_wgsl!("texture.wgsl"));

        // 7. 创建渲染管线

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: None, // 使用默认管线布局
            vertex: wgpu::VertexState {
                module: &shader,         // 顶点着色器模块
                entry_point: Some("vs"), // 入口函数
                buffers: &[],            // 顶点缓冲区布局（本示例为空）
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,         // 片元着色器模块
                entry_point: Some("fs"), // 入口函数
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,                  // 使用表面配置的格式
                    blend: Some(wgpu::BlendState::REPLACE), // 混合模式：直接替换
                    write_mask: wgpu::ColorWrites::ALL,     // 允许写入所有颜色通道
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), // 使用默认图元配置（三角形列表）
            depth_stencil: None,           // 禁用深度/模板测试
            multisample: Default::default(), // 多重采样配置
            multiview: None,
            cache: None,
        });

        let texture_data = gen_texture_data();

        let image_dimensions = [5.0, 7.0]; // 图像尺寸
        let texture_size = wgpu::Extent3d {
            width: 5,
            height: 7,
            ..Default::default()
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("texture"),
            size: texture_size,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_size.width * 4),
                rows_per_image: None,
            },
            texture_size,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
            ],
        });

        let scale = calc_scale(
            image_dimensions,
            [config.width as f32, config.height as f32],
        );
        let scale_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scale Buffer"),
            contents: bytemuck::cast_slice(&scale),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scale_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scale_buffer.as_entire_binding(),
            }],
        });

        Self {
            mag_filter: wgpu::FilterMode::Nearest, // 默认使用最近点采样
            address_mode_u: wgpu::AddressMode::ClampToEdge, // 默认 U 轴边缘拉伸
            address_mode_v: wgpu::AddressMode::ClampToEdge, // 默认 V 轴边缘拉伸
            image_url: String::new(),              // 默认空字符串
            sender,
            pipeline,
            bind_group,
            texture,
            scale_bind_group,
            scale_buffer,
            image_dimensions, // 默认图像尺寸
            loading: false,
        }
    }

    fn update(
        &mut self,
        message: Self::Message,
        WgpuState {
            device,
            queue,
            config,
            ..
        }: &wgpu_egui_tokio::WgpuState,
    ) {
        match message {
            Message::Sampler => {
                // 根据控件设置创建采样器
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: self.address_mode_u,
                    address_mode_v: self.address_mode_v,
                    mag_filter: self.mag_filter,
                    ..Default::default()
                });

                // 创建绑定组并更新
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &self.texture.create_view(&Default::default()),
                            ),
                        },
                    ],
                });
                self.bind_group = bind_group;
            }
            Message::Load => {
                self.loading = true;
                tokio::spawn({
                    let url = self.image_url.clone();
                    let sender = self.sender.clone();
                    async move {
                        let bytes = reqwest::get(url).await.unwrap().bytes().await.unwrap();
                        let image = image::load_from_memory(&bytes).unwrap();
                        sender.send(Message::LoadedImage(image)).await.unwrap();
                    }
                });
            }
            Message::LoadedImage(image) => {
                self.loading = false;
                let (width, height) = image.dimensions();
                self.image_dimensions = [width as f32, height as f32];

                // 计算缩放比例并更新缓冲区
                let scale = calc_scale(
                    self.image_dimensions,
                    [config.width as f32, config.height as f32],
                );
                queue.write_buffer(&self.scale_buffer, 0, bytemuck::cast_slice(&scale));

                // 创建新的纹理并加载图像数据
                let texture_size = wgpu::Extent3d {
                    width,
                    height,
                    ..Default::default()
                };
                let texture_data = image.flipv().to_rgba8().to_vec();

                self.texture = device.create_texture_with_data(
                    &queue,
                    &wgpu::TextureDescriptor {
                        label: Some("net_texture"),
                        size: texture_size,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::COPY_DST
                            | wgpu::TextureUsages::RENDER_ATTACHMENT,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::MipMajor,
                    &texture_data,
                );

                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    address_mode_u: self.address_mode_u,
                    address_mode_v: self.address_mode_v,
                    mag_filter: self.mag_filter,
                    ..Default::default()
                });

                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(
                                &self.texture.create_view(&Default::default()),
                            ),
                        },
                    ],
                });
                self.bind_group = bind_group;
            }
        }
    }
}

impl Render for StudyImageTexture {
    fn ui_draw(&mut self, ctx: &wgpu_egui_tokio::egui::Context) {
        egui::Window::new("Controls").show(ctx, |ui| {
            // 渲染 Mag Filter 下拉框
            ui.horizontal(|ui| {
                ui.label("Mag Filter");
                egui::ComboBox::from_id_salt("mag_filter")
                    .selected_text(format!("{:?}", self.mag_filter))
                    .show_ui(ui, |ui| {
                        let a = ui
                            .selectable_value(
                                &mut self.mag_filter,
                                wgpu::FilterMode::Nearest,
                                "Nearest",
                            )
                            .changed();
                        let b = ui
                            .selectable_value(
                                &mut self.mag_filter,
                                wgpu::FilterMode::Linear,
                                "Linear",
                            )
                            .changed();
                        a || b
                    })
                    .inner
                    .map(|changed| {
                        if changed {
                            self.sender.try_send(Message::Sampler).unwrap(); // 如果值改变，发送消息
                        }
                    })
            });
            ui.add_space(16.0); // 添加间距

            // 渲染 Address Mode U 下拉框
            ui.horizontal(|ui| {
                ui.label("Address Mode U");
                egui::ComboBox::from_id_salt("address_mode_u")
                    .selected_text(format!("{:?}", self.address_mode_u))
                    .show_ui(ui, |ui| {
                        let a = ui
                            .selectable_value(
                                &mut self.address_mode_u,
                                wgpu::AddressMode::ClampToEdge,
                                "ClampToEdge",
                            )
                            .changed();
                        let b = ui
                            .selectable_value(
                                &mut self.address_mode_u,
                                wgpu::AddressMode::Repeat,
                                "Repeat",
                            )
                            .changed();
                        a || b
                    })
                    .inner
                    .map(|changed| {
                        if changed {
                            self.sender.try_send(Message::Sampler).unwrap(); // 如果值改变，发送消息
                        }
                    })
            });
            ui.add_space(16.0); // 添加间距

            // 渲染 Address Mode V 下拉框
            ui.horizontal(|ui| {
                ui.label("Address Mode V");

                egui::ComboBox::from_id_salt("address_mode_v")
                    .selected_text(format!("{:?}", self.address_mode_v))
                    .show_ui(ui, |ui| {
                        let a = ui
                            .selectable_value(
                                &mut self.address_mode_v,
                                wgpu::AddressMode::ClampToEdge,
                                "ClampToEdge",
                            )
                            .changed();
                        let b = ui
                            .selectable_value(
                                &mut self.address_mode_v,
                                wgpu::AddressMode::Repeat,
                                "Repeat",
                            )
                            .changed();
                        a || b
                    })
                    .inner
                    .map(|changed| {
                        if changed {
                            self.sender.try_send(Message::Sampler).unwrap(); // 如果值改变，发送消息
                        }
                    })
            });
            ui.add_space(16.0); // 添加间距

            // 渲染 Image URL 文本框和加载按钮
            ui.vertical(|ui| {
                ui.label("Image URL");
                ui.add_space(8.0); // 添加间距
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.image_url); // 文本框输入 URL
                    if ui.button("Load").clicked() {
                        self.sender.try_send(Message::Load).unwrap(); // 如果值改变，发送消息
                    }
                });
            });

            if self.loading {
                ui.horizontal(|ui| {
                    ui.label("Loading Image...");
                    ui.spinner();
                });
            }
        });
    }

    fn render(
        &self,
        _state: &WgpuState,
        view: &TextureView,
        encoder: &mut CommandEncoder,
    ) -> anyhow::Result<()> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color::BLACK), // 用黑色清除背景
                    store: wgpu::StoreOp::Store,             // 存储渲染结果
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // 5. 设置渲染管线
        pass.set_pipeline(&self.pipeline);

        // 6. 设置绑定组
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.scale_bind_group, &[]);
        // 7. 使用实例化绘制
        pass.draw(0..6, 0..1);

        Ok(())
    }

    fn handle_event(
        &mut self,
        event: wgpu_egui_tokio::winit::event::WindowEvent,
        state: &WgpuState,
    ) {
        match event {
            WindowEvent::Resized(_) => {
                let scale = calc_scale(
                    self.image_dimensions,
                    [state.config.width as f32, state.config.height as f32],
                );
                state
                    .queue
                    .write_buffer(&self.scale_buffer, 0, bytemuck::cast_slice(&scale));
            }
            _ => {}
        }
    }
}
