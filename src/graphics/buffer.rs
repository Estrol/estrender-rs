#![allow(dead_code)]

use wgpu::util::DeviceExt;

use crate::utils::ArcRef;

use super::inner::GPUInner;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferUsages(u32);

bitflags::bitflags! {
    impl BufferUsages: u32 {
        const MAP_READ = 0x0001;
        const MAP_WRITE = 0x0002;
        const COPY_SRC = 0x0004;
        const COPY_DST = 0x0008;
        const INDEX = 0x0010;
        const VERTEX = 0x0020;
        const UNIFORM = 0x0040;
        const STORAGE = 0x0080;
        const INDIRECT = 0x0100;
        const QUERY_RESOLVE = 0x0200;
    }
}

impl Into<wgpu::BufferUsages> for BufferUsages {
    fn into(self) -> wgpu::BufferUsages {
        wgpu::BufferUsages::from_bits(self.bits()).unwrap()
    }
}

pub enum BufferData<T: bytemuck::Pod> {
    None,
    Empty(usize),
    Data(Vec<T>),
}

pub struct BufferBuilder<T: bytemuck::Pod> {
    graphics: ArcRef<GPUInner>,
    data: BufferData<T>,
    usage: BufferUsages,
}

impl<T: bytemuck::Pod> BufferBuilder<T> {
    pub fn new(graphics: ArcRef<GPUInner>) -> Self {
        BufferBuilder {
            graphics,
            data: BufferData::None,
            usage: BufferUsages::empty(),
        }
    }

    pub fn with_data(mut self, data: Vec<T>) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(&data).to_vec());
        self
    }

    pub fn with_slice(mut self, data: &[T]) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(data).to_vec());
        self
    }

    pub fn with_empty(mut self, size: usize) -> Self {
        self.data = BufferData::Empty(size);
        self
    }

    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn build(self) -> Buffer {
        match self.data {
            BufferData::None => panic!("Buffer data is not set"),

            BufferData::Empty(size) => {
                Buffer::new(self.graphics, size as wgpu::BufferAddress, self.usage)
            }
            BufferData::Data(data) => Buffer::from_slice(self.graphics, &data, self.usage),
        }
    }
}

pub struct BufferInner {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) size: wgpu::BufferAddress,
    pub(crate) usage: wgpu::BufferUsages,
}

#[derive(Clone)]
pub struct Buffer {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<BufferInner>,

    pub size: u64,
    pub usage: BufferUsages,
}

impl Buffer {
    pub(crate) fn new(
        graphics: ArcRef<GPUInner>,
        size: wgpu::BufferAddress,
        usage: BufferUsages,
    ) -> Self {
        let graphics_ref = graphics.borrow();
        let usage_wgpu: wgpu::BufferUsages = usage.clone().into();

        let buffer = graphics_ref.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: usage_wgpu,
            mapped_at_creation: false,
        });

        drop(graphics_ref);

        let inner = BufferInner {
            buffer,
            size,
            usage: usage_wgpu,
        };

        Buffer {
            graphics,
            inner: ArcRef::new(inner),
            size: size as u64,
            usage,
        }
    }

    pub(crate) fn from_slice<T: bytemuck::Pod>(
        graphics: ArcRef<GPUInner>,
        data: &[T],
        usage: BufferUsages,
    ) -> Self {
        let graphics_ref = graphics.borrow();
        let usage_wgpu: wgpu::BufferUsages = usage.clone().into();

        let buffer = graphics_ref
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: usage_wgpu,
            });

        drop(graphics_ref);

        let size = (data.len() * std::mem::size_of::<T>()) as wgpu::BufferAddress;

        let inner = BufferInner {
            buffer,
            size,
            usage: usage_wgpu,
        };

        Buffer {
            graphics,
            inner: ArcRef::new(inner),
            size: size as u64,
            usage,
        }
    }

    pub fn write(&self, src: &Buffer) {
        let graphics_ref = self.graphics.borrow();
        let mut encoder =
            graphics_ref
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Write Command Encoder"),
                });

        self.write_cmd(src, &mut encoder);
        graphics_ref.queue.submit(std::iter::once(encoder.finish()));
        graphics_ref.device.poll(wgpu::Maintain::Wait);
    }

    pub fn write_cmd(&self, src: &Buffer, encoder: &mut wgpu::CommandEncoder) {
        if !self.usage.contains(BufferUsages::COPY_DST) {
            panic!("Buffer is not writable");
        }

        if !src.usage.contains(BufferUsages::COPY_SRC) {
            panic!("Source buffer is not readable");
        }

        if self.size < src.size {
            panic!("Destination buffer is too small");
        }

        let src_inner = src.inner.wait_borrow();
        let inner = self.inner.wait_borrow();

        encoder.copy_buffer_to_buffer(&src_inner.buffer, 0, &inner.buffer, 0, self.size);
    }

    pub fn write_raw<T: bytemuck::Pod>(&self, data: &[T]) {
        if !self.usage.contains(BufferUsages::COPY_DST) {
            panic!("Buffer is not writable");
        }

        let graphics_ref = self.graphics.borrow();
        let mut cmd = graphics_ref
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Buffer Write Command Encoder"),
            });

        self.write_raw_cmd(data, &mut cmd);

        graphics_ref.queue.submit(std::iter::once(cmd.finish()));
        graphics_ref.device.poll(wgpu::Maintain::Wait);
    }

    pub(crate) fn write_raw_cmd<T: bytemuck::Pod>(
        &self,
        data: &[T],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if !self.usage.contains(BufferUsages::COPY_DST) {
            panic!("Buffer is not writable");
        }

        let mut graphics_ref = self.graphics.borrow_mut();
        let inner = self.inner.wait_borrow();

        let buffer = graphics_ref.make_buffer(
            self.size,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
        );

        if !futures::executor::block_on(Self::map_buffer(
            &graphics_ref.device,
            &buffer,
            wgpu::MapMode::Write,
        )) {
            panic!("Failed to map buffer");
        }

        let mut buffer_data = buffer.slice(..).get_mapped_range_mut();
        buffer_data.copy_from_slice(bytemuck::cast_slice(&data)); // Safe conversion

        drop(buffer_data);
        buffer.unmap();

        encoder.copy_buffer_to_buffer(&buffer, 0, &inner.buffer, 0, self.size);
    }

    pub fn read<T: bytemuck::Pod>(&self) -> Result<Vec<T>, String> {
        if !self.usage.contains(BufferUsages::COPY_SRC)
            && !self.usage.contains(BufferUsages::MAP_READ)
        {
            return Err("Buffer is not readable".to_string());
        }

        let mut graphics_ref = self.graphics.borrow_mut();
        let inner = self.inner.wait_borrow();

        // Use the existing buffer if it has MAP_READ, otherwise create a temporary one
        let buffer = if self.usage.contains(BufferUsages::MAP_READ) {
            &inner.buffer
        } else {
            &graphics_ref.make_buffer(
                self.size,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            )
        };

        // If we created a new buffer, copy data from the original
        let mut encoder =
            graphics_ref
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Read Encoder"),
                });

        if !self.usage.contains(BufferUsages::MAP_READ) {
            encoder.copy_buffer_to_buffer(&inner.buffer, 0, buffer, 0, self.size);
            graphics_ref.queue.submit(std::iter::once(encoder.finish()));

            // Ensure the GPU has completed execution
            graphics_ref.device.poll(wgpu::Maintain::Wait);
        }

        // Wait until the buffer is mapped
        if !futures::executor::block_on(Self::map_buffer(
            &graphics_ref.device,
            buffer,
            wgpu::MapMode::Read,
        )) {
            return Err("Failed to map buffer".to_string());
        }

        // Read mapped data
        let data = buffer.slice(..).get_mapped_range();
        let result = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        buffer.unmap();

        Ok(result)
    }

    async fn map_buffer(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        map_mode: wgpu::MapMode,
    ) -> bool {
        let (sender, receiver) = futures::channel::oneshot::channel();

        buffer.slice(..).map_async(map_mode, |result| {
            let _ = sender.send(result);
        });

        device.poll(wgpu::Maintain::Wait);

        receiver.await.unwrap().is_ok()
    }
}

impl std::hash::Hash for Buffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Don't hash during borrow
        self.inner.wait_borrow().buffer.hash(state);
        self.size.hash(state);
        self.usage.hash(state);
    }
}
