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

use dashmap::DashMap;
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
    state: Arc<DashMap<NotificationId, NotificationState>>,
}

impl NotificationHandle {
    /// Get the current notification status
    pub async fn status(&self) -> Option<NotificationStatus> {
        self.state.get(&self.id).map(|s| NotificationStatus {
            id: self.id,
            state: s.lifecycle.state.clone(),
            platforms: s.platform_integration.target_platforms.clone(),
            created_at: s.identity.created_at,
        })
    }

    /// Get the lifecycle details
    pub async fn lifecycle(&self) -> Option<NotificationLifecycle> {
        self.state.get(&self.id).map(|s| s.lifecycle.clone())
    }

    /// Get the analytics data
    pub async fn analytics(&self) -> Option<NotificationAnalytics> {
        self.state.get(&self.id).map(|s| s.analytics.clone())
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

/// Result of NotificationManager shutdown operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownResult {
    /// All workers shut down cleanly
    Clean,
    /// One or more workers panicked during shutdown
    WorkersPanicked(usize),
    /// Shutdown timed out waiting for workers
    TimedOut,
}

/// Notification Manager - main entry point for the library
pub struct NotificationManager {
    state: Arc<DashMap<NotificationId, NotificationState>>,
    platform_backends: Arc<HashMap<Platform, Box<dyn PlatformBackend>>>,
    task_handles: Vec<JoinHandle<()>>,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl NotificationManager {
    /// Create a new notification manager and spawn background workers
    pub fn new() -> Self {
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let state = Arc::new(DashMap::new());
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
        let correlation_id = notification.identity.correlation_id.clone();

        // Transition lifecycle to Queued state so delivery_worker will process it
        let mut lifecycle = notification.lifecycle;
        lifecycle.transition_to(
            crate::components::lifecycle::NotificationState::Queued,
            crate::components::lifecycle::TransitionReason::QueuedByAttentionManager,
            Some(correlation_id),
        )?;

        // Store notification state with Queued lifecycle
        self.state.insert(
            id,
            NotificationState {
                identity: notification.identity,
                content: notification.content,
                lifecycle,  // Now in Queued state, ready for delivery_worker
                platform_integration: notification.platform_integration,
                analytics: notification.analytics,
            },
        );

        Ok(NotificationHandle {
            id,
            state: Arc::clone(&self.state),
        })
    }

    /// Track a notification by ID
    pub async fn track(&self, id: NotificationId) -> Option<NotificationStatus> {
        self.state.get(&id).map(|s| NotificationStatus {
            id,
            state: s.lifecycle.state.clone(),
            platforms: s.platform_integration.target_platforms.clone(),
            created_at: s.identity.created_at,
        })
    }

    /// Gracefully shutdown the manager and all background workers with default 30s timeout
    pub async fn shutdown(self) -> ShutdownResult {
        self.shutdown_with_timeout(Duration::from_secs(30)).await
    }

    /// Gracefully shutdown the manager with custom timeout duration
    pub async fn shutdown_with_timeout(self, timeout: Duration) -> ShutdownResult {
        let start = std::time::Instant::now();
        
        // PHASE 1: Collect metrics BEFORE shutdown
        let mut queued_count = 0;
        let mut delivering_count = 0;
        let mut delivered_count = 0;
        let mut failed_count = 0;
        let mut other_count = 0;
        
        for entry in self.state.iter() {
            match &entry.value().lifecycle.state {
                crate::components::lifecycle::NotificationState::Queued => queued_count += 1,
                crate::components::lifecycle::NotificationState::Delivering => delivering_count += 1,
                crate::components::lifecycle::NotificationState::Delivered => delivered_count += 1,
                crate::components::lifecycle::NotificationState::Failed(_) => failed_count += 1,
                _ => other_count += 1,
            }
        }
        
        let total_notifications = queued_count + delivering_count + delivered_count + failed_count + other_count;
        let in_flight_count = queued_count + delivering_count;
        
        ::tracing::info!(
            "NotificationManager shutdown starting - Total: {}, In-flight: {} (Queued: {}, Delivering: {}), Delivered: {}, Failed: {}, Other: {}",
            total_notifications, in_flight_count, queued_count, delivering_count, delivered_count, failed_count, other_count
        );
        
        // PHASE 2: Cancel all non-terminal notifications on their target platforms
        let mut cancelled_count = 0;
        let mut cancel_errors = 0;
        
        for entry in self.state.iter() {
            let notification = entry.value();
            // Only cancel notifications in Delivering state (already sent to platform)
            // Queued notifications haven't been delivered yet, so nothing to cancel
            if matches!(
                notification.lifecycle.state,
                crate::components::lifecycle::NotificationState::Delivering
            ) {
                let notification_id = notification.identity.id.to_string();

                // Cancel on each target platform for this notification
                for platform in &notification.platform_integration.target_platforms {
                    if let Some(backend) = self.platform_backends.get(platform) {
                        // Wrap cancellation in timeout (2 seconds per notification)
                        let cancel_result = tokio::time::timeout(
                            std::time::Duration::from_secs(2),
                            backend.cancel_notification(&notification_id)
                        ).await;

                        match cancel_result {
                            Ok(Ok(())) => {
                                cancelled_count += 1;
                            }
                            Ok(Err(e)) => {
                                ::tracing::warn!(
                                    "Failed to cancel notification {} on {:?}: {}",
                                    notification_id, platform, e
                                );
                                cancel_errors += 1;
                            }
                            Err(_) => {
                                ::tracing::warn!(
                                    "Timeout cancelling notification {} on {:?} (exceeded 2s)",
                                    notification_id, platform
                                );
                                cancel_errors += 1;
                            }
                        }
                    }
                }
            }
        }
        
        if cancelled_count > 0 || cancel_errors > 0 {
            ::tracing::info!(
                "Cancelled {} notification-platform pairs ({} errors)",
                cancelled_count, cancel_errors
            );
        }
        
        // PHASE 3: Send shutdown signal to workers
        if self.shutdown_tx.send(()).is_err() {
            ::tracing::warn!("Shutdown signal send failed - workers may have already stopped");
        }
        
        // PHASE 4: Wait for workers with timeout
        let shutdown_result = tokio::time::timeout(timeout, async {
            let mut results = Vec::new();
            for handle in self.task_handles {
                results.push(handle.await);
            }
            results
        }).await;
        
        let result = match shutdown_result {
            Ok(results) => {
                let panicked: Vec<_> = results.iter()
                    .filter(|r| r.is_err())
                    .collect();
                
                if panicked.is_empty() {
                    ShutdownResult::Clean
                } else {
                    ::tracing::error!("{} worker(s) panicked during shutdown", panicked.len());
                    ShutdownResult::WorkersPanicked(panicked.len())
                }
            }
            Err(_) => {
                ::tracing::error!("Shutdown timed out after {:?}", timeout);
                ShutdownResult::TimedOut
            }
        };
        
        // PHASE 5: Log final metrics
        let elapsed = start.elapsed();
        ::tracing::info!(
            "NotificationManager shutdown completed in {:?} - Result: {:?}, Notifications cancelled: {}, Cancel errors: {}",
            elapsed, result, cancelled_count, cancel_errors
        );
        
        // PHASE 6: Cleanup cached temp images
        crate::backends::cleanup_all_cached_images();
        ::tracing::debug!("Cleaned up cached notification images");

        result
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Background worker for lifecycle monitoring
/// 
/// Uses DashMap for lock-free concurrent access, allowing this worker to iterate
/// over notifications without blocking other workers or status queries.
async fn lifecycle_monitor(
    state: Arc<DashMap<NotificationId, NotificationState>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // DashMap allows lock-free iteration - each entry is locked independently
                for mut entry in state.iter_mut() {
                    let notification_state = entry.value_mut();
                    
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

/// Delivery job collected during read lock phase
struct DeliveryJob {
    notification_id: NotificationId,
    platform: Platform,
    is_authorized: bool,
    request: Option<crate::components::platform::NotificationRequest>,
    correlation_id: CorrelationId,
}

/// Result of a delivery attempt
enum DeliveryResult {
    Success {
        notification_id: NotificationId,
        platform: Platform,
        receipt: crate::components::DeliveryReceipt,
        correlation_id: CorrelationId,
    },
    Failure {
        notification_id: NotificationId,
        platform: Platform,
        error: String,
        correlation_id: CorrelationId,
        retry_count: u32,
    },
    Unauthorized {
        notification_id: NotificationId,
        platform: Platform,
        correlation_id: CorrelationId,
    },
}

/// Background worker for notification delivery
/// 
/// CRITICAL FIX: This worker now uses DashMap for lock-free concurrent access.
/// DashMap provides fine-grained locking at the entry level, so different
/// notifications can be accessed concurrently without blocking each other.
/// The pattern is:
/// 1. Iterate over entries (lock-free), collect work items
/// 2. Perform async deliveries without holding any locks
/// 3. Update individual entries using DashMap's entry API (per-entry locking)
///
/// This prevents blocking other operations (status queries, lifecycle monitoring)
/// during network I/O which can take 3-5+ seconds per delivery.
async fn delivery_worker(
    state: Arc<DashMap<NotificationId, NotificationState>>,
    platform_backends: Arc<HashMap<Platform, Box<dyn PlatformBackend>>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(50));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // PHASE 1: Collect delivery jobs using DashMap iteration (lock-free)
                let delivery_jobs: Vec<DeliveryJob> = state
                    .iter()
                    .filter(|entry| entry.value().lifecycle.state == crate::components::lifecycle::NotificationState::Queued)
                    .flat_map(|entry| {
                        let id = *entry.key();
                        let notification_state = entry.value();
                        
                        notification_state
                            .platform_integration
                            .target_platforms
                            .iter()
                            .filter(|platform| {
                                // Skip if already delivered to this platform
                                if let Some(platform_state) = notification_state.lifecycle.platform_states.get(platform) {
                                    !matches!(platform_state.status, crate::components::lifecycle::PlatformDeliveryStatus::Delivered)
                                } else {
                                    true // Not in platform_states yet, needs delivery
                                }
                            })
                            .map(|platform| {
                                let is_authorized = notification_state.platform_integration.is_authorized(*platform);
                                let request = if is_authorized {
                                    Some(crate::components::platform::NotificationRequest {
                                        notification_id: notification_state.identity.id.to_string(),
                                        content: notification_state.content.clone(),
                                        options: crate::components::platform::DeliveryOptions::default(),
                                        correlation_id: notification_state.identity.correlation_id.to_string(),
                                    })
                                } else {
                                    None
                                };

                                DeliveryJob {
                                    notification_id: id,
                                    platform: *platform,
                                    is_authorized,
                                    request,
                                    correlation_id: notification_state.identity.correlation_id.clone(),
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect();

                // PHASE 2: Transition to Delivering state (per-entry locking via DashMap)
                for job in &delivery_jobs {
                    if job.is_authorized
                        && let Some(mut entry) = state.get_mut(&job.notification_id)
                    {
                        let _ = entry.lifecycle.transition_to(
                            crate::components::lifecycle::NotificationState::Delivering,
                            crate::components::lifecycle::TransitionReason::DeliveryStarted,
                            Some(job.correlation_id.clone()),
                        );
                    }
                }

                // PHASE 3: Perform deliveries WITHOUT holding any lock (can take seconds)
                let mut delivery_results: Vec<DeliveryResult> = Vec::new();

                for job in delivery_jobs {
                    if !job.is_authorized
                        && let Some(backend) = platform_backends.get(&job.platform)
                    {
                        match backend.request_authorization().await {
                            Ok(true) => {
                                // Permission granted! Update authorization state
                                if let Some(mut entry) = state.get_mut(&job.notification_id) {
                                    entry.platform_integration.update_authorization(
                                        job.platform,
                                        AuthorizationState::Authorized {
                                            granted_at: std::time::SystemTime::now(),
                                            permissions: vec![PermissionLevel::Display],
                                        }
                                    );
                                }
                                // Continue with delivery (don't skip!)
                            }
                            Ok(false) | Err(_) => {
                                delivery_results.push(DeliveryResult::Unauthorized {
                                    notification_id: job.notification_id,
                                    platform: job.platform,
                                    correlation_id: job.correlation_id,
                                });
                                continue;
                            }
                        }
                    }

                    if let Some(backend) = platform_backends.get(&job.platform)
                        && let Some(request) = job.request
                    {
                        // ACTUALLY DELIVER TO PLATFORM (no lock held!)
                        match backend.deliver_notification(&request).await {
                            Ok(receipt) => {
                                delivery_results.push(DeliveryResult::Success {
                                    notification_id: job.notification_id,
                                    platform: job.platform,
                                    receipt,
                                    correlation_id: job.correlation_id,
                                });
                            }
                            Err(error) => {
                                // Get retry count using DashMap's get (per-entry locking)
                                let retry_count = state
                                    .get(&job.notification_id)
                                    .map(|ns| ns.lifecycle.retry_policy.current_attempt)
                                    .unwrap_or(0);

                                delivery_results.push(DeliveryResult::Failure {
                                    notification_id: job.notification_id,
                                    platform: job.platform,
                                    error: error.to_string(),
                                    correlation_id: job.correlation_id,
                                    retry_count,
                                });
                            }
                        }
                    }
                }

                // PHASE 4: Update state with results (per-entry locking via DashMap)
                for result in delivery_results {
                    if let Some(mut notification_state) = state.get_mut(&result.notification_id()) {
                        let correlation_id = match &result {
                            DeliveryResult::Success { correlation_id, .. } => correlation_id.clone(),
                            DeliveryResult::Failure { correlation_id, .. } => correlation_id.clone(),
                            DeliveryResult::Unauthorized { correlation_id, .. } => correlation_id.clone(),
                        };

                        match result {
                            DeliveryResult::Success { platform, receipt, .. } => {
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
                                notification_state.lifecycle.update_platform_state(platform, platform_state);
                            }
                            DeliveryResult::Failure { platform, error, retry_count, .. } => {
                                // Update platform state with real error
                                let platform_state = crate::components::lifecycle::PlatformDeliveryState {
                                    platform,
                                    status: crate::components::lifecycle::PlatformDeliveryStatus::Failed(error.clone()),
                                    native_id: None,
                                    attempt_count: retry_count + 1,
                                    last_attempt: Some(std::time::Instant::now()),
                                    delivery_latency: None,
                                    error_details: Some(crate::components::lifecycle::PlatformError {
                                        error_code: None,
                                        error_message: error,
                                        retry_after: None,
                                        is_permanent: false,
                                    }),
                                };
                                notification_state.lifecycle.update_platform_state(platform, platform_state);
                            }
                            DeliveryResult::Unauthorized { platform, .. } => {
                                // Update platform state with authorization error
                                let platform_state = crate::components::lifecycle::PlatformDeliveryState {
                                    platform,
                                    status: crate::components::lifecycle::PlatformDeliveryStatus::Failed("Authorization required".to_string()),
                                    native_id: None,
                                    attempt_count: 1,
                                    last_attempt: Some(std::time::Instant::now()),
                                    delivery_latency: None,
                                    error_details: Some(crate::components::lifecycle::PlatformError {
                                        error_code: None,
                                        error_message: format!("Authorization required for platform: {}", platform.name()),
                                        retry_after: None,
                                        is_permanent: true,
                                    }),
                                };
                                notification_state.lifecycle.update_platform_state(platform, platform_state);
                            }
                        }

                        // After updating platform state, check if we should transition the overall notification state
                        // Get all target platforms for this notification
                        let target_platforms = &notification_state.platform_integration.target_platforms;
                        
                        // Check delivery status across all target platforms
                        let all_delivered = target_platforms.iter().all(|p| {
                            notification_state.lifecycle.platform_states
                                .get(p)
                                .is_some_and(|ps| matches!(ps.status, crate::components::lifecycle::PlatformDeliveryStatus::Delivered))
                        });

                        let any_failed = target_platforms.iter().any(|p| {
                            notification_state.lifecycle.platform_states
                                .get(p)
                                .is_some_and(|ps| matches!(ps.status, crate::components::lifecycle::PlatformDeliveryStatus::Failed(_)))
                        });

                        // Transition to Delivered only if ALL target platforms succeeded
                        if all_delivered {
                            let _ = notification_state.lifecycle.transition_to(
                                crate::components::lifecycle::NotificationState::Delivered,
                                crate::components::lifecycle::TransitionReason::DeliveryCompleted,
                                Some(correlation_id),
                            );
                        } else if any_failed {
                            // At least one platform failed - collect error details
                            let mut platform_errors = std::collections::HashMap::new();
                            let mut max_retry_count = 0;
                            
                            for platform in target_platforms {
                                if let Some(platform_state) = notification_state.lifecycle.platform_states.get(platform)
                                    && let crate::components::lifecycle::PlatformDeliveryStatus::Failed(ref error_msg) = platform_state.status
                                {
                                    platform_errors.insert(*platform, error_msg.clone());
                                    max_retry_count = max_retry_count.max(platform_state.attempt_count.saturating_sub(1));
                                }
                            }

                            if !platform_errors.is_empty() {
                                let error_details = crate::components::lifecycle::ErrorDetails {
                                    error_type: crate::components::lifecycle::ErrorType::PlatformError,
                                    message: format!("Delivery failed for {} platform(s)", platform_errors.len()),
                                    retry_count: max_retry_count,
                                    last_attempt: Some(std::time::SystemTime::now()),
                                    platform_errors,
                                };

                                let _ = notification_state.lifecycle.transition_to(
                                    crate::components::lifecycle::NotificationState::Failed(error_details),
                                    crate::components::lifecycle::TransitionReason::DeliveryFailed,
                                    Some(correlation_id),
                                );
                            }
                        }
                        // else: some platforms are still pending/in-progress, keep current state
                    }
                }
            }
            _ = shutdown_rx.recv() => break,
        }
    }
}

impl DeliveryResult {
    fn notification_id(&self) -> NotificationId {
        match self {
            DeliveryResult::Success { notification_id, .. } => *notification_id,
            DeliveryResult::Failure { notification_id, .. } => *notification_id,
            DeliveryResult::Unauthorized { notification_id, .. } => *notification_id,
        }
    }
}

/// Background worker for analytics aggregation
/// 
/// Uses DashMap for lock-free concurrent access, allowing this worker to update
/// analytics without blocking other workers or status queries.
async fn analytics_aggregator(
    state: Arc<DashMap<NotificationId, NotificationState>>,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // DashMap allows lock-free iteration - each entry is locked independently
                for mut entry in state.iter_mut() {
                    let notification_state = entry.value_mut();
                    
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

/// Error types for notification building
#[derive(Debug, thiserror::Error)]
pub enum NotificationBuildError {
    #[error("Notification content is required")]
    MissingContent,
    
    #[error("Title is required and cannot be empty")]
    MissingTitle,
    
    #[error("At least one target platform is required")]
    NoPlatforms,
    
    #[error("Content validation failed: {0}")]
    ValidationError(#[from] NotificationError),
    
    #[error("Title exceeds maximum length for {platform}: {length} > {max}")]
    TitleTooLong {
        platform: String,
        length: usize,
        max: usize,
    },
    
    #[error("Body exceeds maximum length for {platform}: {length} > {max}")]
    BodyTooLong {
        platform: String,
        length: usize,
        max: usize,
    },
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

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        if let Some(ref mut content) = self.content {
            content.subtitle = Some(subtitle.into());
        } else {
            let mut content = NotificationContent::new("", RichText::plain(""));
            content.subtitle = Some(subtitle.into());
            self.content = Some(content);
        }
        self
    }

    pub fn with_media(mut self, media: MediaAttachment) -> Self {
        if let Some(ref mut content) = self.content {
            content.media.push(media);
        } else {
            let mut content = NotificationContent::new("", RichText::plain(""));
            content.media.push(media);
            self.content = Some(content);
        }
        self
    }

    /// Build the notification with validation
    /// 
    /// Returns an error if:
    /// - Content is missing or invalid
    /// - Title is empty
    /// - No target platforms are specified
    /// - Content exceeds platform-specific limits
    pub fn build(self) -> Result<Notification, NotificationBuildError> {
        // Validate content is present
        let mut content = self.content.ok_or(NotificationBuildError::MissingContent)?;
        
        // Validate title is not empty
        if content.title.is_empty() {
            return Err(NotificationBuildError::MissingTitle);
        }
        
        // Get target platforms (use provided or default to all desktop platforms)
        let platforms = self.platform_integration
            .as_ref()
            .map(|p| p.target_platforms.clone())
            .unwrap_or_else(|| vec![Platform::MacOS, Platform::Windows, Platform::Linux]);
        
        // Validate platforms list is not empty
        if platforms.is_empty() {
            return Err(NotificationBuildError::NoPlatforms);
        }
        
        // Validate content against platform limits for each target platform
        for platform in &platforms {
            let limits = platform.default_capabilities().get_limits();
            
            // Validate title length for this platform
            if let Some(&max_title) = limits.get("max_title_length")
                && content.title.len() > max_title
            {
                return Err(NotificationBuildError::TitleTooLong {
                    platform: platform.name().to_string(),
                    length: content.title.len(),
                    max: max_title,
                });
            }
            
            // Validate using NotificationContent::validate() for this platform
            content.validate(&limits)?;
        }
        
        let session_id = SessionId::generate();
        let creator_context = CreatorContext::new("native-notifications");

        Ok(Notification {
            identity: self
                .identity
                .unwrap_or_else(|| NotificationIdentity::new(session_id.clone(), creator_context)),
            content,
            platform_integration: self.platform_integration.unwrap_or_else(|| {
                PlatformIntegration::new(platforms)
            }),
            lifecycle: self.lifecycle.unwrap_or_default(),
            analytics: self.analytics.unwrap_or_else(|| {
                NotificationAnalytics::new(NotificationId::generate(), CorrelationId::generate())
            }),
        })
    }
}

impl Default for NotificationBuilder {
    fn default() -> Self {
        Self::new()
    }
}
