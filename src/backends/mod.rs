// Real platform backends - complete implementations
// Each backend provides actual platform integration

pub mod linux;
pub mod macos;
pub mod windows;

use std::collections::HashMap;

use crate::components::platform::{Platform, PlatformBackend};

/// Factory for creating platform-specific backends
pub struct PlatformBackendFactory;

impl PlatformBackendFactory {
    /// Create a backend for the specified platform
    /// Returns None if platform is not supported on current OS
    pub fn create_backend(platform: Platform) -> Option<Box<dyn PlatformBackend>> {
        match platform {
            #[cfg(target_os = "macos")]
            Platform::MacOS => Some(Box::new(macos::MacOSBackend::new())),

            #[cfg(target_os = "windows")]
            Platform::Windows => Some(Box::new(windows::WindowsBackend::new())),

            #[cfg(target_os = "linux")]
            Platform::Linux => Some(Box::new(linux::LinuxBackend::new())),

            // Return None for unsupported platforms on current OS
            _ => None,
        }
    }

    /// Get all supported backends for current platform
    pub fn get_supported_backends() -> HashMap<Platform, Box<dyn PlatformBackend>> {
        let mut backends = HashMap::new();

        #[cfg(target_os = "macos")]
        backends.insert(
            Platform::MacOS,
            Box::new(macos::MacOSBackend::new()) as Box<dyn PlatformBackend>,
        );

        #[cfg(target_os = "windows")]
        backends.insert(
            Platform::Windows,
            Box::new(windows::WindowsBackend::new()) as Box<dyn PlatformBackend>,
        );

        #[cfg(target_os = "linux")]
        backends.insert(
            Platform::Linux,
            Box::new(linux::LinuxBackend::new()) as Box<dyn PlatformBackend>,
        );

        backends
    }
}
