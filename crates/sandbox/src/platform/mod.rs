#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

pub mod fallback;

use crate::traits::SandboxProvider;

/// Selects the native `SandboxProvider` for the current operating system.
pub fn default_provider(execution_id: &str) -> Box<dyn SandboxProvider> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxProvider::probe(execution_id))
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsProvider::probe(execution_id))
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsProvider::probe(execution_id))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(fallback::FallbackProvider::probe(execution_id))
    }
}
