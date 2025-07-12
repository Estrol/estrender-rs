#![allow(dead_code)]

use std::cell::RefMut;

// use crate::{gpu::gpu_inner::GPUInner, utils::ArcRef};

// use super::command::CommandBuffer;

use crate::utils::ArcRef;

use super::{
    command::CommandBuffer,
    GPUInner,
};

/// Represents the usage flags for a GPU buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BufferUsage(u32);

bitflags::bitflags! {
    impl BufferUsage: u32 {
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

impl Into<wgpu::BufferUsages> for BufferUsage {
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
    usage: BufferUsage,
    mapped: bool,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> BufferBuilder<T> {
    pub(crate) fn new(graphics: ArcRef<GPUInner>) -> Self {
        BufferBuilder {
            graphics,
            data: BufferData::None,
            usage: BufferUsage::empty(),
            len: 0,
            mapped: false,
        }
    }

    /// Set empty data for the buffer.
    pub fn set_data_empty(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    /// Set data for the buffer from a vector.
    pub fn set_data_vec(mut self, data: Vec<T>) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(&data).to_vec());
        self.len = data.len() * std::mem::size_of::<T>();
        self
    }

    /// Set data for the buffer from a slice.
    pub fn set_data_slice(mut self, data: &[T]) -> Self {
        self.data = BufferData::Data(bytemuck::cast_slice(data).to_vec());
        self.len = data.len() * std::mem::size_of::<T>();
        self
    }

    /// Set the buffer usage flags.
    pub fn set_usage(mut self, usage: BufferUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Set mapped state for the buffer.
    ///
    /// This is useful when you want to map the buffer for writing the data directly to the GPU memory.
    ///
    /// You have to call [Buffer::unmap] to unmap the buffer after you are done using it.
    /// Otherwise, the command will panic when you try to use the buffer on mapped state.
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BufferInner {
    pub buffer: wgpu::Buffer,

    pub size: wgpu::BufferAddress,
    pub usage: BufferUsage,
    pub mapped: bool,
}

/// Represents a GPU buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub(crate) graphics: ArcRef<GPUInner>,
    pub(crate) inner: ArcRef<BufferInner>,

    pub(crate) mapped_buffer: Vec<u8>, // Used for mapped buffers
    pub(crate) mapped_type: BufferMapMode, // Used for mapped buffers
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
        usage: BufferUsage,
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
            size,
            usage,
            mapped,
        };

        Ok(Buffer {
            graphics,
            inner: ArcRef::new(inner),
            mapped_buffer: if mapped {
                vec![0; size as usize]
            } else {
                vec![]
            },
            mapped_type: BufferMapMode::Write,
        })
    }

    pub(crate) fn from_slice<T: bytemuck::Pod>(
        graphics: ArcRef<GPUInner>,
        data: &[T],
        usage: BufferUsage,
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
            size,
            usage,
            mapped,
        };

        Ok(Buffer {
            graphics,
            inner: ArcRef::new(inner),
            mapped_buffer: if mapped {
                bytemuck::cast_slice(data).to_vec()
            } else {
                vec![]
            },
            mapped_type: BufferMapMode::Write,
        })
    }

    pub fn usage(&self) -> BufferUsage {
        self.inner.wait_borrow().usage
    }

    pub fn size(&self) -> u64 {
        self.inner.wait_borrow().size
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

                graphics_ref.create_buffer_with(&old_data, inner.usage.clone().into())
            } else {
                graphics_ref.create_buffer(
                    size as wgpu::BufferAddress,
                    inner.usage.clone().into(),
                    false,
                )
            }
        };

        inner.buffer = new_buffer;
        inner.size = size as wgpu::BufferAddress;

        Ok(())
    }

    /// Writes the contents of the source buffer to this buffer.
    pub fn write(&self, src: &Buffer) {
        let graphics_ref = self.graphics.borrow();
        let mut encoder =
            graphics_ref
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Write Command Encoder"),
                });

        self.internal_write_cmd(src, &mut encoder);

        graphics_ref
            .queue()
            .submit(std::iter::once(encoder.finish()));
        _ = graphics_ref.device().poll(wgpu::PollType::Wait);
    }

    /// Writes the contents of the source buffer to this buffer using a command buffer.
    ///
    /// This function is useful for when you want to write to the buffer in a command buffer context, such as during a render pass.
    ///
    /// [CommandBuffer::write_buffer] is a more convenient way to write a buffer in a command buffer context.
    pub fn write_cmd(&self, src: &Buffer, encoder: &mut CommandBuffer) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.wait_borrow();
            let src_inner = src.inner.wait_borrow();

            if !src_inner.usage.contains(BufferUsage::COPY_SRC) {
                panic!("Source buffer is not readable");
            }

            if inner.size < src_inner.size {
                panic!("Destination buffer is too small");
            }
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
            let inner = self.inner.wait_borrow();
            let src_inner = src.inner.wait_borrow();

            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if !src_inner.usage.contains(BufferUsage::COPY_SRC) {
                panic!("Source buffer is not readable");
            }

            if inner.size < src_inner.size {
                panic!("Destination buffer is too small");
            }
        }

        let src_inner = src.inner.wait_borrow();
        let inner = self.inner.wait_borrow();

        encoder.copy_buffer_to_buffer(&src_inner.buffer, 0, &inner.buffer, 0, inner.size);
    }

    #[inline(always)]
    pub(crate) fn internal_write_cmd(&self, src: &Buffer, encoder: &mut wgpu::CommandEncoder) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.wait_borrow();
            let src_inner = src.inner.wait_borrow();

            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if !src_inner.usage.contains(BufferUsage::COPY_SRC) {
                panic!("Source buffer is not readable");
            }

            if inner.size < src_inner.size {
                panic!("Destination buffer is too small");
            }
        }

        let src_inner = src.inner.wait_borrow();
        let inner = self.inner.wait_borrow();

        encoder.copy_buffer_to_buffer(&src_inner.buffer, 0, &inner.buffer, 0, inner.size);
    }

    /// Writes raw data to the buffer.
    ///
    /// By default, this will create an intermediate buffer to copy the data into, and then write that buffer to the destination buffer.
    /// This function also will automatically pad the data to the required alignment if necessary.
    ///
    /// Will panic if the buffer is not writable or if the data is larger than the buffer size.
    pub fn write_raw<T: bytemuck::Pod + bytemuck::Zeroable>(&self, data: &[T]) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.wait_borrow();
            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if inner.size < data.len() as u64 * std::mem::size_of::<T>() as u64 {
                panic!("Destination buffer is too small");
            }
        }

        let graphics_ref = self.graphics.borrow();

        let mut encoder =
            graphics_ref
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Buffer Write Raw Command Encoder"),
                });

        self.internal_write_raw_cmd(data, &mut encoder);

        graphics_ref
            .queue()
            .submit(std::iter::once(encoder.finish()));

        _ = graphics_ref.device().poll(wgpu::PollType::Wait);
    }

    /// Writes raw data to the buffer using a command buffer, useful for writing data during a render pass.
    ///
    /// This function is useful for when you want to write to the buffer in a command buffer context, such as during a render pass.
    /// This function also will automatically pad the data to the required alignment if necessary.
    ///
    /// [CommandBuffer::write_buffer_raw] is a more convenient way to write raw data to a buffer in a command buffer context.
    ///
    /// Will panic if the buffer is not writable or if the data is larger than the buffer size.
    pub fn write_raw_cmd<T: bytemuck::Pod + bytemuck::Zeroable>(
        &self,
        data: &[T],
        encoder: &mut CommandBuffer,
    ) {
        #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
        {
            let inner = self.inner.wait_borrow();

            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if inner.size < data.len() as u64 * std::mem::size_of::<T>() as u64 {
                panic!("Destination buffer is too small");
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
            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if inner.size < data_len {
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
            if !inner.usage.contains(BufferUsage::COPY_DST) {
                panic!("Buffer is not writable");
            }

            if inner.size < data_len {
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
        let mut graphics_ref = self.graphics.borrow_mut();
        let inner = self.inner.wait_borrow();

        if !inner.usage.contains(BufferUsage::COPY_SRC)
            && !inner.usage.contains(BufferUsage::MAP_READ)
        {
            return Err(BufferError::BufferNotReadable);
        }

        if inner.mapped {
            let data = inner.buffer.slice(..inner.size).get_mapped_range();
            let result = bytemuck::cast_slice(&data).to_vec();
            drop(data);

            Ok(result)
        } else {
            let buffer = graphics_ref.create_buffer(
                inner.size,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                false,
            );

            let mut encoder =
                graphics_ref
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Buffer Read Command Encoder"),
                    });

            encoder.copy_buffer_to_buffer(
                &inner.buffer,
                0,
                &buffer,
                0,
                inner.size as wgpu::BufferAddress,
            );

            graphics_ref
                .queue()
                .submit(std::iter::once(encoder.finish()));

            _ = graphics_ref.device().poll(wgpu::PollType::Wait);

            let result = {
                let mapped_buffer = buffer.slice(..inner.size).get_mapped_range();
                let result = bytemuck::cast_slice(&mapped_buffer).to_vec();

                result
            };

            Ok(result)
        }
    }

    pub fn map(&mut self, mode: BufferMapMode) -> Result<&mut Vec<u8>, BufferError> {
        let mut inner = self.inner.wait_borrow_mut();

        match mode {
            BufferMapMode::Write => {
                inner.mapped = true;

                self.mapped_buffer = vec![0; inner.size as usize];

                return Ok(&mut self.mapped_buffer);
            }
            BufferMapMode::Read => {
                inner.mapped = true;

                drop(inner);

                let buffer = self.read::<u8>()?;
                self.mapped_buffer = buffer;

                return Ok(&mut self.mapped_buffer);
            }
        }
    }

    pub fn unmap(&mut self) {
        if self.mapped_buffer.is_empty() {
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            {
                panic!("Buffer is not mapped");
            }

            #[allow(unreachable_code)]
            return;
        }

        let inner = self.inner.wait_borrow();
        if !inner.mapped {
            #[cfg(any(debug_assertions, feature = "enable-release-validation"))]
            {
                panic!("Buffer is not mapped");
            }

            #[allow(unreachable_code)]
            return;
        }

        match self.mapped_type {
            BufferMapMode::Write => {
                inner.buffer.unmap();

                drop(inner);

                self.write_raw(&self.mapped_buffer);
            }
            BufferMapMode::Read => {
                self.mapped_buffer = vec![];
            }
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
        self.inner.hash(state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferMapMode {
    Read,
    Write,
}
