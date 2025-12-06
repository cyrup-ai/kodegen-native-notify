//! Runtime bundle ID setup for macOS UNUserNotificationCenter
//!
//! This module ensures that NSBundle.mainBundle.bundleIdentifier is set before
//! calling UNUserNotificationCenter APIs. For unbundled binaries (cargo run/test),
//! it uses a private API to set the bundle identifier in memory at runtime.
//!
//! Production .app bundles already have proper Info.plist and bundle identifiers,
//! so this is only needed for development/testing scenarios.

use objc2_foundation::NSBundle;
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use std::ffi::c_void;

/// Ensures a bundle identifier is set for the current process
///
/// **Production**: If running from a proper .app bundle, the bundle ID already exists
/// and this function returns it immediately.
///
/// **Development**: If running from cargo run/test without a bundle, this uses the
/// private API `_LSSetApplicationInformationItem` to set the bundle ID in memory.
///
/// Returns the bundle identifier string on success.
pub fn ensure_bundle_identifier() -> Result<String, BundleError> {
    // Check if bundle ID already exists (production .app bundles)
    let main_bundle = NSBundle::mainBundle();
    if let Some(existing_id) = main_bundle.bundleIdentifier() {
        return Ok(existing_id.to_string());
    }

    // Development path: Set bundle ID using private API
    let bundle_id = "ai.kodegen.notify";
    set_bundle_identifier_via_private_api(bundle_id)?;

    // Verify it worked
    let main_bundle = NSBundle::mainBundle();
    main_bundle
        .bundleIdentifier()
        .map(|id| id.to_string())
        .ok_or(BundleError::PrivateApiSetFailed)
}

/// Sets the bundle identifier using private Launch Services API
///
/// This uses `_LSSetApplicationInformationItem` from LaunchServices framework,
/// which is a private (non-public) API. It's used by Chromium and other browsers
/// for similar purposes.
///
/// **Why private API is acceptable here:**
/// - Only used in development (cargo run/test)
/// - Production .app bundles have proper Info.plist
/// - Not shipped to App Store
/// - Standard practice for development tools
fn set_bundle_identifier_via_private_api(bundle_id: &str) -> Result<(), BundleError> {
    unsafe {
        // Explicitly load LaunchServices framework
        unsafe extern "C" {
            fn dlopen(filename: *const i8, flag: i32) -> *mut c_void;
            fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
            fn dlclose(handle: *mut c_void) -> i32;
        }

        const RTLD_LAZY: i32 = 1;
        const RTLD_GLOBAL: i32 = 8;

        // Load CoreServices framework which contains LaunchServices
        let framework_path = c"/System/Library/Frameworks/CoreServices.framework/CoreServices".as_ptr();
        let framework_handle = dlopen(framework_path, RTLD_LAZY | RTLD_GLOBAL);

        if framework_handle.is_null() {
            return Err(BundleError::FrameworkNotFound);
        }

        // Get function pointer for private API
        let function_name = c"_LSSetApplicationInformationItem".as_ptr();
        let function_ptr = dlsym(framework_handle, function_name);

        if function_ptr.is_null() {
            dlclose(framework_handle);
            return Err(BundleError::PrivateApiFunctionNotFound);
        }

        // Cast to correct function signature
        // OSStatus _LSSetApplicationInformationItem(int, CFTypeRef, CFStringRef, CFTypeRef, CFDictionaryRef)
        type LSSetAppInfoFn = unsafe extern "C" fn(
            i32,           // inItemType (kLSDefaultSessionID = -2)
            *const c_void, // inItemRef (NULL for current process)
            CFStringRef,   // inKey (e.g., "CFBundleIdentifier")
            *const c_void, // inValue (CFString with bundle ID)
            *const c_void, // inDict (NULL)
        ) -> i32; // OSStatus

        let set_app_info: LSSetAppInfoFn = std::mem::transmute(function_ptr);

        // Prepare parameters
        let key = CFString::new("CFBundleIdentifier");
        let value = CFString::new(bundle_id);

        // Call private API
        // kLSDefaultSessionID = -2 (current session)
        let status = set_app_info(
            -2,                                    // kLSDefaultSessionID
            std::ptr::null(),                      // NULL = current process
            key.as_concrete_TypeRef(),             // key
            value.as_concrete_TypeRef() as *const c_void, // value
            std::ptr::null(),                      // NULL = no dictionary
        );

        // Cleanup framework handle
        dlclose(framework_handle);

        if status == 0 {
            Ok(())
        } else {
            Err(BundleError::PrivateApiCallFailed(status))
        }
    }
}

/// Errors that can occur during bundle setup
#[derive(Debug)]
pub enum BundleError {
    FrameworkNotFound,
    PrivateApiFunctionNotFound,
    PrivateApiCallFailed(i32),  // OSStatus error code
    PrivateApiSetFailed,
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameworkNotFound => write!(f, "LaunchServices framework not found"),
            Self::PrivateApiFunctionNotFound => write!(f, "_LSSetApplicationInformationItem function not found"),
            Self::PrivateApiCallFailed(code) => write!(f, "_LSSetApplicationInformationItem failed with OSStatus {}", code),
            Self::PrivateApiSetFailed => write!(f, "Failed to set bundle identifier (verification failed)"),
        }
    }
}

impl std::error::Error for BundleError {}
