use winit::{event, event_loop::EventLoopProxy};

#[cfg(feature = "software")]
use crate::software::software_inner::PixelBufferInner;
use crate::{
    gpu::gpu_inner::GPUInner,
    math::Point2,
    runner::{WindowEvent, runner_inner::Handle},
    utils::{ArcMut, ArcRef},
};

pub(crate) struct WindowInner {
    pub window_id: usize,
    pub window_events: ArcRef<Vec<event::WindowEvent>>,
    pub window_pointer: Option<ArcMut<Handle>>,
    pub proxy: EventLoopProxy<WindowEvent>,
    pub size: Point2,

    pub(crate) graphics: Option<ArcRef<GPUInner>>,

    #[cfg(feature = "software")]
    pub(crate) pixelbuffer: Option<ArcRef<PixelBufferInner>>,
}

impl WindowInner {
    pub fn process_event(&mut self) {
        for event in self.window_events.wait_borrow_mut().iter() {
            match event {
                event::WindowEvent::CloseRequested => {
                    self.graphics = None;
                    self.window_pointer = None;
                }
                event::WindowEvent::Resized(size) => {
                    if let Some(gpu) = &self.graphics {
                        gpu.wait_borrow_mut().resize(*size);
                    }

                    #[cfg(feature = "software")]
                    if let Some(softbuffer) = &self.pixelbuffer {
                        _ = softbuffer.wait_borrow_mut().resize(*size);
                    }

                    self.size = Point2::from(*size);
                }
                _ => {}
            }
        }
    }
}
