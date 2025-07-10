#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterBackend {
    None,
    Vulkan,
    Metal,
    Dx12,
    Gl,
    BrowserWebGpu,
}

#[derive(Clone, Debug)]
pub enum GPUWaitType {
    Wait,
    Poll,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SwapchainError {
    NotAvailable,
    ConfigNeeded,
    DeviceLost,
    Suboptimal(wgpu::SurfaceTexture),
}

impl std::fmt::Display for SwapchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapchainError::NotAvailable => write!(f, "Swapchain not available"),
            SwapchainError::ConfigNeeded => write!(f, "Swapchain config needed"),
            SwapchainError::DeviceLost => write!(f, "Device lost"),
            SwapchainError::Suboptimal(_) => write!(f, "Swapchain suboptimal"),
        }
    }
}
