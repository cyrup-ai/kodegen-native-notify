// macOS UserNotifications backend - Complete implementation

use std::collections::HashMap;


#[cfg(target_os = "macos")]
use block2::RcBlock;
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

// =============================================================================
// Block Helper Functions
// =============================================================================

/// Converts a `FnOnce` closure into a `Fn` closure suitable for block2.
///
/// This is the idiomatic pattern for completion handlers that should only be called once.
/// The closure is wrapped in `Cell<Option<...>>` so it can be taken and consumed on first call.
///
/// # Panics
/// Panics if the returned closure is called more than once.
///
/// # Reference
/// - [block2 documentation](https://docs.rs/block2/latest/block2/#creating-blocks)
#[cfg(target_os = "macos")]
#[allow(dead_code)] // Helper provided for no-arg completion handlers; fnonce_to_fn1 is used instead
fn fnonce_to_fn<F, R>(closure: F) -> impl Fn() -> R
where
    F: FnOnce() -> R,
{
    use std::cell::Cell;
    let cell = Cell::new(Some(closure));
    move || {
        let closure = cell.take().expect("completion handler called more than once");
        closure()
    }
}

/// Variant for completion handlers with one argument.
#[cfg(target_os = "macos")]
fn fnonce_to_fn1<F, A, R>(closure: F) -> impl Fn(A) -> R
where
    F: FnOnce(A) -> R,
{
    use std::cell::Cell;
    let cell = Cell::new(Some(closure));
    move |arg| {
        let closure = cell.take().expect("completion handler called more than once");
        closure(arg)
    }
}

// =============================================================================
// Compile-Time Thread-Safety Assertions
// =============================================================================

/// Compile-time verification that types used in completion handler captures are thread-safe.
///
/// # Why This Matters
/// Apple's completion handlers are invoked on arbitrary threads. The `block2` crate's
/// `Block`, `RcBlock`, and `StackBlock` types are `!Send + !Sync` as a conservative default
/// (see [GitHub Issue #572](https://github.com/madsmtm/objc2/issues/572)).
///
/// However, the code is sound if ALL captured variables are `Send + Sync`. These static
/// assertions verify this at compile time, preventing accidental introduction of non-thread-safe
/// captures.
#[cfg(target_os = "macos")]
const _: () = {
    #[allow(dead_code)] // Used for compile-time verification only
    fn assert_send<T: Send>() {}
    #[allow(dead_code)] // Used for compile-time verification only
    fn assert_sync<T: Sync>() {}

    // Verify oneshot::Sender is Send (it is, as of tokio 1.x)
    fn _assert_sender_send() {
        assert_send::<tokio::sync::oneshot::Sender<bool>>();
    }

    // Verify the full captured type is Send + Sync
    fn _assert_captured_type() {
        assert_send::<std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>>();
        assert_sync::<std::sync::Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>>();
    }
};

// =============================================================================
// MacOS Backend Implementation
// =============================================================================

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

    /// Cancel a notification by removing it from both pending and delivered queues
    /// This ensures no ghost notifications appear after timeout or failure
    #[cfg(target_os = "macos")]
    fn cancel_notification_sync(notification_id: &str) {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let identifier = NSString::from_str(notification_id);

        // Convert to NSArray for the API call
        use objc2_foundation::NSArray;
        let identifiers_vec = vec![&*identifier];
        let identifiers_array = NSArray::from_slice(&identifiers_vec);

        // Remove from both pending and delivered to ensure complete cleanup
        center.removePendingNotificationRequestsWithIdentifiers(&identifiers_array);
        center.removeDeliveredNotificationsWithIdentifiers(&identifiers_array);
    }

    /// Create image attachment from a local file path
    #[cfg(target_os = "macos")]
    fn create_image_attachment_from_path(
        file_path: &std::path::Path,
        identifier: &str,
    ) -> Option<Retained<objc2_user_notifications::UNNotificationAttachment>> {
        use objc2_foundation::NSURL;
        use objc2_user_notifications::UNNotificationAttachment;

        if !file_path.exists() {
            tracing::warn!("Image file does not exist: {:?}", file_path);
            return None;
        }

        let path_str = file_path.to_string_lossy();
        let ns_url = NSURL::fileURLWithPath(&NSString::from_str(&path_str));
        let ns_identifier = NSString::from_str(identifier);

        // objc2-user-notifications returns Result with error, not via pointer
        match unsafe {
            UNNotificationAttachment::attachmentWithIdentifier_URL_options_error(
                &ns_identifier,
                &ns_url,
                None,
            )
        } {
            Ok(attachment) => Some(attachment),
            Err(e) => {
                tracing::warn!("Failed to create notification attachment: {:?}", e);
                None
            }
        }
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

            // No spawn needed - these are quick synchronous calls
            // Scope the block so it's dropped before await (block is not Send)
            {
                let center = UNUserNotificationCenter::currentNotificationCenter();

                // Create a completion handler block that captures authorization status
                let block_tx = Arc::clone(&tx);

                // Create completion handler using RcBlock directly (avoids intermediate StackBlock allocation)
                //
                // SAFETY: Thread-safety analysis for completion handler captures:
                // - `block_tx: Arc<Mutex<Option<oneshot::Sender<bool>>>>` is `Send + Sync`
                // - Apple's `getNotificationSettingsWithCompletionHandler:` invokes the block on an
                //   arbitrary thread (typically com.apple.usernotifications.* queue)
                // - The `block2` types are `!Send + !Sync` as a conservative default, but the code is
                //   sound because all captured variables are verified `Send + Sync` at compile time
                //   (see static assertions above)
                // - The block is heap-allocated via `RcBlock::new()` and Apple retains it for the
                //   callback duration via `_Block_copy` semantics
                // - Reference: https://github.com/madsmtm/objc2/issues/572 (tracking Send+Sync blocks)
                let block = RcBlock::new(fnonce_to_fn1(
                    move |settings: std::ptr::NonNull<
                        objc2_user_notifications::UNNotificationSettings,
                    >| {
                        // SAFETY: Apple guarantees valid pointer in completion handler.
                        // NonNull<T> guarantees the pointer is non-null by construction.
                        // No runtime check needed - NonNull is a compile-time guarantee.
                        let settings_ref = unsafe { settings.as_ref() };
                        let auth_status = settings_ref.authorizationStatus();
                        let is_authorized = matches!(
                            auth_status,
                            UNAuthorizationStatus::Authorized | UNAuthorizationStatus::Provisional
                        );

                        // Send result through channel, handling mutex poisoning gracefully
                        match block_tx.lock() {
                            Ok(mut sender_guard) => {
                                if let Some(sender) = sender_guard.take() {
                                    let _ = sender.send(is_authorized);
                                }
                            }
                            Err(poisoned) => {
                                // Mutex was poisoned by a panic in another thread - still try to send
                                // to avoid blocking the caller indefinitely
                                if let Some(sender) = poisoned.into_inner().take() {
                                    let _ = sender.send(false); // Conservative default on poison
                                }
                            }
                        }
                    },
                ));

                center.getNotificationSettingsWithCompletionHandler(&block);
            } // block is dropped here, before the await

            // Wait for the async callback with timeout
            match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
                Ok(Ok(is_authorized)) => Ok(is_authorized),
                Ok(Err(_)) => {
                    // Channel dropped - callback never fired or panicked
                    Err(crate::components::NotificationError::PlatformError {
                        platform: "macOS".to_string(),
                        error_code: None,
                        message: "Authorization callback channel dropped - callback never fired or panicked".to_string(),
                    })
                }
                Err(_) => {
                    // Timeout - callback didn't fire in time
                    // Note: callback might still fire later, but result will be ignored
                    Err(crate::components::NotificationError::PlatformError {
                        platform: "macOS".to_string(),
                        error_code: None,
                        message: "Authorization check timed out after 5 seconds - callback did not respond".to_string(),
                    })
                }
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

    #[allow(unused_variables)]
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
                let request_body = request.content.body.to_structured_plain_text();
                
                // Extract subtitle from content or body
                let subtitle_text = request.content.subtitle.clone()
                    .or_else(|| request.content.body.extract_subtitle());

                // Create a oneshot channel for the completion handler
                use std::sync::{Arc, Mutex};

                use tokio::sync::oneshot;

                let (completion_tx, completion_rx) = oneshot::channel();
                let completion_tx = Arc::new(Mutex::new(Some(completion_tx)));

                // Resolve images BEFORE entering the non-Send block (downloads remote URLs)
                let resolved_images = super::image_utils::resolve_media_images(&request.content.media).await;

                // No spawn needed - these are quick synchronous calls
                // Scope the block so it's dropped before await (block is not Send)
                {
                    let center = UNUserNotificationCenter::currentNotificationCenter();

                    // Create notification content
                    let content = UNMutableNotificationContent::new();

                    // Title
                    let title_ns = NSString::from_str(&request_title);
                    content.setTitle(&title_ns);

                    // Subtitle - use content.subtitle or extract from body
                    if let Some(ref subtitle) = subtitle_text {
                        let subtitle_ns = NSString::from_str(subtitle);
                        content.setSubtitle(&subtitle_ns);
                    }

                    // Body - use structured plain text for better readability
                    let body_ns = NSString::from_str(&request_body);
                    content.setBody(&body_ns);

                    // Set default sound
                    let default_sound = objc2_user_notifications::UNNotificationSound::defaultSound();
                    content.setSound(Some(&default_sound));

                    // Add resolved image attachments
                    let mut attachments_vec: Vec<Retained<objc2_user_notifications::UNNotificationAttachment>> = Vec::new();
                    for (idx, (_placement, resolved)) in resolved_images.iter().enumerate() {
                        if let Some(attachment) = Self::create_image_attachment_from_path(&resolved.path, &format!("image-{}", idx)) {
                            attachments_vec.push(attachment);
                        }
                    }
                    if !attachments_vec.is_empty() {
                        use objc2_foundation::NSArray;
                        let refs: Vec<&objc2_user_notifications::UNNotificationAttachment> =
                            attachments_vec.iter().map(|a| &**a).collect();
                        let attachments_array = NSArray::from_slice(&refs);
                        content.setAttachments(&attachments_array);
                    }

                    // Create immediate trigger
                    let trigger = UNTimeIntervalNotificationTrigger::triggerWithTimeInterval_repeats(
                        0.1, false,
                    );

                    // Create request
                    let identifier = NSString::from_str(&notification_id);
                    let notification_request = UNNotificationRequest::requestWithIdentifier_content_trigger(
                        &identifier,
                        &content,
                        Some(&trigger),
                    );

                    // Create completion handler using RcBlock directly
                    //
                    // SAFETY: Thread-safety analysis (same as check_authorization):
                    // - `completion_tx: Arc<Mutex<Option<oneshot::Sender<bool>>>>` is `Send + Sync`
                    // - Apple invokes completion handler on arbitrary thread
                    // - All captured variables verified `Send + Sync` at compile time
                    // - Block heap-allocated and retained by Apple for callback duration
                    let completion_block = RcBlock::new(fnonce_to_fn1(
                        move |error: *mut objc2_foundation::NSError| {
                            // Success is indicated by null error pointer (Apple convention)
                            let success = error.is_null();

                            // Send result through channel, handling mutex poisoning gracefully
                            match completion_tx.lock() {
                                Ok(mut sender_guard) => {
                                    if let Some(sender) = sender_guard.take() {
                                        let _ = sender.send(success);
                                    }
                                }
                                Err(poisoned) => {
                                    // Mutex was poisoned - still try to send with failure status
                                    if let Some(sender) = poisoned.into_inner().take() {
                                        let _ = sender.send(false);
                                    }
                                }
                            }
                        },
                    ));

                    center.addNotificationRequest_withCompletionHandler(
                        &notification_request,
                        Some(&completion_block),
                    );
                } // block is dropped here, before the await

                // Extract configured timeout from delivery options
                let timeout_duration = request.options.delivery_timeout;

                // Wait for completion with configurable timeout
                let delivery_success =
                    match tokio::time::timeout(timeout_duration, completion_rx).await
                    {
                        Ok(Ok(true)) => true,
                        Ok(Ok(false)) => {
                            // Notification delivery failed - cancel to prevent ghost notifications
                            Self::cancel_notification_sync(&notification_id);
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: None,
                                message: "Notification delivery failed - Apple API returned error".to_string(),
                            });
                        },
                        Ok(Err(_)) => {
                            // Channel dropped - callback never fired or panicked
                            // Cancel to ensure no ghost notifications
                            Self::cancel_notification_sync(&notification_id);
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: None,
                                message: "Completion callback channel dropped - callback never fired or panicked".to_string(),
                            });
                        },
                        Err(_elapsed) => {
                            // Timeout occurred - cancel the pending notification to prevent ghost notifications
                            tracing::warn!(
                                notification_id = %notification_id,
                                timeout_ms = timeout_duration.as_millis(),
                                "Notification delivery timed out, cancelling pending request"
                            );
                            Self::cancel_notification_sync(&notification_id);
                            return Err(crate::components::NotificationError::PlatformError {
                                platform: "macOS".to_string(),
                                error_code: Some(408), // HTTP 408 Request Timeout semantics
                                message: format!(
                                    "Notification delivery timeout after {}ms - request cancelled",
                                    timeout_duration.as_millis()
                                ),
                            });
                        },
                    };

                // If we get here, delivery was successful
                if delivery_success {
                    // Success - create delivery receipt using the builder pattern
                    let receipt = DeliveryReceipt::new(Platform::MacOS, notification_id)
                        .with_metadata("platform_api".to_string(), "UserNotifications".to_string())
                        .with_metadata("authorization_status".to_string(), "granted".to_string())
                        .with_metadata("delivery_method".to_string(), "tokio_async_task".to_string());

                    Ok(receipt)
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

    #[allow(unused_variables)]
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
                // No spawn needed - these are quick synchronous calls
                let update_id = id.to_string();
                let content_changes = update.content_changes.clone();

                let center = UNUserNotificationCenter::currentNotificationCenter();

                // On macOS, we need to remove and re-add to update
                let identifier = NSString::from_str(&update_id);

                // Convert to NSArray for the API call
                use objc2_foundation::NSArray;
                let identifiers_vec = vec![&*identifier];
                let identifiers_array = NSArray::from_slice(&identifiers_vec);

                center.removePendingNotificationRequestsWithIdentifiers(&identifiers_array);
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
                    );

                    let new_identifier = NSString::from_str(&update_id);
                    let notification_request = UNNotificationRequest::requestWithIdentifier_content_trigger(
                        &new_identifier,
                        &new_content,
                        Some(&trigger),
                    );

                    center.addNotificationRequest_withCompletionHandler(
                        &notification_request,
                        None,
                    );
                }

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

    #[allow(unused_variables)]
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
