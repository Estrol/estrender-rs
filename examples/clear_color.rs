extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");

    let mut window = runner
        .create_window("Clear Color Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let gpu = est_render::create_gpu(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    while runner.pool_events(None) {
        if let Some(mut cmd) = gpu.begin_command() {
            if let Some(mut gp) = cmd.begin_renderpass() {
                gp.set_clear_color(Color::BLUE); // Set the clear color to blue
            }
        }
    }
}
