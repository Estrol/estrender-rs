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

fn main() {
    let mut runner = create_runner().expect("Failed to create runner");
    let mut window = runner
        .create_window("Engine Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let gpu = create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut msaa_texture = gpu
        .create_texture()
        .with_render_target(Rect::with_size(800, 600), None)
        .with_sample_count(SampleCount::SampleCount4)
        .build()
        .expect("Failed to create MSAA texture");

    let blank_texture = gpu
        .create_texture()
        .with_raw(
            &[255u8; 4],
            Rect::with_size(1, 1),
            TextureFormat::Bgra8Unorm,
        )
        .with_usage(TextureUsage::Sampler)
        .build()
        .expect("Failed to create blank texture");

    let shader = gpu
        .create_graphics_shader()
        .set_vertex_code(VERTEX_DRAWING_SHADER)
        .set_fragment_code(FRAGMENT_DRAWING_SHADER)
        .build()
        .expect("Failed to create graphics shader");

    let pipeline = gpu
        .create_render_pipeline()
        .set_shader(Some(&shader))
        .set_blend(Some(&TextureBlend::ALPHA_BLEND))
        .set_attachment_texture(0, 0, Some(&blank_texture))
        .set_attachment_sampler(0, 1, Some(&TextureSampler::DEFAULT))
        .build()
        .expect("Failed to create render pipeline");

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
        .set_usage(BufferUsages::VERTEX)
        .build()
        .expect("Failed to create vertex buffer");

    let ibo = gpu
        .create_buffer()
        .set_data_vec(indexes)
        .set_usage(BufferUsages::INDEX)
        .build()
        .expect("Failed to create index buffer");

    let mut cursor_pos = Vector2::new(0.0, 0.0);
    while runner.pool_events(None) {
        for event in runner.get_events() {
            println!("{:?}", event);
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
                Event::Resized { window_id: _, size } => {
                    if size.x <= 0 || size.y <= 0 {
                        continue; // Skip invalid sizes
                    }

                    msaa_texture = gpu
                        .create_texture()
                        .with_render_target(Rect::new(0, 0, size.x as u32, size.y as u32), None)
                        .with_sample_count(SampleCount::SampleCount4)
                        .build()
                        .expect("Failed to resize MSAA texture");
                }
                Event::CursorMoved { window_id: _, pos } => {
                    cursor_pos = Vector2::new(pos.x as f32, pos.y as f32);
                }
                _ => {}
            }
        }

        if let Some(mut cmd) = gpu.begin_command() {
            if let Some(mut rp) = cmd.begin_renderpass() {
                rp.set_clear_color(Color::BLACK);
                rp.set_multi_sample_texture(Some(&msaa_texture));
                
                rp.set_pipeline(Some(&pipeline));
                rp.set_gpu_buffer(Some(&vbo), Some(&ibo));
                rp.draw_indexed(0..3, 0, 1);

                if let Some(mut g) = rp.begin_drawing() {
                    g.rectangle_filled(Vector2::new(20, 20), Vector2::new(100, 100), Color::WHITE);
                    g.circle_filled(cursor_pos, 10.0, 30, Color::RED);
                }
            }
        }
    }
}
