// Sophisticated notification lifecycle management with enterprise patterns
// Based on comprehensive study of Slack's distributed tracing, Discord's state management,
// Teams' client data layer, and production notification delivery patterns

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::time_wrapper::DefaultableInstant;
use super::{CorrelationId, NotificationError, NotificationResult, Platform};

/// Comprehensive notification lifecycle management component
/// Incorporates enterprise patterns for state tracking, retry logic, and observability
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct NotificationLifecycle {
    /// Current notification state
    pub state: NotificationState,
    /// Platform-specific delivery states
    pub platform_states: HashMap<Platform, PlatformDeliveryState>,
    /// Timing information for performance monitoring
    pub timing: NotificationTiming,
    /// Retry policy and current attempt tracking
    pub retry_policy: RetryPolicy,
    /// Expiration and TTL management
    pub expiration: ExpirationPolicy,
    /// Delivery receipt and confirmation
    pub delivery_receipt: Option<DeliveryReceipt>,
    /// State transition history for debugging
    pub state_history: Vec<StateTransition>,
    /// Performance metrics collection
    pub performance_metrics: PerformanceMetrics,
}

impl NotificationLifecycle {
    pub fn new() -> Self {
        let now = DefaultableInstant::now();
        Self {
            state: NotificationState::Created,
            platform_states: HashMap::new(),
            timing: NotificationTiming::new(now),
            retry_policy: RetryPolicy::default(),
            expiration: ExpirationPolicy::default(),
            delivery_receipt: None,
            state_history: vec![StateTransition {
                from_state: None,
                to_state: NotificationState::Created,
                timestamp: now.inner(),
                reason: TransitionReason::Initial,
                correlation_id: None,
            }],
            performance_metrics: PerformanceMetrics::new(),
        }
    }

    /// Update timing information with current elapsed time
    pub fn update_timing(&mut self, elapsed: Duration) {
        self.timing.update(elapsed);
    }

    /// Schedule a retry with the specified delay
    pub fn schedule_retry(&mut self, delay: Duration) {
        self.retry_policy.schedule_next_attempt(delay);
    }

    /// Transition to a new state with proper tracking and validation
    pub fn transition_to(
        &mut self,
        new_state: NotificationState,
        reason: TransitionReason,
        correlation_id: Option<CorrelationId>,
    ) -> NotificationResult<()> {
        // Validate state transition
        if !self.state.can_transition_to(&new_state) {
            return Err(NotificationError::ValidationError {
                field: "state_transition".to_string(),
                message: format!(
                    "Invalid transition from {:?} to {:?}",
                    self.state, new_state
                ),
            });
        }

        let now = DefaultableInstant::now();
        let previous_state = self.state.clone();

        // Record state transition
        let transition = StateTransition {
            from_state: Some(previous_state.clone()),
            to_state: new_state.clone(),
            timestamp: now.inner(),
            reason,
            correlation_id,
        };

        // Update timing based on state
        match &new_state {
            NotificationState::Validating => {
                self.timing.validation_started = Some(now);
            },
            NotificationState::PlatformRouting => {
                if let Some(validation_start) = self.timing.validation_started {
                    self.timing.validation_duration = Some(now.duration_since(validation_start));
                }
                self.timing.platform_routing_started = Some(now);
            },
            NotificationState::Queued => {
                self.timing.queued_at = Some(now);
            },
            NotificationState::Delivering => {
                self.timing.delivery_started = Some(now);
            },
            NotificationState::Delivered => {
                if let Some(delivery_start) = self.timing.delivery_started {
                    self.timing.delivery_duration = Some(now.duration_since(delivery_start));
                }
                self.timing.delivered_at = Some(now);

                // Calculate total processing time
                self.timing.total_processing_time =
                    Some(now.duration_since(self.timing.created_at));
            },
            NotificationState::InteractionReceived => {
                self.timing.last_interaction = Some(now);
            },
            NotificationState::Failed(_) => {
                self.timing.failed_at = Some(now);
            },
            NotificationState::Expired => {
                self.timing.expired_at = Some(now);
            },
            _ => {},
        }

        // Update state and history
        self.state = new_state;
        self.state_history.push(transition);

        // Update performance metrics
        self.performance_metrics
            .record_state_transition(&previous_state, &self.state, now.inner());

        Ok(())
    }

    /// Check if notification has expired based on TTL or absolute expiration
    pub fn is_expired(&self) -> bool {
        let now = DefaultableInstant::now();

        // Check absolute expiration time
        if let Some(expires_at) = self.expiration.expires_at
            && now.inner() >= expires_at {
                return true;
            }

        // Check TTL from creation
        if let Some(ttl) = self.expiration.ttl
            && now.duration_since(self.timing.created_at) >= ttl {
                return true;
            }

        // Check state-specific timeouts
        match &self.state {
            NotificationState::Delivering => {
                if let Some(started) = self.timing.delivery_started
                    && now.duration_since(started) > self.expiration.delivery_timeout {
                        return true;
                    }
            },
            NotificationState::InteractionPending => {
                if let Some(delivered) = self.timing.delivered_at
                    && now.duration_since(delivered) > self.expiration.interaction_timeout {
                        return true;
                    }
            },
            _ => {},
        }

        false
    }

    /// Check if notification should be retried based on policy
    pub fn should_retry(&self) -> bool {
        matches!(self.state, NotificationState::Failed(_))
            && self.retry_policy.current_attempt < self.retry_policy.max_attempts
            && self.retry_policy.circuit_breaker_state == CircuitBreakerState::Closed
    }

    /// Calculate next retry delay using exponential backoff
    pub fn next_retry_delay(&self) -> Duration {
        self.retry_policy
            .calculate_next_delay(self.retry_policy.current_attempt)
    }

    /// Record a delivery attempt for retry tracking
    pub fn record_delivery_attempt(&mut self, result: DeliveryAttemptResult) {
        self.retry_policy.current_attempt += 1;
        self.retry_policy.last_attempt = Some(DefaultableInstant::now().inner());

        match result {
            DeliveryAttemptResult::Success(receipt) => {
                self.delivery_receipt = Some(receipt);
                self.retry_policy.consecutive_failures = 0;
            },
            DeliveryAttemptResult::Failure(error) => {
                self.retry_policy.consecutive_failures += 1;
                self.retry_policy.last_error = Some(error);

                // Update circuit breaker state
                if self.retry_policy.consecutive_failures
                    >= self.retry_policy.circuit_breaker_threshold
                {
                    self.retry_policy.circuit_breaker_state = CircuitBreakerState::Open;
                    self.retry_policy.circuit_breaker_opened_at =
                        Some(DefaultableInstant::now().inner());
                }
            },
        }
    }

    /// Update platform-specific delivery state
    pub fn update_platform_state(&mut self, platform: Platform, state: PlatformDeliveryState) {
        self.platform_states.insert(platform, state);
    }

    /// Get the overall delivery progress across all platforms
    pub fn delivery_progress(&self) -> DeliveryProgress {
        if self.platform_states.is_empty() {
            return match self.state {
                NotificationState::Created | NotificationState::Validating => {
                    DeliveryProgress::NotStarted
                },
                NotificationState::Delivered => DeliveryProgress::Completed,
                NotificationState::Failed(_) => DeliveryProgress::Failed,
                _ => DeliveryProgress::InProgress,
            };
        }

        let total_platforms = self.platform_states.len();
        let successful = self
            .platform_states
            .values()
            .filter(|state| matches!(state.status, PlatformDeliveryStatus::Delivered))
            .count();
        let failed = self
            .platform_states
            .values()
            .filter(|state| matches!(state.status, PlatformDeliveryStatus::Failed(_)))
            .count();

        if successful == total_platforms {
            DeliveryProgress::Completed
        } else if failed == total_platforms {
            DeliveryProgress::Failed
        } else {
            DeliveryProgress::PartiallyComplete {
                successful,
                total: total_platforms,
            }
        }
    }

    /// Get detailed performance metrics
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_metrics
    }
}

impl Default for NotificationLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive notification state machine with enterprise-grade states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum NotificationState {
    /// Initial state when notification is created
    #[default]
    Created,
    /// Content validation and sanitization in progress
    Validating,
    /// Platform capability negotiation and routing
    PlatformRouting,
    /// Queued for delivery (attention management, batching)
    Queued,
    /// Active delivery to platform(s) in progress
    Delivering,
    /// Successfully delivered to platform(s)
    Delivered,
    /// Waiting for user interaction
    InteractionPending,
    /// User has interacted with notification
    InteractionReceived,
    /// Processing interaction response
    ProcessingResponse,
    /// Notification updated with new content
    Updated,
    /// Notification cancelled before delivery
    Cancelled,
    /// Notification expired (TTL reached)
    Expired,
    /// Delivery or processing failed
    Failed(ErrorDetails),
    /// Notification completed its full lifecycle
    Completed,
}


impl NotificationState {
    /// Check if transition to target state is valid
    pub fn can_transition_to(&self, target: &NotificationState) -> bool {
        use NotificationState::*;

        match (self, target) {
            // From Created
            (Created, Validating) => true,
            (Created, Cancelled) => true,

            // From Validating
            (Validating, PlatformRouting) => true,
            (Validating, Failed(_)) => true,
            (Validating, Cancelled) => true,

            // From PlatformRouting
            (PlatformRouting, Queued) => true,
            (PlatformRouting, Delivering) => true,
            (PlatformRouting, Failed(_)) => true,
            (PlatformRouting, Cancelled) => true,

            // From Queued
            (Queued, Delivering) => true,
            (Queued, Expired) => true,
            (Queued, Cancelled) => true,

            // From Delivering
            (Delivering, Delivered) => true,
            (Delivering, Failed(_)) => true,
            (Delivering, Expired) => true,
            (Delivering, Cancelled) => true,

            // From Delivered
            (Delivered, InteractionPending) => true,
            (Delivered, InteractionReceived) => true,
            (Delivered, Updated) => true,
            (Delivered, Expired) => true,
            (Delivered, Completed) => true,

            // From InteractionPending
            (InteractionPending, InteractionReceived) => true,
            (InteractionPending, ProcessingResponse) => true,
            (InteractionPending, Expired) => true,

            // From InteractionReceived
            (InteractionReceived, ProcessingResponse) => true,
            (InteractionReceived, Completed) => true,

            // From ProcessingResponse
            (ProcessingResponse, InteractionPending) => true,
            (ProcessingResponse, Completed) => true,
            (ProcessingResponse, Failed(_)) => true,

            // From Updated
            (Updated, Delivering) => true,
            (Updated, InteractionPending) => true,
            (Updated, Completed) => true,

            // From Failed - can retry
            (Failed(_), Validating) => true,
            (Failed(_), PlatformRouting) => true,
            (Failed(_), Delivering) => true,
            (Failed(_), Cancelled) => true,

            // Terminal states generally don't allow transitions
            (Cancelled, _) => false,
            (Expired, _) => false,
            (Completed, _) => false,

            _ => false,
        }
    }

    /// Check if this is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            NotificationState::Cancelled
                | NotificationState::Expired
                | NotificationState::Completed
        )
    }

    /// Check if this state indicates success
    pub fn is_successful(&self) -> bool {
        matches!(
            self,
            NotificationState::Delivered
                | NotificationState::InteractionReceived
                | NotificationState::Completed
        )
    }

    /// Check if this state indicates failure
    pub fn is_failed(&self) -> bool {
        matches!(self, NotificationState::Failed(_))
    }
}

/// Platform-specific delivery state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDeliveryState {
    pub platform: Platform,
    pub status: PlatformDeliveryStatus,
    pub native_id: Option<String>,
    pub attempt_count: u32,
    #[serde(skip)]
    pub last_attempt: Option<Instant>,
    pub delivery_latency: Option<Duration>,
    pub error_details: Option<PlatformError>,
}

/// Platform delivery status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformDeliveryStatus {
    Pending,
    InProgress,
    Delivered,
    Failed(String),
    Cancelled,
}

/// Platform-specific error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformError {
    pub error_code: Option<i32>,
    pub error_message: String,
    pub retry_after: Option<Duration>,
    pub is_permanent: bool,
}

/// Detailed error information for failed notifications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub error_type: ErrorType,
    pub message: String,
    pub retry_count: u32,
    pub last_attempt: Option<SystemTime>,
    pub platform_errors: HashMap<Platform, String>,
}

/// Types of errors that can occur
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    ValidationError,
    AuthorizationError,
    NetworkError,
    PlatformError,
    TimeoutError,
    ResourceError,
    ConfigurationError,
}

/// Comprehensive timing information for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationTiming {
    /// When notification was created
    #[serde(skip)]
    pub created_at: DefaultableInstant,
    /// When validation started
    #[serde(skip)]
    pub validation_started: Option<DefaultableInstant>,
    /// How long validation took
    pub validation_duration: Option<Duration>,
    /// When platform routing started
    #[serde(skip)]
    pub platform_routing_started: Option<DefaultableInstant>,
    /// When notification was queued
    #[serde(skip)]
    pub queued_at: Option<DefaultableInstant>,
    /// How long notification was queued
    pub queue_duration: Option<Duration>,
    /// When delivery started
    #[serde(skip)]
    pub delivery_started: Option<DefaultableInstant>,
    /// How long delivery took
    pub delivery_duration: Option<Duration>,
    /// When successfully delivered
    #[serde(skip)]
    pub delivered_at: Option<DefaultableInstant>,
    /// Last user interaction time
    #[serde(skip)]
    pub last_interaction: Option<DefaultableInstant>,
    /// When notification failed
    #[serde(skip)]
    pub failed_at: Option<DefaultableInstant>,
    /// When notification expired
    #[serde(skip)]
    pub expired_at: Option<DefaultableInstant>,
    /// Total processing time from creation to completion
    pub total_processing_time: Option<Duration>,
}

impl Default for NotificationTiming {
    fn default() -> Self {
        Self::new(DefaultableInstant::now())
    }
}

impl NotificationTiming {
    pub fn new(created_at: DefaultableInstant) -> Self {
        Self {
            created_at,
            validation_started: None,
            validation_duration: None,
            platform_routing_started: None,
            queued_at: None,
            queue_duration: None,
            delivery_started: None,
            delivery_duration: None,
            delivered_at: None,
            last_interaction: None,
            failed_at: None,
            expired_at: None,
            total_processing_time: None,
        }
    }

    /// Update timing information with elapsed time
    /// This method is called by NotificationLifecycle to maintain accurate timing data
    pub fn update(&mut self, _elapsed: Duration) {
        let now = DefaultableInstant::now();

        // Update queue duration if we have a queue start time
        if let Some(queued_at) = self.queued_at
            && self.queue_duration.is_none() {
                // Calculate how long we've been queued so far
                let time_queued = now.duration_since(queued_at);
                self.queue_duration = Some(time_queued);
            }

        // Update validation duration if validation started but not yet recorded
        if let (Some(validation_start), None) = (self.validation_started, self.validation_duration)
            && (self.platform_routing_started.is_some() || self.delivery_started.is_some()) {
                self.validation_duration = Some(now.duration_since(validation_start));
            }

        // Update delivery duration if delivery started but not yet completed
        if let (Some(delivery_start), None) = (self.delivery_started, self.delivery_duration)
            && self.delivered_at.is_some() {
                self.delivery_duration = Some(now.duration_since(delivery_start));
                // Also update total processing time when delivery completes
                self.total_processing_time = Some(now.duration_since(self.created_at));
            }
    }

    /// Calculate time spent in current state
    pub fn time_in_current_state(&self, current_state: &NotificationState) -> Duration {
        let now = DefaultableInstant::now();

        match current_state {
            NotificationState::Validating => self.validation_started.unwrap_or(now).elapsed(),
            NotificationState::PlatformRouting => {
                self.platform_routing_started.unwrap_or(now).elapsed()
            },
            NotificationState::Queued => self.queued_at.unwrap_or(now).elapsed(),
            NotificationState::Delivering => self.delivery_started.unwrap_or(now).elapsed(),
            _ => now.duration_since(self.created_at),
        }
    }
}

/// Enterprise-grade retry policy with exponential backoff and circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Current attempt number
    pub current_attempt: u32,
    /// Backoff strategy for retry delays
    pub backoff_strategy: BackoffStrategy,
    /// Circuit breaker configuration
    pub circuit_breaker_state: CircuitBreakerState,
    pub circuit_breaker_threshold: u32,
    #[serde(skip)]
    pub circuit_breaker_opened_at: Option<Instant>,
    pub circuit_breaker_timeout: Duration,
    /// Failure tracking
    pub consecutive_failures: u32,
    pub last_error: Option<NotificationError>,
    #[serde(skip)]
    pub last_attempt: Option<Instant>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            current_attempt: 0,
            backoff_strategy: BackoffStrategy::ExponentialWithJitter {
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(30),
                multiplier: 2.0,
                jitter: 0.1,
            },
            circuit_breaker_state: CircuitBreakerState::Closed,
            circuit_breaker_threshold: 5,
            circuit_breaker_opened_at: None,
            circuit_breaker_timeout: Duration::from_secs(5 * 60),
            consecutive_failures: 0,
            last_error: None,
            last_attempt: None,
        }
    }
}

impl RetryPolicy {
    pub fn calculate_next_delay(&self, attempt: u32) -> Duration {
        self.backoff_strategy.calculate_delay(attempt)
    }

    pub fn should_retry_circuit_breaker(&mut self) -> bool {
        match self.circuit_breaker_state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(opened_at) = self.circuit_breaker_opened_at {
                    if opened_at.elapsed() >= self.circuit_breaker_timeout {
                        self.circuit_breaker_state = CircuitBreakerState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Schedule the next retry attempt with the specified delay
    /// This method is called by NotificationLifecycle to manage retry scheduling
    pub fn schedule_next_attempt(&mut self, _delay: Duration) {
        let now = DefaultableInstant::now();

        // Record this scheduling attempt
        self.last_attempt = Some(now.inner());

        // Reset circuit breaker state if it has been open long enough
        if self.circuit_breaker_state == CircuitBreakerState::Open
            && let Some(opened_at) = self.circuit_breaker_opened_at
                && now.inner().duration_since(opened_at) >= self.circuit_breaker_timeout {
                    self.circuit_breaker_state = CircuitBreakerState::HalfOpen;
                }

        // If circuit breaker is in half-open state and we're scheduling a retry,
        // it means the previous attempt succeeded, so we can close the circuit
        if self.circuit_breaker_state == CircuitBreakerState::HalfOpen {
            self.circuit_breaker_state = CircuitBreakerState::Closed;
            self.consecutive_failures = 0;
            self.circuit_breaker_opened_at = None;
        }

        // Store the delay for monitoring and metrics
        // Note: The actual scheduling is handled by the Bevy system using this delay
    }
}

/// Backoff strategies for retry delays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed(Duration),
    /// Linear increase in delay
    Linear {
        base_delay: Duration,
        increment: Duration,
        max_delay: Duration,
    },
    /// Exponential backoff with optional jitter
    ExponentialWithJitter {
        base_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        jitter: f64,
    },
}

impl BackoffStrategy {
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            BackoffStrategy::Fixed(delay) => *delay,
            BackoffStrategy::Linear {
                base_delay,
                increment,
                max_delay,
            } => {
                let calculated = *base_delay + *increment * attempt;
                calculated.min(*max_delay)
            },
            BackoffStrategy::ExponentialWithJitter {
                base_delay,
                max_delay,
                multiplier,
                jitter,
            } => {
                let exponential_delay = Duration::from_millis(
                    (base_delay.as_millis() as f64 * multiplier.powi(attempt as i32)) as u64,
                );

                let with_jitter = if *jitter > 0.0 {
                    let jitter_amount = exponential_delay.as_millis() as f64
                        * jitter
                        * (rand::random::<f64>() - 0.5)
                        * 2.0;
                    Duration::from_millis(
                        (exponential_delay.as_millis() as f64 + jitter_amount) as u64,
                    )
                } else {
                    exponential_delay
                };

                with_jitter.min(*max_delay)
            },
        }
    }
}

/// Circuit breaker states for failure handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Normal operation, requests allowed
    Closed,
    /// Failing fast, requests blocked
    Open,
    /// Testing if service recovered
    HalfOpen,
}

/// Expiration policy for notification TTL management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpirationPolicy {
    /// Time-to-live from creation
    pub ttl: Option<Duration>,
    /// Absolute expiration time
    #[serde(skip)]
    pub expires_at: Option<Instant>,
    /// Maximum time allowed for delivery
    pub delivery_timeout: Duration,
    /// Maximum time to wait for user interaction
    pub interaction_timeout: Duration,
    /// Cleanup behavior when expired
    pub cleanup_on_expiry: bool,
}

impl Default for ExpirationPolicy {
    fn default() -> Self {
        Self {
            ttl: Some(Duration::from_secs(60 * 60)), // Default 1 hour TTL
            expires_at: None,
            delivery_timeout: Duration::from_secs(30),
            interaction_timeout: Duration::from_secs(10 * 60),
            cleanup_on_expiry: true,
        }
    }
}

/// Delivery receipt for successful notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeliveryReceipt {
    pub platform: Platform,
    pub native_id: String,
    #[serde(skip)]
    pub delivered_at: Instant,
    pub delivery_latency: Duration,
    pub receipt_id: String,
    pub metadata: HashMap<String, String>,
}

impl Default for DeliveryReceipt {
    fn default() -> Self {
        Self {
            platform: Platform::default(),
            native_id: String::default(),
            delivered_at: DefaultableInstant::now().inner(),
            delivery_latency: Duration::default(),
            receipt_id: String::default(),
            metadata: HashMap::default(),
        }
    }
}

/// Result of a delivery attempt
#[derive(Debug, Clone)]
pub enum DeliveryAttemptResult {
    Success(DeliveryReceipt),
    Failure(NotificationError),
}

/// State transition tracking for debugging and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StateTransition {
    pub from_state: Option<NotificationState>,
    pub to_state: NotificationState,
    #[serde(skip)]
    pub timestamp: Instant,
    pub reason: TransitionReason,
    pub correlation_id: Option<CorrelationId>,
}

impl Default for StateTransition {
    fn default() -> Self {
        Self {
            from_state: None,
            to_state: NotificationState::default(),
            timestamp: DefaultableInstant::now().inner(),
            reason: TransitionReason::default(),
            correlation_id: None,
        }
    }
}

/// Reasons for state transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum TransitionReason {
    #[default]
    Initial,
    ValidationCompleted,
    ValidationFailed,
    PlatformCapabilitiesResolved,
    QueuedByAttentionManager,
    DeliveryStarted,
    DeliveryCompleted,
    DeliveryFailed,
    UserInteraction,
    Timeout,
    Cancellation,
    Expiration,
    Retry,
    Update,
    SystemEvent,
}


/// Overall delivery progress tracking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryProgress {
    NotStarted,
    InProgress,
    PartiallyComplete { successful: usize, total: usize },
    Completed,
    Failed,
}

/// Performance metrics collection for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// State transition timings
    pub state_transitions: Vec<StateTransitionMetric>,
    /// Platform-specific performance
    pub platform_metrics: HashMap<Platform, PlatformMetrics>,
    /// Overall performance indicators
    pub overall_metrics: OverallMetrics,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            state_transitions: Vec::new(),
            platform_metrics: HashMap::new(),
            overall_metrics: OverallMetrics::default(),
        }
    }

    pub fn record_state_transition(
        &mut self,
        from_state: &NotificationState,
        to_state: &NotificationState,
        timestamp: Instant,
    ) {
        let transition = StateTransitionMetric {
            from_state: from_state.clone(),
            to_state: to_state.clone(),
            timestamp,
            duration_in_previous_state: None, // Calculate from previous transition
        };

        // Calculate duration in previous state
        if let Some(last_transition) = self.state_transitions.last_mut() {
            last_transition.duration_in_previous_state =
                Some(timestamp.duration_since(last_transition.timestamp));
        }

        self.state_transitions.push(transition);
        self.overall_metrics.total_state_transitions += 1;
    }

    pub fn record_platform_metrics(&mut self, platform: Platform, metrics: PlatformMetrics) {
        self.platform_metrics.insert(platform, metrics);
    }
}

/// State transition performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StateTransitionMetric {
    pub from_state: NotificationState,
    pub to_state: NotificationState,
    #[serde(skip)]
    pub timestamp: Instant,
    pub duration_in_previous_state: Option<Duration>,
}

impl Default for StateTransitionMetric {
    fn default() -> Self {
        Self {
            from_state: NotificationState::default(),
            to_state: NotificationState::default(),
            timestamp: DefaultableInstant::now().inner(),
            duration_in_previous_state: None,
        }
    }
}

/// Platform-specific performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformMetrics {
    pub delivery_latency: Option<Duration>,
    pub success_rate: f64,
    pub error_count: u32,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

/// Overall notification performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverallMetrics {
    pub total_state_transitions: u64,
    pub average_processing_time: Option<Duration>,
    pub success_rate: f64,
    pub error_rate: f64,
    pub retry_rate: f64,
}


