#[derive(Debug, Clone)]
pub struct StagingBuffer {
    buffers: Vec<StagingBufferItem>,
}

const MAX_CYCLES: u64 = 60;

#[derive(Debug, Clone)]
pub struct StagingBufferItem {
    pub buffer: wgpu::Buffer,
    pub cycle: u64,
    pub used: bool,
}

impl StagingBuffer {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    pub fn cycle(&mut self) {
        for item in &mut self.buffers {
            item.cycle += 1;
            item.used = false;
        }
        
        self.buffers.retain(|item| item.cycle < MAX_CYCLES);
    }

    pub fn allocate(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let aligned = wgpu::COPY_BUFFER_ALIGNMENT;
        let size = (data.len() as wgpu::BufferAddress + aligned - 1) / aligned * aligned;

        let buffer = {
            if let Some(item) = self.buffers.iter_mut().find(|item| !item.used && item.buffer.size() >= size) {
                item.used = true;
                item.cycle = 0;
                item.buffer.clone()
            } else {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size,
                    usage,
                    mapped_at_creation: false,
                });

                self.buffers.push(StagingBufferItem {
                    buffer: buffer.clone(),
                    cycle: 0,
                    used: true,
                });

                buffer
            }
        };

        let aligned_data = {
            let mut aligned_data = vec![0u8; size as usize];
            aligned_data[..data.len()].copy_from_slice(data);
            aligned_data
        };

        queue.write_buffer(&buffer, 0, &aligned_data);

        buffer
    }
}