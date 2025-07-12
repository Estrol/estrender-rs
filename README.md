# Estrol Rendering Library
Easy to use winit, softbuffer & wgpu abstractions

## Status
The crate still on heavy development, changes between namespaces are expected!

## Features
* Abstraction over `winit's Window`, `softbuffer's Pixel Buffer` and `wgpu's GPU Device` creation with simple builder.
* Support `Swapchain`, `Render Target` and `GPU Compute`.
* Support passing raw byte to uniform buffer with managed GPU Buffer.
* Support for directly writing into Window pixel buffer.
* Support for multiple window and multiple software or gpu context or both.

## Supported Platforms

| Platform | Status        |
|----------|--------------|
| Windows  | Supported    |
| Linux    | Supported    |
| macOS*    | Untested     |
| Android*  | Untested     |
| WASM**     | Unsupported  |
| iOS***      | Unsupported  |

\*It might be useable, but not gurranted. \
\**WASM is not supported due how the library designed. \
\***iOS is always unsupported because I don't have macOS and iPhone.

## List crate's features
- `software` - Use softbuffer to display content to window instead GPU.
- `x11` - Use X11 platform instead wayland on linux
- `font` - Font rasterization support using fontdue and ttf_parser

## Example
Examples are available at folder `examples`.

## License
Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT-0 license
   ([LICENSE-MIT-0](LICENSE-MIT-0) or http://opensource.org/licenses/MIT-0)

at your option.