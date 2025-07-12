extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");

    let mut window = runner
        .create_window("Font Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::gpu::new(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut font_manager = est_render::font::new();

    let font = font_manager
        .load_font("Arial", None, 20.0)
        .expect("Failed to load font");

    // Generate baked text texture
    let texture = font
        .create_baked_text(&mut gpu, "Hello, World!\nThis is a clear color example.")
        .expect("Failed to create baked text");

    while runner.pool_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        if let Ok(mut cmd) = gpu.begin_command() {
            if let Ok(mut gp) = cmd.begin_renderpass() {
                gp.set_clear_color(Color::BLUE);

                // The best texture blend for font rendering, others may has artifacts like black borders
                gp.set_blend(0, Some(&BlendState::ADDITIVE_BLEND));
                
                if let Some(mut drawing) = gp.begin_drawing() {
                    let size: Vector2 = texture.size().into();

                    // Baked text rendering
                    drawing.set_texture(Some(&texture));
                    drawing.draw_rect_image(Vector2::new(0.0, 0.0), size, Color::WHITE);

                    // Online text rendering
                    drawing.set_font(&font);
                    drawing.draw_text(
                        "Hello, World!\nThis is a clear color example.",
                        Vector2::new(size.x, 0.0),
                        Color::WHITE,
                    );
                }
            }
        }
    }
}
