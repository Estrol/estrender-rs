// use crate::utils::ArcRef;

// use super::inner::GPUInner;

// pub struct SamplerBuilder {}

// pub struct Sampler {
//     pub inner: wgpu::Sampler,
// }

// impl Sampler {
//     pub(crate) fn new(graphics: &ArcRef<GPUInner>, desc: &wgpu::SamplerDescriptor) -> Sampler {
//         let inner = graphics.borrow().device.create_sampler(desc);

//         Sampler { inner }
//     }
// }
