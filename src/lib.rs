//! Enterprise-grade cross-platform notification system
//!
//! This crate provides a sophisticated notification system with rich media support,
//! comprehensive analytics, distributed tracing, and full cross-platform capabilities.
//!
//! Based on comprehensive study of enterprise notification architectures from
//! Slack, Discord, VS Code, Teams, and native platform capabilities.
#![recursion_limit = "256"]
#![allow(hidden_glob_reexports)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub mod backends;
pub mod components;

// Re-export all components for convenience
pub use backends::*;
pub use components::*;

/// Internal state container for notification data
#[allow(dead_code)] // Internal state management structure
struct NotificationState {
    identity: NotificationIdentity,
    content: NotificationContent,
    lifecycle: NotificationLifecycle,
    platform_integration: PlatformIntegration,
    analytics: NotificationAnalytics,
}

/// Notification struct that replaces the ECS Bundle
#[derive(Debug, Clone)]
pub struct Notification {
    pub identity: NotificationIdentity,
    pub content: NotificationContent,
    pub platform_integration: PlatformIntegration,
    pub lifecycle: NotificationLifecycle,
    pub analytics: NotificationAnalytics,
}

/// Handle for querying notification status
pub struct NotificationHandle {
    pub id: NotificationId,
    state: Arc<RwLock<HashMap<NotificationId, NotificationState>>>,
}

impl NotificationHandle {
    /// Get the current notification status
    pub async fn status(&self) -> Option<NotificationStatus> {
        let state = self.state.read().await;
        state.get(&self.id).map(|s| NotificationStatus {
            id: self.id,
            state: s.lifecycle.state.clone(),
            platforms: s.platform_integration.target_platforms.clone(),
            created_at: s.identity.created_at,
        })
    }

    /// Get the lifecycle details
    pub async fn lifecycle(&self) -> Option<NotificationLifecycle> {
        let state = self.state.read().await;
        state.get(&self.id).map(|s| s.lifecycle.clone())
    }

    /// Get the analytics data
    pub async fn analytics(&self) -> Option<NotificationAnalytics> {
        let state = self.state.read().await;
        state.get(&self.id).map(|s| s.analytics.clone())
    }
}

/// Public status structure for notification queries
#[derive(Debug, Clone)]
pub struct NotificationStatus {
    pub id: NotificationId,
    pub state: crate::components::lifecycle::NotificationState,
    pub platforms: Vec<Platform>,
    pub created_at: crate::components::time_wrapper::DefaultableInstant,
}

/// Notification Manager - main entry point for the library
pub struct NotificationManager {
    state: Arc<RwLock<HashMap<NotificationId, NotificationState>>>,
    #[allow(dead_code)] // Stored for ownership; passed to workers during initialization
    platform_backends: Arc<HashMap<Platform, Box<dyn PlatformBackend>>>,
    task_handles: Vec<JoinHandle<()>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl NotificationManager {
    /// Create a new notification manager and spawn background workers
    pub fn new() -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let state = Arc::new(RwLock::new(HashMap::new()));
        let platform_backends = Arc::new(PlatformBackendFactory::get_supported_backends());

        // Spawn background workers
        let task_handles = vec![
            // Lifecycle monitor worker
            tokio::spawn(lifecycle_monitor(
                Arc::clone(&state),
                shutdown_tx.subscribe(),
            )),
            // Delivery worker
            tokio::spawn(delivery_worker(
                Arc::clone(&state),
                Arc::clone(&platform_backends),
                shutdown_tx.subscribe(),
            )),
            // Analytics aggregator
            tokio::spawn(analytics_aggregator(
                Arc::clone(&state),
                shutdown_tx.subscribe(),
            )),
        ];

        Self {
            state,
            platform_backends,
            task_handles,
            shutdown_tx,
        }
    }

    /// Send a notification and get a handle for tracking
    pub async fn send(
        &self,
        notification: Notification,
    ) -> Result<NotificationHandle, NotificationError> {
        let id = notification.identity.id;

        // Store notification state
        {
            let mut state = self.state.write().await;
            state.insert(
                id,
                NotificationState {
                    identity: notification.identity,
                    content: notification.content,
                    lifecycle: notification.lifecycle,
                    platform_integration: notification.platform_integration,
                    analytics: notification.analytics,
                },
            );
        }

        Ok(NotificationHandle {
            id,
            state: Arc::clone(&self.state),
        })
    }

    /// Track a notification by ID
    pub async fn track(&self, id: NotificationId) -> Option<NotificationStatus> {
        let state = self.state.read().await;
        state.get(&id).map(|s| NotificationStatus {
            id,
            state: s.lifecycle.state.clone(),
            platforms: s.platform_integration.target_platforms.clone(),
            created_at: s.identity.created_at,
        })
    }

    /// Gracefully shutdown the manager and all background workers
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        for handle in self.task_handles {
            let _ = handle.await;
        }
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Background worker for lifecycle monitoring
async fn lifecycle_monitor(
    state: Arc<RwLock<HashMap<NotificationId, NotificationState>>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut state = state.write().await;

                for (_id, notification_state) in state.iter_mut() {
                    // Update lifecycle timing
                    notification_state.lifecycle.update_timing();

                    // Check for expired notifications
                    if notification_state.lifecycle.is_expired() {
                        let _ = notification_state.lifecycle.transition_to(
                            crate::components::lifecycle::NotificationState::Expired,
                            crate::components::lifecycle::TransitionReason::Expiration,
                            None,
                        );
                    }

                    // Handle retry logic
                    if notification_state.lifecycle.should_retry() {
                        let delay = notification_state.lifecycle.next_retry_delay();
                        notification_state.lifecycle.schedule_retry(delay);
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}

/// Background worker for notification delivery
async fn delivery_worker(
    state: Arc<RwLock<HashMap<NotificationId, NotificationState>>>,
    platform_backends: Arc<HashMap<Platform, Box<dyn PlatformBackend>>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut state = state.write().await;

                for (_id, notification_state) in state.iter_mut() {
                    if notification_state.lifecycle.state == crate::components::lifecycle::NotificationState::Queued {
                        // Attempt delivery to each target platform
                        for platform in &notification_state.platform_integration.target_platforms.clone() {
                            if notification_state.platform_integration.is_authorized(*platform) {
                                let _ = notification_state.lifecycle.transition_to(
                                    crate::components::lifecycle::NotificationState::Delivering,
                                    crate::components::lifecycle::TransitionReason::DeliveryStarted,
                                    Some(notification_state.identity.correlation_id.clone()),
                                );

                                // Get backend from pre-loaded map
                                if let Some(backend) = platform_backends.get(platform) {
                                    let request = crate::components::platform::NotificationRequest {
                                        notification_id: notification_state.identity.id.to_string(),
                                        content: notification_state.content.clone(),
                                        options: crate::components::platform::DeliveryOptions::default(),
                                        correlation_id: notification_state.identity.correlation_id.to_string(),
                                    };

                                    // ACTUALLY DELIVER TO PLATFORM
                                    match backend.deliver_notification(&request).await {
                                        Ok(receipt) => {
                                            // Real success - notification actually sent to platform
                                            let _ = notification_state.lifecycle.transition_to(
                                                crate::components::lifecycle::NotificationState::Delivered,
                                                crate::components::lifecycle::TransitionReason::DeliveryCompleted,
                                                Some(notification_state.identity.correlation_id.clone()),
                                            );

                                            // Update platform state with real receipt
                                            let platform_state = crate::components::lifecycle::PlatformDeliveryState {
                                                platform: receipt.platform,
                                                status: crate::components::lifecycle::PlatformDeliveryStatus::Delivered,
                                                native_id: Some(receipt.native_id),
                                                attempt_count: 1,
                                                last_attempt: Some(std::time::Instant::now()),
                                                delivery_latency: Some(
                                                    receipt
                                                        .delivered_at
                                                        .duration_since(std::time::UNIX_EPOCH)
                                                        .unwrap_or_default(),
                                                ),
                                                error_details: None,
                                            };
                                            notification_state.lifecycle.update_platform_state(*platform, platform_state);
                                        }
                                        Err(error) => {
                                            // Real failure - actual platform error
                                            let error_details = crate::components::lifecycle::ErrorDetails {
                                                error_type: crate::components::lifecycle::ErrorType::PlatformError,
                                                message: error.to_string(),
                                                retry_count: notification_state.lifecycle.retry_policy.current_attempt,
                                                last_attempt: Some(std::time::SystemTime::now()),
                                                platform_errors: {
                                                    let mut errors = std::collections::HashMap::new();
                                                    errors.insert(*platform, error.to_string());
                                                    errors
                                                },
                                            };

                                            let _ = notification_state.lifecycle.transition_to(
                                                crate::components::lifecycle::NotificationState::Failed(error_details),
                                                crate::components::lifecycle::TransitionReason::DeliveryFailed,
                                                Some(notification_state.identity.correlation_id.clone()),
                                            );

                                            // Update platform state with real error
                                            let platform_state = crate::components::lifecycle::PlatformDeliveryState {
                                                platform: *platform,
                                                status: crate::components::lifecycle::PlatformDeliveryStatus::Failed(
                                                    error.to_string(),
                                                ),
                                                native_id: None,
                                                attempt_count: notification_state.lifecycle.retry_policy.current_attempt + 1,
                                                last_attempt: Some(std::time::Instant::now()),
                                                delivery_latency: None,
                                                error_details: Some(crate::components::lifecycle::PlatformError {
                                                    error_code: None,
                                                    error_message: error.to_string(),
                                                    retry_after: None,
                                                    is_permanent: false,
                                                }),
                                            };
                                            notification_state.lifecycle.update_platform_state(*platform, platform_state);
                                        }
                                    }
                                }
                            } else {
                                // Real authorization check failed
                                let error_details = crate::components::lifecycle::ErrorDetails {
                                    error_type: crate::components::lifecycle::ErrorType::AuthorizationError,
                                    message: format!("Authorization required for platform: {}", platform.name()),
                                    retry_count: 0,
                                    last_attempt: Some(std::time::SystemTime::now()),
                                    platform_errors: {
                                        let mut errors = std::collections::HashMap::new();
                                        errors.insert(*platform, "Authorization required".to_string());
                                        errors
                                    },
                                };

                                let _ = notification_state.lifecycle.transition_to(
                                    crate::components::lifecycle::NotificationState::Failed(error_details),
                                    crate::components::lifecycle::TransitionReason::DeliveryFailed,
                                    Some(notification_state.identity.correlation_id.clone()),
                                );
                            }
                        }
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}

/// Background worker for analytics aggregation
async fn analytics_aggregator(
    state: Arc<RwLock<HashMap<NotificationId, NotificationState>>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut state = state.write().await;

                for (_id, notification_state) in state.iter_mut() {
                    // Update analytics and metrics
                    notification_state.analytics.update_metrics();

                    let effectiveness_score = notification_state.analytics.calculate_effectiveness_score();
                    if effectiveness_score > 0.0 {
                        notification_state.analytics.record_effectiveness_calculation(effectiveness_score);
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}

/// Builder for creating notifications with fluent API
pub struct NotificationBuilder {
    identity: Option<NotificationIdentity>,
    content: Option<NotificationContent>,
    platform_integration: Option<PlatformIntegration>,
    lifecycle: Option<NotificationLifecycle>,
    analytics: Option<NotificationAnalytics>,
}

impl NotificationBuilder {
    pub fn new() -> Self {
        Self {
            identity: None,
            content: None,
            platform_integration: None,
            lifecycle: None,
            analytics: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        let title_string = title.into();
        if let Some(ref mut content) = self.content {
            content.title = title_string;
        } else {
            self.content = Some(NotificationContent::new(title_string, RichText::plain("")));
        }
        self
    }

    pub fn with_body(mut self, body: impl Into<RichText>) -> Self {
        if let Some(ref mut content) = self.content {
            content.body = body.into();
        } else {
            self.content = Some(NotificationContent::new("", body));
        }
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        if let Some(ref mut content) = self.content {
            content.priority = priority;
        }
        self
    }

    pub fn with_platforms(mut self, platforms: Vec<Platform>) -> Self {
        self.platform_integration = Some(PlatformIntegration::new(platforms));
        self
    }

    /// Build the notification (returns Notification instead of NotificationBundle)
    pub fn build(self) -> Notification {
        let session_id = SessionId::generate();
        let creator_context = CreatorContext::new("native-notifications");

        Notification {
            identity: self
                .identity
                .unwrap_or_else(|| NotificationIdentity::new(session_id.clone(), creator_context)),
            content: self
                .content
                .unwrap_or_else(|| NotificationContent::new("Notification", RichText::plain(""))),
            platform_integration: self.platform_integration.unwrap_or_else(|| {
                PlatformIntegration::new(vec![Platform::MacOS, Platform::Windows, Platform::Linux])
            }),
            lifecycle: self.lifecycle.unwrap_or_default(),
            analytics: self.analytics.unwrap_or_else(|| {
                NotificationAnalytics::new(NotificationId::generate(), CorrelationId::generate())
            }),
        }
    }
}

impl Default for NotificationBuilder {
    fn default() -> Self {
        Self::new()
    }
}
