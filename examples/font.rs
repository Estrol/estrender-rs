extern crate est_render;

use est_render::prelude::*;

#[cfg(feature = "font")]
fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");

    let mut window = runner
        .create_window("Clear Color Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut font_manager = est_render::create_font_manager();

    let font = font_manager
        .load_font("Arial", None, 20.0)
        .expect("Failed to load font");

    let (data, width, height) = font
        .bake_text("Hello, world!", FontBakeFormat::Rgba)
        .expect("Failed to bake text");

    let texture = gpu
        .create_texture()
        .with_raw(
            &data,
            Rect::new(0, 0, width, height),
            TextureFormat::Bgra8Unorm,
        )
        .with_usage(TextureUsage::Sampler)
        .build()
        .expect("Failed to create texture");

    while runner.pool_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        if let Some(mut cmd) = gpu.begin_command() {
            if let Some(mut gp) = cmd.begin_renderpass() {
                gp.set_clear_color(Color::BLUE); // Set the clear color to blue

                if let Some(mut drawing) = gp.begin_drawing() {
                    drawing.set_texture(Some(&texture));
                    drawing.rectangle_filled(
                        Vector2::new(100.0, 100.0),
                        Vector2::new(200.0, 200.0),
                        Color::RED,
                    );

                    drawing.set_texture(None);
                    drawing.circle_filled(Vector2::new(400.0, 300.0), 50.0, 25, Color::GREEN);
                }
            }
        }
    }
}

#[cfg(not(feature = "font"))]
fn main() {
    println!("Font feature is not enabled. Please enable the 'font' feature to run this example.");
}
