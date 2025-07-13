extern crate est_render;

use est_render::prelude::*;

pub(crate) const VERTEX_DRAWING_SHADER: &str = r#"
// Vertex Shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) texCoord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) texCoord: vec2<f32>,
};

@vertex
fn main_vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 1.0);
    output.color = input.color;
    output.texCoord = input.texCoord;
    return output;
}"#;

pub(crate) const FRAGMENT_DRAWING_SHADER: &str = r#"
// Fragment Shader
@group(0) @binding(0) var myTexture: texture_2d<f32>;
@group(0) @binding(1) var mySampler: sampler;

struct FragmentInput {
    @location(0) color: vec4<f32>,
    @location(1) texCoord: vec2<f32>,
};

@fragment
fn main_fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    let checkerSize = 50.0;
    let x = floor(input.texCoord.x * checkerSize);
    let y = floor(input.texCoord.y * checkerSize);
    let isWhite = ((x + y) % 2.0) == 0.0;

    if isWhite {
        return input.color; // white square
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0); // black square
    }
}"#;

pub(crate) const COMPUTE_NOOP_SHADER: &str = r#"
// Compute Shader
@compute @workgroup_size(1)
fn main() {
    // This compute shader does nothing
    // It can be used to test the compute pipeline setup
}
"#;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");
    let mut window = runner
        .create_window("Engine Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::gpu::new(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut msaa_texture = gpu
        .create_texture()
        .set_render_target(Point2::new(800, 600), None)
        .set_sample_count(SampleCount::SampleCount4)
        .build()
        .expect("Failed to create MSAA texture");

    let blank_texture = gpu
        .create_texture()
        .set_raw_image(
            &[255u8; 4],
            Point2::new(1, 1),
            TextureFormat::Bgra8Unorm,
        )
        .set_usage(TextureUsage::Sampler)
        .build()
        .expect("Failed to create blank texture");

    let shader = gpu
        .create_graphics_shader()
        .set_vertex_code(VERTEX_DRAWING_SHADER)
        .set_fragment_code(FRAGMENT_DRAWING_SHADER)
        .build()
        .expect("Failed to create graphics shader");

    let compute_shader = gpu
        .create_compute_shader()
        .set_source(COMPUTE_NOOP_SHADER)
        .build()
        .expect("Failed to create compute shader");

    let pipeline = gpu
        .create_render_pipeline()
        .set_shader(Some(&shader))
        .set_blend(Some(&BlendState::ALPHA_BLEND))
        .set_attachment_texture(0, 0, Some(&blank_texture))
        .set_attachment_sampler(0, 1, Some(&TextureSampler::DEFAULT))
        .build()
        .expect("Failed to create render pipeline");

    let compute_pipeline = gpu
        .create_compute_pipeline()
        .set_shader(Some(&compute_shader))
        .build()
        .expect("Failed to create compute pipeline");

    // Triangle vertices
    let vertices = vec![
        Vertex {
            position: Vector3::new(-0.5, -0.5, 0.0),
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            texcoord: Vector2::new(0.0, 1.0),
        },
        Vertex {
            position: Vector3::new(0.5, -0.5, 0.0),
            color: Color::new(0.0, 1.0, 0.0, 1.0),
            texcoord: Vector2::new(1.0, 1.0),
        },
        Vertex {
            position: Vector3::new(0.0, 0.5, 0.0),
            color: Color::new(0.0, 0.0, 1.0, 1.0),
            texcoord: Vector2::new(0.5, 0.0),
        },
    ];

    let indexes = vec![0u16, 1u16, 2u16];

    let vbo = gpu
        .create_buffer()
        .set_data_vec(vertices)
        .set_usage(BufferUsage::VERTEX)
        .build()
        .expect("Failed to create vertex buffer");

    let ibo = gpu
        .create_buffer()
        .set_data_vec(indexes)
        .set_usage(BufferUsage::INDEX)
        .build()
        .expect("Failed to create index buffer");

    while runner.pump_events(PumpMode::WaitDraw) {
        for event in runner.get_events() {
            match event {
                Event::KeyboardInput {
                    window_id,
                    key,
                    pressed,
                } => {
                    if *window_id == window.id() && key == "Escape" && *pressed {
                        window.quit();
                    }
                }
                Event::WindowResized { window_id: _, size } => {
                    if size.x <= 0 || size.y <= 0 {
                        continue; // Skip invalid sizes
                    }

                    msaa_texture = gpu
                        .create_texture()
                        .set_render_target(Point2::new(size.x as u32, size.y as u32), None)
                        .set_sample_count(SampleCount::SampleCount4)
                        .build()
                        .expect("Failed to resize MSAA texture");
                }
                Event::RedrawRequested { window_id: _ } => {
                    if let Ok(mut cmd) = gpu.begin_command() {
                        if let Ok(mut cm) = cmd.begin_computepass() {
                            cm.set_pipeline(Some(&compute_pipeline));
                            cm.dispatch(1, 1, 1);
                        }

                        if let Ok(mut rp) = cmd.begin_renderpass() {
                            rp.set_clear_color(Color::BLACK);
                            rp.push_msaa_texture(&msaa_texture);

                            rp.set_pipeline(Some(&pipeline));
                            rp.set_gpu_buffer(Some(&vbo), Some(&ibo));
                            rp.draw_indexed(0..3, 0, 1);
                        }
                    }

                    window.request_redraw();
                }
                _ => {}
            }
        }
    }
}
