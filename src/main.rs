use est_render::prelude::*;

fn main() {
    let mut font = FontManager::new();
    let arial_font = font.load_font("Arial", None, 24.0)
        .expect("Failed to load Arial font");

    let (data, width, height) = arial_font.bake_text("Hello, World!\nHello", FontBakeFormat::Rgba)
        .expect("Failed to bake text");

    let mut runner = create_runner()
        .expect("Failed to create runner");

    let mut window = runner.create_window("Hello world", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let texture = gpu.create_texture()
        .with_raw(&data, Rect::new(0, 0, width, height), TextureFormat::Bgra8Unorm)
        .with_usage(TextureUsage::Sampler)
        .build()
        .expect("Failed to create texture");

    while runner.pool_events(None) {
        if let Some(mut cmd) = gpu.begin_command() {
            if let Some(mut rp) = cmd.begin_renderpass() {
                rp.set_clear_color(Color::BLACK);

                if let Some(mut drawing) = rp.begin_drawing() {
                    drawing.set_texture(Some(&texture));
                    drawing.rectangle_filled(Vector2::new(100.0, 100.0), Vector2::new(width, height), Color::WHITE);
                }
            }
        }
    }
}