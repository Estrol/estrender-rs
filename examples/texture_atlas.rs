extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");

    let mut window = runner
        .create_window("Clear Color Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let texture_atlas = gpu
        .create_texture_atlas()
        .add_texture_file(
            "example_texture",
            "./examples/resources/test1.png",
        )
        .add_texture_file(
            "example_texture2",
            "./examples/resources/test2.png",
        )
        .build()
        .expect("Failed to create texture atlas");

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
                gp.set_clear_color(Color::BLUEVIOLET);
                gp.set_blend(0, Some(&TextureBlend::NONE));

                if let Some(mut drawing) = gp.begin_drawing() {
                    drawing.set_texture_atlas(Some((&texture_atlas, "example_texture")));
                    drawing.draw_rect_image(
                        Vector2::new(100.0, 100.0),
                        Vector2::new(200.0, 200.0),
                        Color::WHITE,
                    );
                    drawing.set_texture_atlas(Some((&texture_atlas, "example_texture2")));
                    drawing.draw_rect_image(
                        Vector2::new(350.0, 100.0),
                        Vector2::new(200.0, 200.0),
                        Color::WHITE,
                    );
                    drawing.draw_circle_image(Vector2::new(600.0, 200.0), 100.0, 20, Color::WHITE);
                }
            }
        }
    }
}
