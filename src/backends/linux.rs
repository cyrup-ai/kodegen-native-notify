// Linux D-Bus Notifications backend - Complete implementation

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use zbus::{Connection, Result as ZbusResult, dbus_proxy};

use crate::components::NotificationResult;
use crate::components::platform::{
    DeliveryReceipt, NotificationRequest, NotificationUpdate, PlatformBackend, PlatformCapabilities,
};

#[cfg(target_os = "linux")]
#[dbus_proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    /// Send a notification to the desktop notification daemon
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<&str>,
        hints: std::collections::HashMap<&str, zbus::zvariant::Value>,
        expire_timeout: i32,
    ) -> ZbusResult<u32>;

    /// Get the capabilities supported by the notification server
    fn get_capabilities(&self) -> ZbusResult<Vec<String>>;

    /// Get information about the notification server
    fn get_server_information(&self) -> ZbusResult<(String, String, String, String)>;

    /// Close a notification
    fn close_notification(&self, id: u32) -> ZbusResult<()>;
}

pub struct LinuxBackend {
    #[cfg(target_os = "linux")]
    connection: Arc<Mutex<Option<Connection>>>,
    #[cfg(target_os = "linux")]
    capabilities: Arc<Mutex<Option<Vec<String>>>>,
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            connection: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            capabilities: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn check_authorization(&self) -> NotificationResult<bool> {
        #[cfg(target_os = "linux")]
        {
            // Linux D-Bus notifications don't require explicit authorization
            // They work by default unless disabled in system settings
            // We can check if D-Bus service is available as a proxy for authorization
            match self.get_connection().await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false), // Service not available, treat as unauthorized
            }
        }
        #[cfg(not(target_os = "linux"))]
        Ok(false)
    }

    #[cfg(target_os = "linux")]
    async fn get_connection(&self) -> Result<Connection, crate::components::NotificationError> {
        let mut conn_guard = self.connection.lock().map_err(|_| {
            crate::components::NotificationError::PlatformError {
                platform: "Linux".to_string(),
                error_code: None,
                message: "Failed to acquire connection lock".to_string(),
            }
        })?;

        if conn_guard.is_none() {
            let connection = Connection::session().await.map_err(|e| {
                crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: format!("Failed to connect to D-Bus session: {:?}", e),
                }
            })?;
            *conn_guard = Some(connection.clone());
            Ok(connection)
        } else {
            // Safe to unwrap here as we know it's Some from the preceding check,
            // but use unwrap_or_else with clear message for better debugging
            Ok(conn_guard
                .as_ref()
                .unwrap_or_else(|| panic!("Critical error: connection should exist after preceding None check - this indicates a race condition or programming error"))
                .clone())
        }
    }

    #[cfg(target_os = "linux")]
    async fn get_capabilities(&self) -> Result<Vec<String>, crate::components::NotificationError> {
        let mut caps_guard = self.capabilities.lock().map_err(|_| {
            crate::components::NotificationError::PlatformError {
                platform: "Linux".to_string(),
                error_code: None,
                message: "Failed to acquire capabilities lock".to_string(),
            }
        })?;

        if caps_guard.is_none() {
            let connection = self.get_connection().await?;
            let proxy = NotificationsProxy::new(&connection).await.map_err(|e| {
                crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: format!("Failed to create D-Bus proxy: {:?}", e),
                }
            })?;
            let caps = proxy.get_capabilities().await.map_err(|e| {
                crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: format!("Failed to get capabilities: {:?}", e),
                }
            })?;
            *caps_guard = Some(caps.clone());
            Ok(caps)
        } else {
            // Safe to unwrap here as we know it's Some from the preceding check,
            // but use unwrap_or_else with clear message for better debugging
            Ok(caps_guard
                .as_ref()
                .unwrap_or_else(|| panic!("Critical error: capabilities should exist after preceding None check - this indicates a race condition or programming error"))
                .clone())
        }
    }

    #[cfg(target_os = "linux")]
    fn create_hints(
        &self,
        request: &NotificationRequest,
    ) -> std::collections::HashMap<&str, zbus::zvariant::Value> {
        let mut hints = std::collections::HashMap::new();

        // Set urgency level based on priority
        let urgency = match request.content.priority {
            crate::components::Priority::Low => 0u8,
            crate::components::Priority::Normal => 1u8,
            crate::components::Priority::High => 2u8,
            crate::components::Priority::Critical | crate::components::Priority::Urgent => 2u8,
        };
        hints.insert("urgency", zbus::zvariant::Value::U8(urgency));

        // Set category if available
        if let Some(category) = &request.content.category {
            hints.insert(
                "category",
                zbus::zvariant::Value::Str(category.identifier.as_str().into()),
            );
        }

        // Set desktop entry if available
        hints.insert(
            "desktop-entry",
            zbus::zvariant::Value::Str("ecs-notifications".into()),
        );

        hints
    }
}

impl PlatformBackend for LinuxBackend {
    fn negotiate_capabilities(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<PlatformCapabilities>> + Send + '_>,
    > {
        Box::pin(async move {
            #[cfg(target_os = "linux")]
            {
                // Try to connect to the D-Bus notification service
                let connection_result = self.get_connection().await;
                if connection_result.is_err() {
                    return Ok(PlatformCapabilities {
                        compatibility_level: CompatibilityLevel::None,
                        ..PlatformCapabilities::default()
                    });
                }

                // Get server capabilities
                let capabilities = self.get_capabilities().await?;

                // Parse capabilities into platform features
                let supports_actions = capabilities.contains(&"actions".to_string());
                let supports_markup = capabilities.contains(&"body-markup".to_string())
                    || capabilities.contains(&"markup".to_string());
                let supports_images = capabilities.contains(&"body-images".to_string());
                let supports_sound = capabilities.contains(&"sound".to_string());
                let supports_persistence = capabilities.contains(&"persistence".to_string());

                // Get server information for version details
                let connection = self.get_connection().await?;
                let proxy = NotificationsProxy::new(&connection).await.map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to create D-Bus proxy: {:?}", e),
                    }
                })?;

                let (server_name, vendor, version, spec_version) =
                    proxy.get_server_information().await.unwrap_or_else(|_| {
                        (
                            "Unknown".to_string(),
                            "Unknown".to_string(),
                            "Unknown".to_string(),
                            "1.2".to_string(),
                        )
                    });

                Ok(PlatformCapabilities {
                    supports_actions,
                    supports_rich_media: supports_images,
                    supports_markup,
                    supports_sound,
                    supports_scheduling: false, /* D-Bus notifications don't support native
                                                 * scheduling */
                    supports_progress: false, // Basic D-Bus spec doesn't support progress
                    supports_categories: true, // Through hints
                    supports_replies: false,  // Not in basic spec
                    supports_custom_ui: false,
                    supports_background_activation: supports_actions,
                    supports_update_content: true, // Through replaces_id
                    supports_persistent: supports_persistence,
                    supports_priority: true, // Through urgency hints
                    supports_grouping: false,
                    supports_badges: false,
                    supports_vibration: false,
                    max_actions: None,               // Server dependent
                    max_title_length: Some(512),     // Conservative estimate
                    max_body_length: Some(4096),     // Conservative estimate
                    max_image_size: Some(1_048_576), // 1MB conservative estimate
                    max_sound_duration: None,
                    platform_features: {
                        let mut features = std::collections::HashMap::new();
                        for cap in capabilities.iter() {
                            features.insert(cap.clone(), true);
                        }
                        features
                    },
                    platform_limits: std::collections::HashMap::new(),
                    authorization_required: false, /* D-Bus notifications don't require
                                                    * authorization */
                    permission_levels: vec![PermissionLevel::Display],
                    platform_version: Some(format!("{} {} by {}", server_name, version, vendor)),
                    api_version: Some(format!("D-Bus Notifications Spec {}", spec_version)),
                    compatibility_level: if supports_actions {
                        CompatibilityLevel::High
                    } else {
                        CompatibilityLevel::Medium
                    },
                    delivery_latency_estimate: Some(std::time::Duration::from_millis(20)),
                    supports_batching: false,
                    rate_limits: None,
                })
            }

            #[cfg(not(target_os = "linux"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: "Linux backend not available on this platform".to_string(),
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
            #[cfg(target_os = "linux")]
            {
                let connection = self.get_connection().await?;

                let proxy = NotificationsProxy::new(&connection).await.map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to create D-Bus proxy: {:?}", e),
                    }
                })?;

                // Create hints for the notification
                let hints = self.create_hints(request);

                // Convert actions to D-Bus format
                let mut actions = Vec::new();
                for action in &request.content.interactions.actions {
                    actions.push(action.id.as_str());
                    actions.push(&action.title);
                }

                // Set expire timeout based on priority
                let expire_timeout = match request.options.ttl {
                    Some(ttl) => ttl.as_millis() as i32,
                    None => match request.content.priority {
                        crate::components::Priority::Critical
                        | crate::components::Priority::Urgent => 0, // Never expire
                        crate::components::Priority::High => 10000, // 10 seconds
                        crate::components::Priority::Normal => 5000, // 5 seconds
                        crate::components::Priority::Low => 3000,   // 3 seconds
                    },
                };

                // Send the notification
                let start_time = SystemTime::now();
                let notification_id = proxy
                    .notify(
                        "ECS Notifications",
                        0,  // replaces_id - 0 for new notification
                        "", // app_icon - empty uses default
                        &request.content.title,
                        &request.content.body.to_plain_text(),
                        actions.iter().map(|s| *s).collect(),
                        hints,
                        expire_timeout,
                    )
                    .await
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to send D-Bus notification: {:?}", e),
                    })?;

                // Create delivery receipt
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("platform_api".to_string(), "D-Bus".to_string());
                metadata.insert(
                    "dbus_service".to_string(),
                    "org.freedesktop.Notifications".to_string(),
                );
                metadata.insert("notification_id".to_string(), notification_id.to_string());

                Ok(DeliveryReceipt {
                    platform: Platform::Linux,
                    native_id: notification_id.to_string(),
                    delivered_at: SystemTime::now(),
                    metadata,
                })
            }

            #[cfg(not(target_os = "linux"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: "Linux backend not available on this platform".to_string(),
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
            #[cfg(target_os = "linux")]
            {
                let connection = self.get_connection().await?;

                let proxy = NotificationsProxy::new(&connection).await.map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to create D-Bus proxy: {:?}", e),
                    }
                })?;

                // Parse the notification ID to use as replaces_id
                let replaces_id: u32 = id.parse().map_err(|e| {
                    crate::components::NotificationError::ValidationError {
                        field: "notification_id".to_string(),
                        message: format!("Invalid notification ID format: {}", e),
                    }
                })?;

                // Create updated notification request
                let updated_request = NotificationRequest {
                    notification_id: id.to_string(),
                    content: update.content.clone(),
                    options: update.options.clone().unwrap_or_default(),
                    correlation_id: format!("update-{}", id),
                };

                // Create hints for the updated notification
                let hints = self.create_hints(&updated_request);

                // Convert actions to D-Bus format
                let mut actions = Vec::new();
                for action in &updated_request.content.interactions.actions {
                    actions.push(action.id.as_str());
                    actions.push(&action.title);
                }

                // Set expire timeout based on priority
                let expire_timeout = match updated_request.options.ttl {
                    Some(ttl) => ttl.as_millis() as i32,
                    None => match updated_request.content.priority {
                        crate::components::Priority::Critical
                        | crate::components::Priority::Urgent => 0,
                        crate::components::Priority::High => 10000,
                        crate::components::Priority::Normal => 5000,
                        crate::components::Priority::Low => 3000,
                    },
                };

                // Send the updated notification with replaces_id
                let _new_id = proxy
                    .notify(
                        "ECS Notifications",
                        replaces_id, // This replaces the existing notification
                        "",          // app_icon
                        &updated_request.content.title,
                        &updated_request.content.body.to_plain_text(),
                        actions.iter().map(|s| *s).collect(),
                        hints,
                        expire_timeout,
                    )
                    .await
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to update D-Bus notification: {:?}", e),
                    })?;

                Ok(())
            }

            #[cfg(not(target_os = "linux"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: "Linux backend not available on this platform".to_string(),
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
            #[cfg(target_os = "linux")]
            {
                let connection = self.get_connection().await?;

                let proxy = NotificationsProxy::new(&connection).await.map_err(|e| {
                    crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to create D-Bus proxy: {:?}", e),
                    }
                })?;

                // Parse the notification ID
                let notification_id: u32 = id.parse().map_err(|e| {
                    crate::components::NotificationError::ValidationError {
                        field: "notification_id".to_string(),
                        message: format!("Invalid notification ID format: {}", e),
                    }
                })?;

                // Close the notification
                proxy
                    .close_notification(notification_id)
                    .await
                    .map_err(|e| crate::components::NotificationError::PlatformError {
                        platform: "Linux".to_string(),
                        error_code: None,
                        message: format!("Failed to close D-Bus notification: {:?}", e),
                    })?;

                Ok(())
            }

            #[cfg(not(target_os = "linux"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "Linux".to_string(),
                    error_code: None,
                    message: "Linux backend not available on this platform".to_string(),
                })
            }
        })
    }
}

// Add Clone implementation for LinuxBackend
impl Clone for LinuxBackend {
    fn clone(&self) -> Self {
        Self {
            #[cfg(target_os = "linux")]
            connection: Arc::clone(&self.connection), // Clone the Arc, share the Mutex
            #[cfg(target_os = "linux")]
            capabilities: Arc::clone(&self.capabilities), // Clone the Arc, share the Mutex
        }
    }
}
