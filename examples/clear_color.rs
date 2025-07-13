extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");

    let mut window = runner
        .create_window("Clear Color Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut gpu = est_render::gpu::new(Some(&mut window))
        .build()
        .expect("Failed to create GPU");

    while runner.pump_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        if let Ok(mut cmd) = gpu.begin_command() {
            let surface = cmd.get_surface_texture();
            if surface.is_err() {
                println!("Failed to get surface texture: {:?}", surface.err());
                continue;
            }

            // Or you could use `cmd.begin_renderpass()` directly
            if let Ok(mut rp) = cmd.renderpass_builder()
                .add_surface_color_attachment(surface.as_ref().unwrap(), Some(&BlendState::ALPHA_BLEND))
                .build() 
            {
                rp.set_clear_color(Color::BLUE);
            }
        }
    }
}
