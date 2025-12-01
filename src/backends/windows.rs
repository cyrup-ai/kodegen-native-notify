// Windows Toast Notifications backend - Complete implementation

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

pub struct WindowsBackend {
    #[cfg(target_os = "windows")]
    app_id: String,
    #[cfg(target_os = "windows")]
    notifier: Arc<Mutex<Option<ToastNotifier>>>,
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
            notifier: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_notifier(&self) -> Result<ToastNotifier, crate::components::NotificationError> {
        let mut notifier_guard = self.notifier.lock().map_err(|_| {
            crate::components::NotificationError::PlatformError {
                platform: "Windows".to_string(),
                error_code: None,
                message: "Failed to acquire notifier lock".to_string(),
            }
        })?;

        if notifier_guard.is_none() {
            let app_id = HSTRING::from(&self.app_id);
            let notifier =
                ToastNotificationManager::CreateToastNotifierWithId(&app_id).map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Windows".to_string(),
                        error_code: Some(e.code().0 as i32),
                        message: format!("Failed to create toast notifier: {:?}", e),
                    }
                })?;
            *notifier_guard = Some(notifier.clone());
            Ok(notifier)
        } else {
            // Safe to unwrap here as we know it's Some from the preceding check,
            // but use unwrap_or_else with clear message for better debugging
            Ok(notifier_guard
                .as_ref()
                .unwrap_or_else(|| panic!("Critical error: notifier should exist after preceding None check - this indicates a race condition or programming error"))
                .clone())
        }
    }

    #[cfg(target_os = "windows")]
    fn create_toast_xml(&self, title: &str, body: &str) -> WindowsResult<XmlDocument> {
        // Escape XML content to prevent injection
        let escaped_title = title
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;");
        let escaped_body = body
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;");

        let toast_xml = format!(
            r#"
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>{}</text>
            <text>{}</text>
        </binding>
    </visual>
    <actions>
        <input id="textBox" type="text" placeHolderContent="Type something..." />
        <action activationType="background" content="Reply" arguments="reply" />
        <action activationType="background" content="Dismiss" arguments="dismiss" />
    </actions>
</toast>
"#,
            escaped_title, escaped_body
        );

        let xml_doc = XmlDocument::new()?;
        let xml_hstring = HSTRING::from(&toast_xml);
        xml_doc.LoadXml(&xml_hstring)?;
        Ok(xml_doc)
    }
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

    fn deliver_notification(
        &self,
        _request: &NotificationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<DeliveryReceipt>> + Send + '_>,
    > {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                let notifier = self.get_notifier().await?;

                // Create XML content for the toast
                let xml_doc = self
                    .create_toast_xml(
                        &request.content.title,
                        &request.content.body.as_plain_text(),
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

                // Create delivery receipt
                let mut metadata = HashMap::new();
                metadata.insert("platform_api".to_string(), "WinRT".to_string());
                metadata.insert("toast_template".to_string(), "ToastGeneric".to_string());
                metadata.insert("app_id".to_string(), self.app_id.clone());

                Ok(DeliveryReceipt {
                    platform: Platform::Windows,
                    native_id: request.notification_id.clone(),
                    delivered_at: SystemTime::now(),
                    metadata,
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

    fn update_notification(
        &self,
        _id: &str,
        _update: &NotificationUpdate,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            #[cfg(target_os = "windows")]
            {
                // Windows toasts can't be updated directly - need to remove and recreate
                // This is a platform limitation, so we implement the best possible solution

                // First, try to remove the existing notification
                let _ = self.cancel_notification(id).await;

                // Create new notification with updated content
                let updated_request = NotificationRequest {
                    notification_id: id.to_string(),
                    content: update.content.clone(),
                    options: update.options.clone().unwrap_or_default(),
                    correlation_id: format!("update-{}", id),
                };

                // Deliver the updated notification
                self.deliver_notification(&updated_request).await?;

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

    fn cancel_notification(
        &self,
        _id: &str,
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
            notifier: Arc::clone(&self.notifier), // Clone the Arc, share the Mutex
        }
    }
}
