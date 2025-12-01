// macOS UserNotifications backend - Complete implementation

use std::collections::HashMap;
use std::time::SystemTime;

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;
#[cfg(target_os = "macos")]
use objc2_user_notifications::{
    UNAuthorizationStatus, UNMutableNotificationContent, UNNotificationRequest,
    UNTimeIntervalNotificationTrigger, UNUserNotificationCenter,
};

use crate::components::NotificationResult;
use crate::components::platform::{
    DeliveryReceipt, NotificationRequest, NotificationUpdate, Platform, PlatformBackend,
    PlatformCapabilities,
};

pub struct MacOSBackend {
    // No stored fields needed - we'll get the notification center fresh each time
}

impl Default for MacOSBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MacOSBackend {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(target_os = "macos")]
    fn get_notification_center() -> Retained<UNUserNotificationCenter> {
        UNUserNotificationCenter::currentNotificationCenter()
    }
}

impl MacOSBackend {
    pub async fn check_authorization(&self) -> NotificationResult<bool> {
        #[cfg(target_os = "macos")]
        {
            use std::sync::{Arc, Mutex};

            use tokio::sync::oneshot;

            let (tx, rx) = oneshot::channel();
            let tx = Arc::new(Mutex::new(Some(tx)));

            // Use Bevy's AsyncComputeTaskPool for consistency with ECS patterns
            let task_pool = bevy::tasks::AsyncComputeTaskPool::get();
            let _task_handle = task_pool.spawn(async move {
                // objc2 operations are sync but wrapped in async for Bevy integration
                let center = UNUserNotificationCenter::currentNotificationCenter();

                // Create a completion handler block that captures authorization status
                let block_tx = Arc::clone(&tx);
                let block = block2::StackBlock::new(
                    move |settings: std::ptr::NonNull<
                        objc2_user_notifications::UNNotificationSettings,
                    >| {
                        let auth_status = unsafe { settings.as_ref() }.authorizationStatus();
                        let is_authorized = matches!(
                            auth_status,
                            UNAuthorizationStatus::Authorized | UNAuthorizationStatus::Provisional
                        );

                        // Send result through channel
                        if let Ok(mut sender_guard) = block_tx.lock()
                            && let Some(sender) = sender_guard.take() {
                                let _ = sender.send(is_authorized);
                            }
                    },
                );

                // Get notification settings with real completion handler
                let block = block.copy();
                center.getNotificationSettingsWithCompletionHandler(&block);
                
            });

            // Wait for the async callback with timeout
            match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
                Ok(Ok(is_authorized)) => Ok(is_authorized),
                Ok(Err(_)) => Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "Authorization check callback failed".to_string(),
                }),
                Err(_) => Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "Authorization check timeout".to_string(),
                }),
            }
        }
        #[cfg(not(target_os = "macos"))]
        Ok(false)
    }
}

impl PlatformBackend for MacOSBackend {
    fn negotiate_capabilities(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<PlatformCapabilities>> + Send + '_>,
    > {
        Box::pin(async move {
            #[cfg(target_os = "macos")]
            {
                // Check actual authorization status
                let is_authorized: bool = (self.check_authorization().await).unwrap_or_default();

                let authorization_required = !is_authorized;

                Ok(PlatformCapabilities {
                    supports_actions: true,
                    supports_rich_media: true,
                    supports_markup: false,
                    supports_sound: true,
                    supports_scheduling: true,
                    supports_progress: false,
                    supports_categories: true,
                    supports_replies: true,
                    supports_custom_ui: false,
                    supports_background_activation: true,
                    supports_update_content: true,
                    supports_persistent: true,
                    supports_priority: true,
                    supports_grouping: true,
                    supports_badges: true,
                    supports_vibration: false,
                    max_actions: Some(4),
                    max_title_length: Some(256),
                    max_body_length: Some(2048),
                    max_image_size: Some(10_485_760), // 10MB
                    max_sound_duration: Some(std::time::Duration::from_secs(30)),
                    platform_features: HashMap::new(),
                    platform_limits: HashMap::new(),
                    authorization_required,
                    permission_levels: vec![
                        crate::components::platform::PermissionLevel::Display,
                        crate::components::platform::PermissionLevel::Sound,
                        crate::components::platform::PermissionLevel::Badge,
                        crate::components::platform::PermissionLevel::Actions,
                        crate::components::platform::PermissionLevel::Critical,
                    ],
                    platform_version: Some("macOS".to_string()),
                    api_version: Some("UserNotifications".to_string()),
                    compatibility_level: crate::components::platform::CompatibilityLevel::Full,
                    delivery_latency_estimate: Some(std::time::Duration::from_millis(100)),
                    supports_batching: false,
                    rate_limits: None,
                })
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "macOS backend not available on this platform".to_string(),
                })
            }
        })
    }

    fn deliver_notification(
        &self,
        request: &NotificationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<DeliveryReceipt>> + Send + '_>,
    > {
        let request = request.clone();
        Box::pin(async move {
            #[cfg(target_os = "macos")]
            {
                // Check authorization before attempting to deliver
                let is_authorized = self.check_authorization().await?;
                if !is_authorized {
                    return Err(crate::components::NotificationError::AuthorizationError {
                        platform: "macOS".to_string(),
                        required_permission: "notification_display".to_string(),
                    });
                }

                // Schedule notification using Bevy's AsyncComputeTaskPool for consistency
                // This provides proper error handling while maintaining Bevy ECS patterns
                let notification_id = request.notification_id.clone();
                let request_title = request.content.title.clone();
                let request_body = request.content.body.to_plain_text();

                // Create a oneshot channel for the completion handler
                use std::sync::{Arc, Mutex};

                use tokio::sync::oneshot;

                let (completion_tx, completion_rx) = oneshot::channel();
                let completion_tx = Arc::new(Mutex::new(Some(completion_tx)));

                // Set up the notification using Bevy's AsyncComputeTaskPool
                let task_pool = bevy::tasks::AsyncComputeTaskPool::get();
                let result = task_pool
                    .spawn(async move {
                        let center =
                            UNUserNotificationCenter::currentNotificationCenter();

                        // Create notification content using Bevy async task pool
                        let content = UNMutableNotificationContent::new();
                        let title_ns = NSString::from_str(&request_title);
                        let body_ns = NSString::from_str(&request_body);

                        content.setTitle(&title_ns);
                            content.setBody(&body_ns);
                        

                        // Set default sound
                        let default_sound = objc2_user_notifications::UNNotificationSound::defaultSound()
                        ;
                        content.setSound(Some(&default_sound));
                        

                        // Create immediate trigger
                        let trigger = UNTimeIntervalNotificationTrigger::triggerWithTimeInterval_repeats(
                                0.1, false,
                            )
                        ;

                        // Create request
                        let identifier = NSString::from_str(&notification_id);
                        let notification_request = UNNotificationRequest::requestWithIdentifier_content_trigger(
                                &identifier,
                                &content,
                                Some(&trigger),
                            )
                        ;

                        // Create completion handler block
                        let completion_block = block2::StackBlock::new(
                            move |error: *mut objc2_foundation::NSError| {
                                let success = error.is_null();
                                if let Ok(mut sender_guard) = completion_tx.lock()
                                    && let Some(sender) = sender_guard.take() {
                                        let _ = sender.send(success);
                                    }
                            },
                        );

                        // Schedule the notification
                        let completion_block = completion_block.copy();
                        center.addNotificationRequest_withCompletionHandler(
                                &notification_request,
                                Some(&completion_block),
                            );
                        

                        // Return the notification ID for success case
                        Ok::<String, String>(notification_id.clone())
                    })
                    .await;

                // Handle the AsyncComputeTaskPool result
                let notification_id = match result {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(crate::components::NotificationError::PlatformError {
                            platform: "macOS".to_string(),
                            error_code: None,
                            message: format!("Notification setup failed: {}", e),
                        });
                    },
                };

                // Wait for completion with timeout (using Bevy async patterns)
                let delivery_success =
                    match tokio::time::timeout(std::time::Duration::from_secs(3), completion_rx)
                        .await
                    {
                        Ok(Ok(true)) => true,
                        Ok(Ok(false)) => {
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: None,
                                message: "Notification delivery failed".to_string(),
                            });
                        },
                        Ok(Err(_)) => {
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: None,
                                message: "Completion callback failed".to_string(),
                            });
                        },
                        Err(_) => {
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: None,
                                message: "Notification delivery timeout".to_string(),
                            });
                        },
                    };

                // If we get here, delivery was successful
                if delivery_success {
                    // Success - create delivery receipt
                    let mut metadata = HashMap::new();
                    metadata.insert("platform_api".to_string(), "UserNotifications".to_string());
                    metadata.insert("authorization_status".to_string(), "granted".to_string());
                    metadata.insert(
                        "delivery_method".to_string(),
                        "bevy_async_task_pool".to_string(),
                    );

                    Ok(DeliveryReceipt {
                        platform: Platform::MacOS,
                        native_id: notification_id,
                        delivered_at: SystemTime::now(),
                        metadata,
                    })
                } else {
                    // This should never be reached due to earlier error handling
                    Err(crate::components::NotificationError::PlatformError {
                        platform: "macOS".to_string(),
                        error_code: None,
                        message: "Unexpected delivery failure".to_string(),
                    })
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "macOS backend not available on this platform".to_string(),
                })
            }
        })
    }

    fn update_notification(
        &self,
        id: &str,
        update: &NotificationUpdate,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        let id = id.to_string();
        let update = update.clone();
        Box::pin(async move {
            #[cfg(target_os = "macos")]
            {
                // Move all objc2 work inside Bevy async task pool
                let update_id = id.to_string();
                let content_changes = update.content_changes.clone();

                let task_pool = bevy::tasks::AsyncComputeTaskPool::get();
                

                // AsyncComputeTaskPool returns the result directly
                task_pool
                    .spawn(async move {
                        let center =
                            UNUserNotificationCenter::currentNotificationCenter();

                        // On macOS, we need to remove and re-add to update
                        let identifier = NSString::from_str(&update_id);

                        // Convert to NSArray for the API call
                        use objc2_foundation::NSArray;
                        let identifiers_vec = vec![&*identifier];
                        let identifiers_array = NSArray::from_slice(&identifiers_vec);

                        center.removePendingNotificationRequestsWithIdentifiers(
                                &identifiers_array,
                            );
                            center.removeDeliveredNotificationsWithIdentifiers(&identifiers_array);
                        

                        // If update contains new content, create and schedule new notification
                        if !content_changes.is_empty() {
                            let new_content = UNMutableNotificationContent::new();

                            // Use content_changes fields since update.content doesn't exist
                            if let Some(title) = content_changes.get("title") {
                                let title_ns = NSString::from_str(title);
                                new_content.setTitle(&title_ns);
                                
                            }

                            if let Some(body) = content_changes.get("body") {
                                let body_ns = NSString::from_str(body);
                                new_content.setBody(&body_ns);
                                
                            }

                            let trigger = UNTimeIntervalNotificationTrigger::triggerWithTimeInterval_repeats(
                                    0.1, false,
                                )
                            ;

                            let new_identifier = NSString::from_str(&update_id);
                            let notification_request = UNNotificationRequest::requestWithIdentifier_content_trigger(
                                    &new_identifier,
                                    &new_content,
                                    Some(&trigger),
                                )
                            ;

                            center.addNotificationRequest_withCompletionHandler(
                                    &notification_request,
                                    None,
                                );
                            
                        }

                        // Return success result
                        Ok::<(), crate::components::NotificationError>(())
                    })
                    .await
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "macOS backend not available on this platform".to_string(),
                })
            }
        })
    }

    fn cancel_notification(
        &self,
        id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        let id = id.to_string();
        Box::pin(async move {
            #[cfg(target_os = "macos")]
            {
                let center = Self::get_notification_center();
                let identifier = NSString::from_str(&id);

                // Convert to NSArray for the API call
                use objc2_foundation::NSArray;
                let identifiers_vec = vec![&*identifier];
                let identifiers_array = NSArray::from_slice(&identifiers_vec);

                center.removePendingNotificationRequestsWithIdentifiers(&identifiers_array);
                    center.removeDeliveredNotificationsWithIdentifiers(&identifiers_array);
                

                Ok(())
            }

            #[cfg(not(target_os = "macos"))]
            {
                Err(crate::components::NotificationError::PlatformError {
                    platform: "macOS".to_string(),
                    error_code: None,
                    message: "macOS backend not available on this platform".to_string(),
                })
            }
        })
    }
}
