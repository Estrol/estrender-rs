# Estrol Rendering Library
Easy to use winit, softbuffer & wgpu abstractions

## Features
* Abstraction over `winit's Window`, `softbuffer's Pixel Buffer` and `wgpu's GPU Device` creation with simple builder.
* Support `Swapchain`, `Render Target` and `GPU Compute`.
* Support passing raw byte to uniform buffer with managed GPU Buffer.
* Support for directly writing into Window pixel buffer.
* Support for multiple window and multiple software or gpu context or both.

## Example
```rs
use erlib::Engine;

pub fn main() -> Result<(), String> {
  let mut runner = Engine::make_runner();
  let window = Engine::make_window("Test Window", Size::new(800, 600), Position::new(0, 0))
    .build(&mut runner)
    .unwrap();

  let softcontext = window.create_softbuffer_context().unwrap();
  let pixels = vec![0x80808080u32; 800 * 600];

  loop {
    if !runner.pool_events() {
      break;
    }

    let result = softcontext.write_pixels(
      Vector2::Zero,
      Vector2::new(800.0, 600.0),
      &pixels,
      PixelWriteMode::Clear,
      None
    );

    if result.is_err() {
      println("Failed to write pixels: {:?}", result.err())
    }
  }
}
```


## License
Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT-0 license
   ([LICENSE-MIT-0](LICENSE-MIT-0) or http://opensource.org/licenses/MIT-0)

at your option.