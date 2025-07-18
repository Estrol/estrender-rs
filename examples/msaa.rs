extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");
    let mut window = runner
        .create_window("Engine Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::gpu::new(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    let mut msaa_texture = Some(
        gpu.create_texture()
            .set_render_target(Point2::new(800, 600), None)
            .set_usage(TextureUsage::Sampler)
            .set_sample_count(SampleCount::SampleCount4)
            .build()
            .expect("Failed to create MSAA texture"),
    );

    let mut msaa_count = SampleCount::SampleCount4;
    let mut window_size = Point2::new(800, 600);

    while runner.pump_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                Event::KeyboardInput { key, pressed, .. } => {
                    if !*pressed {
                        continue;
                    }

                    let mut need_recreate = false;
                    if *key == "1" {
                        msaa_count = SampleCount::SampleCount1;
                        need_recreate = true;
                    }

                    if *key == "2" {
                        msaa_count = SampleCount::SampleCount2;
                        need_recreate = true;
                    }

                    if *key == "3" {
                        msaa_count = SampleCount::SampleCount4;
                        need_recreate = true;
                    }

                    if *key == "4" {
                        msaa_count = SampleCount::SampleCount8;
                        need_recreate = true;
                    }

                    if need_recreate {
                        if msaa_count == SampleCount::SampleCount1 {
                            msaa_texture = None;
                        } else {
                            msaa_texture = Some(
                                gpu.create_texture()
                                    .set_render_target(
                                        Point2::new(window_size.x, window_size.y),
                                        None,
                                    )
                                    .set_usage(TextureUsage::Sampler)
                                    .set_sample_count(msaa_count)
                                    .build()
                                    .expect("Failed to recreate MSAA texture"),
                            );
                        }
                    }
                }
                Event::WindowResized { size, .. } => {
                    if size.x <= 0 || size.y <= 0 {
                        eprintln!("Invalid window size: {:?}", size);
                        continue;
                    }

                    window_size = *size;

                    // Resize the MSAA texture to match the new window size
                    msaa_texture = Some(
                        gpu.create_texture()
                            .set_render_target(Point2::new(size.x, size.y), None)
                            .set_usage(TextureUsage::Sampler)
                            .set_sample_count(msaa_count)
                            .build()
                            .expect("Failed to resize MSAA texture"),
                    );
                }
                _ => {}
            }
        }

        if let Ok(mut cmd) = gpu.begin_command() {
            if let Ok(mut rp) = cmd.begin_renderpass() {
                rp.set_clear_color(Color::BLACK);
                if let Some(texture) = msaa_texture.as_ref() {
                    rp.push_msaa_texture(texture);
                }

                if let Some(mut drawing) = rp.begin_drawing() {
                    let pos1 = Vector2::new(0.0, 0.0);
                    let pos2 = Vector2::new(800.0, 0.0);
                    let pos3 = Vector2::new(400.0, 600.0);

                    // Draw a full triangle covering the window
                    drawing.draw_triangle_filled(pos1, pos2, pos3, Color::BLUE);
                }
            }
        }
    }
}
