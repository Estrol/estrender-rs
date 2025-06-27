extern crate est_render;

use est_render::prelude::*;

pub(crate) const IMGUI_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn main_vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    return output;
}

@location(0) var myTexture: texture_2d<f32>;
@location(1) var mySampler: sampler;

@fragment
fn main_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(myTexture, mySampler, input.uv);
    return color * input.color; // Apply vertex color modulation
}
"#;

pub struct ImGuiState {
    pub context: imgui::Context,
    pub buffer_vertex: Option<Buffer>,
    pub buffer_index: Option<Buffer>,
    pub shader: GraphicsShader,
    pub textures: Vec<Texture>,
}

impl ImGuiState {
    pub fn new(gpu: &mut GPU) -> Self {
        let mut context = imgui::Context::create();
        context.set_ini_filename(None); // Disable ini file

        let shader = gpu
            .create_graphics_shader()
            .set_source(IMGUI_SHADER)
            .build()
            .expect("Failed to create ImGui shader");

        Self {
            context,
            buffer_vertex: None,
            buffer_index: None,
            shader,
            textures: Vec::new(),
        }
    }

    pub fn prepare_font(&mut self, gpu: &mut GPU) {
        let font_data = self.context.fonts().build_rgba32_texture();
        let texture = gpu
            .create_texture()
            .with_raw(
                &font_data.data,
                Rect::new(0, 0, font_data.width, font_data.height),
                TextureFormat::Bgra8Unorm,
            )
            .with_usage(TextureUsage::Sampler)
            .build()
            .expect("Failed to create ImGui font texture");
    }

    pub fn draw(&mut self, command_buffer: &mut CommandBuffer, render_pass: &mut RenderPass) {}
}

fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");

    let mut window = runner
        .create_window("Clear Color Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut imgui_state = ImGuiState::new(&mut gpu);
    let mut show_demo_window = true;

    while runner.pool_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        let ui = imgui_state.context.frame();

        ui.window("Hello, world!")
            .size([300.0, 100.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("This is some useful text.");

                if ui.button("Button") {
                    println!("Button pressed!");
                }
            });

        ui.show_about_window(&mut show_demo_window);

        ui.end_frame_early();

        let draw_list = imgui_state.context.render();

        if let Some(mut cmd) = gpu.begin_command() {
            if let Some(mut gp) = cmd.begin_renderpass() {
                gp.set_clear_color(Color::BLUE); // Set the clear color to blue
            }
        }
    }
}
