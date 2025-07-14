extern crate est_render;

use est_render::prelude::*;

fn main() {
    let mut runner = est_render::runner::new().expect("Failed to create runner");

    let _window = runner
        .create_window("Input Example", Point2::new(800, 600))
        .build()
        .expect("Failed to create window");

    let window2 = runner
        .create_window("Second Window", Point2::new(800, 600))
        .build()
        .expect("Failed to create second window");

    let input = runner.create_input(None);
    let input2 = runner.create_input(Some(&window2));

    while runner.pump_events(None) {
        for event in runner.get_events() {
            match event {
                Event::WindowClosed { .. } => {
                    return;
                }
                _ => {}
            }
        }

        if input.mouse_pressed_once("Left") {
            println!("Mouse position: {:?}", input.mouse_position());
        }

        if input2.mouse_pressed_once("Left") {
            println!("Mouse position in second window: {:?}", input2.mouse_position());
        }
    }
}
