extern crate est_render;

use est_render::prelude::*;

#[cfg(feature = "software")]
fn main() {
    let mut runner = est_render::create_runner().expect("Failed to create runner");
    let mut window = runner
        .create_window("Engine Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let mut sw = est_render::create_pixel_buffer(Some(&mut window))
        .build()
        .expect("Failed to create pixel buffer");

    let mut pixels = vec![128u32; (800 * 600) as usize];

    while runner.pool_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                Event::WindowResized { size, .. } => {
                    pixels.resize((size.x * size.y) as usize, 128);
                }
                _ => {}
            }
        }

        if let Err(e) = sw.write_buffers(&pixels, Vector2::new(800.0, 600.0)) {
            eprintln!("Error writing buffers: {}", e);
        }
    }
}

#[cfg(not(feature = "software"))]
fn main() {
    eprintln!(
        "Software rendering is not enabled. Please enable the 'software' feature to run this example."
    );
}
