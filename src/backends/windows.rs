// Windows Toast Notifications backend - Complete implementation

#[cfg(target_os = "windows")]
use std::collections::HashMap;

#[cfg(target_os = "windows")]
use std::sync::Arc;

#[cfg(target_os = "windows")]
use std::time::SystemTime;

#[cfg(target_os = "windows")]
use tokio::sync::OnceCell;

#[cfg(target_os = "windows")]
use windows::{
    ApplicationModel::AppInfo,
    Data::Xml::Dom::XmlDocument,
    Foundation::DateTime,
    UI::Notifications::{ToastNotification, ToastNotificationManager, ToastNotifier},
    core::{HSTRING, Result as WindowsResult},
};

use crate::components::NotificationResult;
use crate::components::platform::{
    DeliveryReceipt, NotificationRequest, NotificationUpdate, PlatformBackend, PlatformCapabilities,
};

#[cfg(target_os = "windows")]
use crate::components::platform::{CompatibilityLevel, PermissionLevel};

#[cfg(target_os = "windows")]
use crate::components::Platform;

pub struct WindowsBackend {
    #[cfg(target_os = "windows")]
    app_id: String,
    #[cfg(target_os = "windows")]
    notifier: Arc<OnceCell<ToastNotifier>>,
}

impl Default for WindowsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            app_id: "EcsNotifications.App".to_string(),
            #[cfg(target_os = "windows")]
            notifier: Arc::new(OnceCell::new()),
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_notifier(&self) -> Result<ToastNotifier, crate::components::NotificationError> {
        self.notifier
            .get_or_try_init(|| async {
                let app_id = HSTRING::from(&self.app_id);
                ToastNotificationManager::CreateToastNotifierWithId(&app_id).map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create toast notifier: {:?}", e),
                    }
                })
            })
            .await
            .cloned()
    }

    #[cfg(target_os = "windows")]
    fn create_toast_xml(
        &self,
        title: &str,
        subtitle: Option<&str>,
        body: &str,
        hero_image_url: Option<&str>,
        app_logo_url: Option<&str>,
    ) -> WindowsResult<XmlDocument> {
        let escaped_title = xml_escape(title);
        let escaped_body = xml_escape(body);
        
        // Hero image (large banner at top)
        let hero_element = hero_image_url
            .map(|url| format!(
                r#"<image placement="hero" src="{}"/>"#,
                xml_escape(url)
            ))
            .unwrap_or_default();
        
        // App logo override (replaces default app icon)
        let logo_element = app_logo_url
            .map(|url| format!(
                r#"<image placement="appLogoOverride" hint-crop="circle" src="{}"/>"#,
                xml_escape(url)
            ))
            .unwrap_or_default();
        
        // Subtitle as secondary styled text
        let subtitle_element = subtitle
            .map(|s| format!(
                r#"<text hint-style="captionSubtle">{}</text>"#,
                xml_escape(s)
            ))
            .unwrap_or_default();
        
        let toast_xml = format!(
            r#"<toast>
    <visual>
        <binding template="ToastGeneric">
            {hero_element}
            {logo_element}
            <text hint-style="title">{escaped_title}</text>
            {subtitle_element}
            <text hint-style="body">{escaped_body}</text>
        </binding>
    </visual>
    <audio src="ms-winsoundevent:Notification.Default"/>
</toast>"#
        );

        let xml_doc = XmlDocument::new()?;
        let xml_hstring = HSTRING::from(&toast_xml);
        xml_doc.LoadXml(&xml_hstring)?;
        Ok(xml_doc)
    }
}

#[cfg(target_os = "windows")]
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl WindowsBackend {
    pub async fn check_authorization(&self) -> NotificationResult<bool> {
        #[cfg(target_os = "windows")]
        {
            // Windows Toast notifications don't require explicit authorization
            // They work by default unless disabled in system settings
            Ok(true)
        }
        #[cfg(not(target_os = "windows"))]
        Ok(false)
    }
}

impl PlatformBackend for WindowsBackend {
    fn negotiate_capabilities(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<PlatformCapabilities>> + Send + '_>,
    > {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                // Check if we can create a toast notifier (indicates Windows 10+ with notification
                // support)
                let app_id = HSTRING::from("EcsNotifications.Capability.Test");
                let test_result = ToastNotificationManager::CreateToastNotifierWithId(&app_id);

                let supports_toasts = test_result.is_ok();

                Ok(PlatformCapabilities {
                    supports_actions: supports_toasts,
                    supports_rich_media: supports_toasts,
                    supports_markup: true, // Windows supports XML markup
                    supports_sound: supports_toasts,
                    supports_scheduling: false, // Windows doesn't support scheduled toasts directly
                    supports_progress: true,    // Windows supports progress bars in toasts
                    supports_categories: false,
                    supports_replies: supports_toasts,
                    supports_custom_ui: true, // Windows supports custom toast templates
                    supports_background_activation: supports_toasts,
                    supports_update_content: true,
                    supports_persistent: true,
                    supports_priority: false,
                    supports_grouping: true,
                    supports_badges: false,
                    supports_vibration: false,
                    max_actions: Some(5),
                    max_title_length: Some(128),
                    max_body_length: Some(1024),
                    max_image_size: Some(204_800), // 200KB limit for toast images
                    max_sound_duration: Some(std::time::Duration::from_secs(10)),
                    platform_features: {
                        let mut features = HashMap::new();
                        features.insert("adaptive_templates".to_string(), true);
                        features.insert("xml_content".to_string(), true);
                        features.insert("input_elements".to_string(), supports_toasts);
                        features
                    },
                    platform_limits: {
                        let mut limits = HashMap::new();
                        limits.insert("max_buttons".to_string(), 5);
                        limits.insert("max_inputs".to_string(), 3);
                        limits.insert("xml_size_limit".to_string(), 5120); // 5KB XML limit
                        limits
                    },
                    authorization_required: false, /* Windows doesn't require explicit permission
                                                    * for
                                                    * toasts */
                    permission_levels: vec![
                        PermissionLevel::Display,
                        PermissionLevel::Actions,
                        PermissionLevel::Media,
                    ],
                    platform_version: Some("Windows 10+".to_string()),
                    api_version: Some("WinRT".to_string()),
                    compatibility_level: if supports_toasts {
                        CompatibilityLevel::Full
                    } else {
                        CompatibilityLevel::None
                    },
                    delivery_latency_estimate: Some(std::time::Duration::from_millis(50)),
                    supports_batching: false,
                    rate_limits: None,
                })
            }

            #[cfg(not(target_os = "windows"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Windows".to_string(),
                    error_code: None,
                    message: "Windows backend not available on this platform".to_string(),
                })
            }
        })
    }

    #[allow(unused_variables)]
    fn deliver_notification(
        &self,
        request: &NotificationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<DeliveryReceipt>> + Send + '_>,
    > {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                let notifier = self.get_notifier().await?;

                // Resolve all media images (downloads remote URLs to temp files)
                let resolved_images = super::image_utils::resolve_media_images(&request.content.media).await;

                // Extract hero image path from resolved images
                let hero_image_path = resolved_images.iter()
                    .find(|(placement, _)| *placement == crate::components::ImagePlacement::Hero)
                    .map(|(_, resolved)| resolved.path.to_string_lossy().to_string());

                // Extract app logo path from resolved images
                let app_logo_path = resolved_images.iter()
                    .find(|(placement, _)| *placement == crate::components::ImagePlacement::AppIcon)
                    .map(|(_, resolved)| resolved.path.to_string_lossy().to_string());

                // Extract subtitle from content or body
                let subtitle = request.content.subtitle.clone()
                    .or_else(|| request.content.body.extract_subtitle());

                // Create XML content for the toast with enhanced formatting
                // Use local file paths (downloaded from remote URLs if needed)
                let xml_doc = self
                    .create_toast_xml(
                        &request.content.title,
                        subtitle.as_deref(),
                        &request.content.body.to_structured_plain_text(),
                        hero_image_path.as_deref(),
                        app_logo_path.as_deref(),
                    )
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create toast XML: {:?}", e),
                    })?;

                // Create the toast notification
                let toast = ToastNotification::CreateToastNotification(&xml_doc).map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create toast notification: {:?}", e),
                    }
                })?;

                // Set expiration time if specified
                if let Some(ttl) = request.options.ttl {
                    let expiry_time = SystemTime::now() + ttl;

                    // Convert SystemTime to Windows DateTime
                    match expiry_time.duration_since(SystemTime::UNIX_EPOCH) {
                        Ok(duration) => {
                            // Windows DateTime uses 100-nanosecond intervals since January 1, 1601
                            // Unix epoch is January 1, 1970, which is 11644473600 seconds later
                            let windows_epoch_offset = 11644473600u64;
                            let total_seconds = duration.as_secs() + windows_epoch_offset;
                            let total_nanos =
                                total_seconds * 10_000_000 + (duration.subsec_nanos() as u64) / 100;

                            // Create Windows DateTime
                            let windows_datetime = windows::Foundation::DateTime {
                                UniversalTime: total_nanos as i64,
                            };

                            // Set expiration time on toast
                            if let Err(e) = toast.SetExpirationTime(&windows_datetime) {
                                eprintln!("Warning: Failed to set expiration time: {:?}", e);
                            }
                        },
                        Err(e) => {
                            eprintln!("Warning: Failed to calculate expiration time: {:?}", e);
                        },
                    }
                }

                // Show the toast
                let start_time = SystemTime::now();
                notifier.Show(&toast).map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to show toast notification: {:?}", e),
                    }
                })?;

                // Create delivery receipt using the builder pattern
                let receipt = DeliveryReceipt::new(Platform::Windows, request.notification_id.clone())
                    .with_metadata("platform_api".to_string(), "WinRT".to_string())
                    .with_metadata("toast_template".to_string(), "ToastGeneric".to_string())
                    .with_metadata("app_id".to_string(), self.app_id.clone());

                Ok(receipt)
            }

            #[cfg(not(target_os = "windows"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Windows".to_string(),
                    error_code: None,
                    message: "Windows backend not available on this platform".to_string(),
                })
            }
        })
    }

    #[allow(unused_variables)]
    fn update_notification(
        &self,
        id: &str,
        update: &NotificationUpdate,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                // Windows toasts can't be updated directly - need to remove and recreate
                // This is a platform limitation, so we implement the best possible solution

                // First, try to remove the existing notification
                let _ = self.cancel_notification(id).await;

                // Build updated content from content_changes map
                let title = update.content_changes.get("title")
                    .cloned()
                    .unwrap_or_else(|| "Updated Notification".to_string());
                let body = update.content_changes.get("body")
                    .cloned()
                    .unwrap_or_default();

                // Create XML for updated toast
                let xml_doc = self.create_toast_xml(&title, None, &body, None, None)
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create updated toast XML: {:?}", e),
                    })?;

                // Create and show the updated toast
                let notifier = self.get_notifier().await?;
                let toast = ToastNotification::CreateToastNotification(&xml_doc)
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create updated toast notification: {:?}", e),
                    })?;

                notifier.Show(&toast)
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to show updated toast notification: {:?}", e),
                    })?;

                Ok(())
            }

            #[cfg(not(target_os = "windows"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Windows".to_string(),
                    error_code: None,
                    message: "Windows backend not available on this platform".to_string(),
                })
            }
        })
    }

    #[allow(unused_variables)]
    fn cancel_notification(
        &self,
        id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                // Windows Toast API limitation: no direct cancellation by ID
                // Best effort approach: use ToastNotificationHistory to remove

                let app_id = HSTRING::from(&self.app_id);
                let history = ToastNotificationManager::History();

                // Remove from history (this removes delivered notifications from Action Center)
                let notification_id = HSTRING::from(id);
                history
                    .RemoveWithTagAndGroup(&notification_id, &HSTRING::from(""), &app_id)
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to remove notification from history: {:?}", e),
                    })?;

                // Note: This only removes already-delivered notifications from Action Center
                // Pending notifications cannot be cancelled due to Windows API limitations
                Ok(())
            }

            #[cfg(not(target_os = "windows"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Windows".to_string(),
                    error_code: None,
                    message: "Windows backend not available on this platform".to_string(),
                })
            }
        })
    }
}

// Add Clone implementation for WindowsBackend
impl Clone for WindowsBackend {
    fn clone(&self) -> Self {
        Self {
            #[cfg(target_os = "windows")]
            app_id: self.app_id.clone(),
            #[cfg(target_os = "windows")]
            notifier: Arc::clone(&self.notifier), // Clone the Arc, share the OnceCell
        }
    }
}
