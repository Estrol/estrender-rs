extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");

    let mut window = runner
        .create_window("Drawing Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::gpu::new(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut rotation = 0.0f32;
    while runner.pump_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        rotation += runner.get_frame_time() as f32 * 60.0f32; // Rotate at 60 degrees per second
        if rotation >= 360.0 {
            rotation -= 360.0; // Reset rotation after a full circle
        }

        if let Ok(mut cmd) = gpu.begin_command() {
            if let Ok(mut gp) = cmd.begin_renderpass() {
                gp.set_clear_color(Color::BLUE); // Set the clear color to blue

                if let Some(mut drawing) = gp.begin_drawing() {
                    drawing.set_rotation(rotation); // Set rotation to 45 degrees
                    gp.set_blend(0, Some(&BlendState::ALPHA_BLEND));
                    drawing.draw_rect_filled(
                        Vector2::new(100.0, 100.0),
                        Vector2::new(200.0, 200.0),
                        Color::RED,
                    );

                    drawing.draw_circle_filled(Vector2::new(400.0, 300.0), 50.0, 25, Color::GREEN);

                    gp.set_blend(0, Some(&BlendState::ADDITIVE_BLEND));

                    drawing.draw_text(
                        "Hello, World!",
                        Vector2::new(300.0, 500.0),
                        Color::WHITE,
                    );
                }
            }
        }
    }
}
