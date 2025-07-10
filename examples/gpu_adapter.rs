extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");
    let mut window = runner
        .create_window("Engine Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let adapters = est_render::query_gpu_adapter(Some(&window));
    if adapters.is_empty() {
        eprintln!("No GPU adapters found. Exiting.");
        return;
    }

    let selected_adapter = adapters
        .iter()
        .find(|adapter| adapter.backend_enum == AdapterBackend::Vulkan)
        .cloned();

    if selected_adapter.is_none() {
        eprintln!("No suitable GPU adapter found. Exiting.");
        return;
    }

    let adapter = selected_adapter.unwrap();
    let mut gpu = create_gpu(Some(&mut window))
        .set_adapter(&adapter)
        .build()
        .expect("Failed to create GPU");

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
            if let Ok(mut rp) = cmd.begin_renderpass() {
                rp.set_clear_color(Color::LIGHTBLUE);
            }
        }
    }
}
