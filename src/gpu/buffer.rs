#![allow(dead_code)]

use std::cell::RefMut;

use crate::{dbg_log, utils::ArcRef};

use super::{command::CommandBuffer, inner::GPUInner};

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

pub enum BufferData<T: bytemuck::Pod + bytemuck::Zeroable> {
    None,
    Data(Vec<T>),
}

pub struct BufferBuilder<T: bytemuck::Pod + bytemuck::Zeroable> {
    graphics: ArcRef<GPUInner>,
    data: BufferData<T>,
    len: usize,
    usage: BufferUsages,
    mapped: bool,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> BufferBuilder<T> {
    pub(crate) fn new(graphics: ArcRef<GPUInner>) -> Self {
        BufferBuilder {
            graphics,
            data: BufferData::None,
            usage: BufferUsages::empty(),
            len: 0,
            mapped: false,
        }
    }

    pub fn set_len(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    pub fn set_data_vec(mut self, data: Vec<T>) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(&data).to_vec());
        self.len = data.len() * std::mem::size_of::<T>();
        self
    }

    pub fn set_data_slice(mut self, data: &[T]) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(data).to_vec());
        self.len = data.len() * std::mem::size_of::<T>();
        self
    }

    pub fn set_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    pub fn set_mapped(mut self, mapped: bool) -> Self {
        self.mapped = mapped;
        self
    }

    pub fn build(self) -> Result<Buffer, BufferError> {
        if self.len == 0 && matches!(self.data, BufferData::None) {
            return Err(BufferError::InvalidSize);
        }

        match self.data {
            BufferData::None => Buffer::new(
                self.graphics,
                self.len as wgpu::BufferAddress,
                self.usage,
                self.mapped,
            ),
            BufferData::Data(data) => {
                Buffer::from_slice(self.graphics, &data, self.usage, self.mapped)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntermediateBuffer {
    pub buffer: wgpu::Buffer,
    pub write: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BufferInner {
    pub buffer: wgpu::Buffer,
    pub intermediate_buffer: Option<IntermediateBuffer>,

    pub size: wgpu::BufferAddress,
    pub usage: wgpu::BufferUsages,
    pub mapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<BufferInner>,

    pub size: u64,
    pub usage: BufferUsages,
}

#[derive(Debug, Clone, Copy)]
pub enum BufferError {
    InvalidUsage,
    InvalidSize,
    BufferNotReadable,
    BufferNotWritable,
    FailedToMapBuffer,
}

impl Buffer {
    pub(crate) fn new(
        graphics: ArcRef<GPUInner>,
        size: wgpu::BufferAddress,
        usage: BufferUsages,
        mapped: bool,
    ) -> Result<Self, BufferError> {
        if size == 0 {
            return Err(BufferError::InvalidSize);
        }

        let buffer = {
            let mut graphics_ref = graphics.borrow_mut();
            let usage_wgpu: wgpu::BufferUsages = usage.clone().into();

            graphics_ref.create_buffer(size, usage_wgpu, mapped)
        };

        let inner = BufferInner {
            buffer,
            intermediate_buffer: None,
            size,
            usage: usage.clone().into(),
            mapped,
        };

        Ok(Buffer {
            graphics,
            inner: ArcRef::new(inner),
            size: size as u64,
            usage,
        })
    }

    pub(crate) fn from_slice<T: bytemuck::Pod>(
        graphics: ArcRef<GPUInner>,
        data: &[T],
        usage: BufferUsages,
        mapped: bool,
    ) -> Result<Self, BufferError> {
        if data.is_empty() {
            return Err(BufferError::InvalidSize);
        }

        let size = (data.len() * std::mem::size_of::<T>()) as wgpu::BufferAddress;
        let buffer = {
            let mut graphics_ref = graphics.borrow_mut();
            let usage_wgpu: wgpu::BufferUsages = usage.clone().into();

            graphics_ref.create_buffer_with(data, usage_wgpu)
        };

        let inner = BufferInner {
            buffer,
            intermediate_buffer: None,
            size,
            usage: usage.clone().into(),
            mapped,
        };

        Ok(Buffer {
            graphics,
            inner: ArcRef::new(inner),
            size: size as u64,
            usage,
        })
    }

    /// Resizes the buffer to the specified size.
    ///
    /// Due to the nature of GPU buffers, this will create a new buffer and copy the existing data into it IF: \
    /// - The old buffer has usage [BufferUsages::COPY_SRC] and [BufferUsages::MAP_READ].
    ///
    /// Otherwise, it will simply resize the buffer without copying the data.
    pub fn resize(&mut self, size: u64) -> Result<(), BufferError> {
        if size == 0 {
            return Err(BufferError::InvalidSize);
        }

        let old_data = self.read::<u8>();

        let mut inner = self.inner.wait_borrow_mut();
        let mut graphics_ref = self.graphics.borrow_mut();

        let new_buffer = {
            if let Ok(old_data) = old_data {
                // truance or increase old data
                let mut old_data = old_data;

                if old_data.len() < size as usize {
                    // If the old data is smaller than the new size, we need to pad it with zeros
                    old_data.resize(size as usize, 0);
                } else if old_data.len() > size as usize {
                    // If the old data is larger than the new size, we need to truncate it
                    old_data.truncate(size as usize);
                }

                graphics_ref.create_buffer_with(&old_data, self.usage.clone().into())
            } else {
                graphics_ref.create_buffer(
                    size as wgpu::BufferAddress,
                    self.usage.clone().into(),
                    false,
                )
            }
        };

        inner.buffer = new_buffer;
        inner.size = size as wgpu::BufferAddress;

        self.size = size;

        Ok(())
    }

    pub fn write(&self, src: &Buffer) {
        let graphics_ref = self.graphics.borrow();
        let mut encoder =
            graphics_ref
                .get_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Write Command Encoder"),
                });

        self.internal_write_cmd(src, &mut encoder);

        graphics_ref
            .get_queue()
            .submit(std::iter::once(encoder.finish()));
        _ = graphics_ref.get_device().poll(wgpu::PollType::Wait);
    }

    pub fn write_cmd(&self, src: &Buffer, encoder: &mut CommandBuffer) {
        if !self.usage.contains(BufferUsages::COPY_DST) {
            panic!("Buffer is not writable");
        }

        if encoder.command.is_none() {
            panic!("Command buffer is not writable");
        }

        self.internal_write_cmd(src, &mut encoder.command.as_mut().unwrap().borrow_mut());
    }

    #[inline(always)]
    pub(crate) fn internal_write_cmd_mut_ref(
        &self,
        src: &Buffer,
        encoder: &mut RefMut<'_, wgpu::CommandEncoder>,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if !self.usage.contains(BufferUsages::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if !src.usage.contains(BufferUsages::COPY_SRC) {
                panic!("Source buffer is not readable");
            }

            if self.size < src.size {
                panic!("Destination buffer is too small");
            }
        }

        let src_inner = src.inner.wait_borrow();
        let inner = self.inner.wait_borrow();

        encoder.copy_buffer_to_buffer(&src_inner.buffer, 0, &inner.buffer, 0, self.size);
    }

    #[inline(always)]
    pub(crate) fn internal_write_cmd(&self, src: &Buffer, encoder: &mut wgpu::CommandEncoder) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if !self.usage.contains(BufferUsages::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if !src.usage.contains(BufferUsages::COPY_SRC) {
                panic!("Source buffer is not readable");
            }

            if self.size < src.size {
                panic!("Destination buffer is too small");
            }
        }

        let src_inner = src.inner.wait_borrow();
        let inner = self.inner.wait_borrow();

        encoder.copy_buffer_to_buffer(&src_inner.buffer, 0, &inner.buffer, 0, self.size);
    }

    pub fn write_raw<T: bytemuck::Pod + bytemuck::Zeroable>(&self, data: &[T]) {
        if !self.usage.contains(BufferUsages::COPY_DST) {
            panic!("Buffer is not writable");
        }

        let graphics_ref = self.graphics.borrow();

        let mut encoder =
            graphics_ref
                .get_device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Write Raw Command Encoder"),
                });

        self.internal_write_raw_cmd(data, &mut encoder);

        graphics_ref
            .get_queue()
            .submit(std::iter::once(encoder.finish()));

        _ = graphics_ref.get_device().poll(wgpu::PollType::Wait);
    }

    pub fn write_raw_cmd<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        data: &[T],
        encoder: &mut CommandBuffer,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if self.size < data.len() as u64 * std::mem::size_of::<T>() as u64 {
                panic!("Destination buffer is too small");
            }

            if !self.usage.contains(BufferUsages::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if encoder.command.is_none() {
                panic!("Command buffer is not writable");
            }
        }

        let mut cmd = encoder.command.as_mut().unwrap().borrow_mut();

        self.internal_write_raw_cmd(data, &mut cmd);
    }

    pub(crate) fn internal_write_raw_cmd<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        data: &[T],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let inner = self.inner.wait_borrow();
        let mut graphics_ref = self.graphics.borrow_mut();

        let data_len = data.len() as u64 * std::mem::size_of::<T>() as u64;

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if !self.usage.contains(BufferUsages::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if self.size < data_len {
                panic!("Destination buffer is too small");
            }
        }

        let buffer = {
            let data: Vec<u8> = bytemuck::cast_slice(data).to_vec();

            if data.len() as wgpu::BufferAddress % wgpu::COPY_BUFFER_ALIGNMENT != 0 {
                // If the data length is not aligned, we need to pad it
                let mut padded_data = data.to_vec();
                padded_data.resize(
                    ((data_len + wgpu::COPY_BUFFER_ALIGNMENT as u64 - 1)
                        / wgpu::COPY_BUFFER_ALIGNMENT as u64
                        * wgpu::COPY_BUFFER_ALIGNMENT as u64) as usize,
                    0,
                );

                graphics_ref.create_buffer_with(&padded_data, wgpu::BufferUsages::COPY_SRC)
            } else {
                graphics_ref.create_buffer_with(&data, wgpu::BufferUsages::COPY_SRC)
            }
        };

        encoder.copy_buffer_to_buffer(
            &buffer,
            0,
            &inner.buffer,
            0,
            buffer.size() as wgpu::BufferAddress,
        );
    }

    pub(crate) fn internal_write_raw_cmd_ref<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        data: &[T],
        encoder: &mut RefMut<'_, wgpu::CommandEncoder>,
    ) {
        let inner = self.inner.wait_borrow();
        let mut graphics_ref = self.graphics.borrow_mut();

        let data_len = data.len() as u64 * std::mem::size_of::<T>() as u64;

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if !self.usage.contains(BufferUsages::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if self.size < data_len {
                panic!("Destination buffer is too small");
            }
        }

        let buffer = graphics_ref.create_buffer_with(data, wgpu::BufferUsages::COPY_SRC);

        encoder.copy_buffer_to_buffer(
            &buffer,
            0,
            &inner.buffer,
            0,
            buffer.size() as wgpu::BufferAddress,
        );
    }

    /// Reads the buffer data into a vector of type T.
    ///
    /// Unless if the buffer was created with [BufferUsages::COPY_SRC] or [BufferUsages::MAP_READ], this will create an
    /// intermediate buffer to copy the data into, and then read from that buffer.
    pub fn read<T: bytemuck::Pod + bytemuck::Zeroable>(&self) -> Result<Vec<T>, BufferError> {
        if !self.usage.contains(BufferUsages::COPY_SRC)
            && !self.usage.contains(BufferUsages::MAP_READ)
        {
            return Err(BufferError::BufferNotReadable);
        }

        let mut graphics_ref = self.graphics.borrow_mut();
        let inner = self.inner.wait_borrow();

        if inner.mapped {
            let data = inner.buffer.slice(..self.size).get_mapped_range();
            let result = bytemuck::cast_slice(&data).to_vec();
            drop(data);

            Ok(result)
        } else {
            let buffer = graphics_ref.create_buffer(
                self.size,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                false,
            );

            let mut encoder =
                graphics_ref
                    .get_device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Buffer Read Command Encoder"),
                    });

            encoder.copy_buffer_to_buffer(
                &inner.buffer,
                0,
                &buffer,
                0,
                self.size as wgpu::BufferAddress,
            );

            graphics_ref
                .get_queue()
                .submit(std::iter::once(encoder.finish()));

            _ = graphics_ref.get_device().poll(wgpu::PollType::Wait);

            let mapped_buffer = buffer.slice(..self.size).get_mapped_range();

            let result = bytemuck::cast_slice(&mapped_buffer).to_vec();
            drop(mapped_buffer);

            Ok(result)
        }
    }

    pub fn map(&mut self, mode: BufferMapMode) -> Result<(), BufferError> {
        let mut inner = self.inner.wait_borrow_mut();
        if inner.mapped {
            return Ok(());
        }

        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            if !inner.usage.contains(wgpu::BufferUsages::MAP_READ)
                && !inner.usage.contains(wgpu::BufferUsages::MAP_WRITE)
            {
                panic!("Buffer is not mappable");
            }

            if mode == BufferMapMode::Write && !inner.usage.contains(wgpu::BufferUsages::MAP_WRITE)
            {
                panic!("Buffer is not writable");
            }

            if mode == BufferMapMode::Read && !inner.usage.contains(wgpu::BufferUsages::MAP_READ) {
                panic!("Buffer is not readable");
            }
        }

        let graphics_ref = self.graphics.borrow();
        let device = graphics_ref.get_device();
        let buffer = &inner.buffer;

        let result = futures::executor::block_on(Self::map_buffer(
            device,
            buffer,
            match mode {
                BufferMapMode::Read => wgpu::MapMode::Read,
                BufferMapMode::Write => wgpu::MapMode::Write,
            },
        ));

        if !result {
            dbg_log!("Failed to map buffer");
            return Err(BufferError::FailedToMapBuffer);
        }

        inner.mapped = true;

        dbg_log!("Buffer mapped successfully");
        Ok(())
    }

    pub fn unmap(&mut self) {
        let mut inner = self.inner.wait_borrow_mut();
        if inner.mapped {
            inner.buffer.unmap();
            inner.mapped = false;
        }
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

        _ = device.poll(wgpu::PollType::Wait);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferMapMode {
    Read,
    Write,
}
