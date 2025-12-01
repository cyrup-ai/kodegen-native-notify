// Enterprise-grade ECS notification components
// Based on comprehensive study of Slack, Discord, VS Code, Teams architectures

use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use time_wrapper::DefaultableInstant;
use uuid::Uuid;

pub mod analytics;
pub mod content;
pub mod lifecycle;
pub mod platform;
pub mod serde_time;
pub mod time_wrapper;
pub mod tracing;

pub use analytics::{
    AnalyticsError, AnalyticsSummary, BusinessImpact, BusinessMetrics, ContentEffectiveness,
    ContentEffectivenessMetrics, ContentPerformance, ConversionEvent, ConversionType, CostMetrics,
    DistributedTraceData, EngagementDepth, ErrorAnalytics, ErrorType as AnalyticsErrorType,
    ExperimentData, ExperimentType, FeedbackType, InteractionContext, InteractionOutcome,
    InteractionType, JourneyStage, NotificationAnalytics,
    PerformanceMetrics as AnalyticsPerformanceMetrics, PlatformAnalytics, PlatformSummary,
    RetentionMetrics, ServiceHop, TraceEvent, TraceEventType, UserBehaviorMetrics, UserInteraction,
};
pub use content::{
    AccessibilityMetadata, ActionConfirmation, ActionIcon, ActionId, ActionPayload, ActionStyle,
    ActivationType, AudioSource, ContextMenuAction, ImageData, ImageFormat, ImagePlacement,
    InputId, InputValidation, InteractionSet, LocalizationData, MediaAttachment,
    NotificationAction, NotificationContent, NotificationInput, NotificationInteraction,
    QuickReply, RichText, SelectionOption, SystemSound, ValidationState, VideoData, VideoFormat,
    VideoSource,
};
pub use lifecycle::{
    BackoffStrategy, CircuitBreakerState, DeliveryAttemptResult, DeliveryProgress,
    ErrorDetails as LifecycleErrorDetails, ErrorType as LifecycleErrorType, ExpirationPolicy,
    NotificationLifecycle, NotificationState, NotificationTiming,
    PerformanceMetrics as LifecyclePerformanceMetrics, PlatformDeliveryState,
    PlatformDeliveryStatus, PlatformError, RetryPolicy, StateTransition, TransitionReason,
};
pub use platform::{
    ActionChange, ActionFallback, AuthorizationManager, AuthorizationState, CompatibilityLevel,
    DegradationStrategy, DeliveryOptions, DeliveryReceipt as PlatformDeliveryReceipt,
    FeatureDegradation, FeatureMatrix, GlobalPreferences, MarkupFallback, MediaChange,
    MediaFallback, NativeHandleMetadata, NotificationRequest, NotificationUpdate, PermissionLevel,
    Platform, PlatformBackend, PlatformCapabilities, PlatformConfig, PlatformIntegration,
    PlatformManager, PlatformPreferences, PlatformUserSettings, RateLimit,
};
pub use tracing::{
    CorrelationData, PerformanceMarker, PerformanceMarkerType, SamplingConfig, TraceBreadcrumb,
    TraceContext, TracingContext, notification_tracing_system,
};

/// Unique notification identity with enterprise-grade tracing
/// Incorporates Slack's distributed tracing patterns and Discord's entity architecture
#[derive(Component, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationIdentity {
    /// Globally unique notification identifier
    pub id: NotificationId,
    /// Correlation ID for distributed tracing across services (Slack pattern)
    pub correlation_id: CorrelationId,
    /// Session identifier for user activity tracking
    pub session_id: SessionId,
    /// High-precision creation timestamp for latency measurement
    #[serde(skip)]
    pub created_at: DefaultableInstant,
    /// OpenTelemetry trace span for observability
    pub trace_span: Option<TraceSpan>,
    /// Application context that created this notification
    pub creator_context: CreatorContext,
}

impl Default for NotificationIdentity {
    fn default() -> Self {
        Self {
            id: NotificationId::generate(),
            correlation_id: CorrelationId::generate(),
            session_id: SessionId::generate(),
            created_at: DefaultableInstant::now(),
            trace_span: None,
            creator_context: CreatorContext::default(),
        }
    }
}

impl NotificationIdentity {
    pub fn new(session_id: SessionId, creator_context: CreatorContext) -> Self {
        Self {
            id: NotificationId::generate(),
            correlation_id: CorrelationId::generate(),
            session_id,
            created_at: DefaultableInstant::now(),
            trace_span: None,
            creator_context,
        }
    }

    pub fn with_correlation(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = correlation_id;
        self
    }

    pub fn with_trace_span(mut self, trace_span: TraceSpan) -> Self {
        self.trace_span = Some(trace_span);
        self
    }
}

/// Globally unique notification identifier with type safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationId(Uuid);

impl NotificationId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for NotificationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for NotificationId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Correlation ID for distributed tracing (Slack's approach)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// User session identifier for activity correlation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Context about the application/service that created this notification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorContext {
    /// Service name (e.g., "action-items-launcher")
    pub service_name: String,
    /// Service version for debugging and compatibility
    pub service_version: String,
    /// Feature/module that triggered notification
    pub feature_context: Option<String>,
    /// User ID or identifier if applicable
    pub user_context: Option<String>,
}

impl CreatorContext {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            feature_context: None,
            user_context: None,
        }
    }

    pub fn with_feature(mut self, feature: impl Into<String>) -> Self {
        self.feature_context = Some(feature.into());
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user_context = Some(user.into());
        self
    }
}

impl Default for CreatorContext {
    fn default() -> Self {
        Self::new("unknown")
    }
}

/// Distributed tracing span data (OpenTelemetry integration)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceSpan {
    pub span_id: SpanId,
    pub trace_id: TraceId,
    pub parent_span: Option<SpanId>,
    pub operation_name: String,
    #[serde(skip)]
    pub start_time: DefaultableInstant,
    pub attributes: HashMap<String, String>,
}

impl Default for TraceSpan {
    fn default() -> Self {
        Self {
            span_id: SpanId::generate(),
            trace_id: TraceId::generate(),
            parent_span: None,
            operation_name: String::default(),
            start_time: DefaultableInstant::now(),
            attributes: HashMap::default(),
        }
    }
}

impl TraceSpan {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            span_id: SpanId::generate(),
            trace_id: TraceId::generate(),
            parent_span: None,
            operation_name: operation_name.into(),
            start_time: DefaultableInstant::now(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_parent(mut self, parent: SpanId) -> Self {
        self.parent_span = Some(parent);
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    pub fn duration(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// OpenTelemetry Span ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl SpanId {
    pub fn generate() -> Self {
        Self(rand::random())
    }

    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// OpenTelemetry Trace ID  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(u128);

impl TraceId {
    pub fn generate() -> Self {
        Self(rand::random())
    }

    pub fn from_u128(id: u128) -> Self {
        Self(id)
    }

    pub fn as_u128(&self) -> u128 {
        self.0
    }
}

/// Priority levels for attention management (Slack's approach)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Background notifications (system updates, non-urgent info)
    Low = 1,
    /// Normal notifications (messages, general alerts)  
    Normal = 2,
    /// Important notifications (mentions, direct messages)
    High = 3,
    /// Critical notifications (emergencies, system failures)
    Critical = 4,
    /// Urgent notifications (incoming calls, time-sensitive alerts)
    Urgent = 5,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

impl Priority {
    /// Check if this priority level should bypass Do Not Disturb settings
    pub fn bypasses_dnd(&self) -> bool {
        matches!(self, Priority::Critical | Priority::Urgent)
    }

    /// Get timeout duration based on priority (higher priority = longer display)
    pub fn default_timeout(&self) -> Option<Duration> {
        match self {
            Priority::Low => Some(Duration::from_secs(3)),
            Priority::Normal => Some(Duration::from_secs(5)),
            Priority::High => Some(Duration::from_secs(10)),
            Priority::Critical => None, // No timeout - requires user action
            Priority::Urgent => None,   // No timeout - requires user action
        }
    }
}

/// Notification categories for grouping and template management
/// Inspired by macOS UNNotificationCategory and enterprise categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationCategory {
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub actions: Vec<CategoryAction>,
    pub options: CategoryOptions,
}

impl NotificationCategory {
    pub fn new(identifier: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            display_name: display_name.into(),
            description: None,
            actions: Vec::new(),
            options: CategoryOptions::default(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_action(mut self, action: CategoryAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_options(mut self, options: CategoryOptions) -> Self {
        self.options = options;
        self
    }
}

/// Pre-defined category actions for consistent UX
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryAction {
    pub identifier: String,
    pub title: String,
    pub options: ActionOptions,
    pub icon: Option<ActionIcon>,
}

/// Action behavior options (macOS UNNotificationActionOptions inspired)
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionOptions {
    pub authentication_required: bool,
    pub destructive: bool,
    pub foreground: bool,
}

/// Category behavior options (macOS UNNotificationCategoryOptions inspired)  
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryOptions {
    pub custom_dismiss_action: bool,
    pub allow_in_car_play: bool,
    pub hidden_preview_show_title: bool,
    pub hidden_preview_show_subtitle: bool,
}

/// Error types for comprehensive error handling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum NotificationError {
    /// Platform-specific delivery error
    PlatformError {
        platform: String,
        error_code: Option<i32>,
        message: String,
    },
    /// Content validation error
    ValidationError { field: String, message: String },
    /// Permission/authorization error
    AuthorizationError {
        platform: String,
        required_permission: String,
    },
    /// Resource error (file not found, network error, etc.)
    ResourceError {
        resource_type: String,
        resource_id: String,
        message: String,
    },
    /// Timeout error for delivery or interaction
    TimeoutError {
        operation: String,
        timeout_duration: Duration,
    },
    /// System resource exhaustion
    ResourceExhausted {
        resource_type: String,
        limit: usize,
        requested: usize,
    },
    /// Sanitization error (HTML/Markdown processing failed)
    SanitizationError {
        content_type: String,  // "html", "markdown", etc.
        message: String,
    },
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::PlatformError {
                platform,
                error_code,
                message,
            } => {
                write!(
                    f,
                    "Platform error on {}: {} (code: {:?})",
                    platform, message, error_code
                )
            },
            NotificationError::ValidationError { field, message } => {
                write!(f, "Validation error in {}: {}", field, message)
            },
            NotificationError::AuthorizationError {
                platform,
                required_permission,
            } => {
                write!(
                    f,
                    "Authorization error on {}: missing permission {}",
                    platform, required_permission
                )
            },
            NotificationError::ResourceError {
                resource_type,
                resource_id,
                message,
            } => {
                write!(
                    f,
                    "Resource error with {} '{}': {}",
                    resource_type, resource_id, message
                )
            },
            NotificationError::TimeoutError {
                operation,
                timeout_duration,
            } => {
                write!(
                    f,
                    "Timeout error in {} after {:?}",
                    operation, timeout_duration
                )
            },
            NotificationError::ResourceExhausted {
                resource_type,
                limit,
                requested,
            } => {
                write!(
                    f,
                    "Resource exhausted: {} limit {} exceeded (requested {})",
                    resource_type, limit, requested
                )
            },
            NotificationError::SanitizationError { content_type, message } => {
                write!(f, "Sanitization error for {}: {}", content_type, message)
            },
        }
    }
}

impl std::error::Error for NotificationError {}



/// Type alias for notification results with comprehensive error handling
pub type NotificationResult<T> = Result<T, NotificationError>;

// Helper traits for enterprise patterns

/// Trait for objects that can be traced in distributed systems
pub trait Traceable {
    fn correlation_id(&self) -> &CorrelationId;
    fn trace_span(&self) -> Option<&TraceSpan>;
}

impl Traceable for NotificationIdentity {
    fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    fn trace_span(&self) -> Option<&TraceSpan> {
        self.trace_span.as_ref()
    }
}

/// Trait for objects that support metrics collection
pub trait Measurable {
    fn record_metric(&self, metric_name: &str, value: f64, tags: Option<&HashMap<String, String>>);
    fn increment_counter(&self, counter_name: &str, tags: Option<&HashMap<String, String>>);
}

/// Trait for platform-specific feature support
pub trait PlatformSupport {
    fn supports_feature(&self, feature: &str) -> bool;
    fn get_platform_limits(&self) -> HashMap<String, usize>;
}

/// Default implementation of Measurable for NotificationIdentity
impl Measurable for NotificationIdentity {
    fn record_metric(&self, metric_name: &str, value: f64, tags: Option<&HashMap<String, String>>) {
        // Basic implementation - in production this would integrate with metrics backend
        println!(
            "Recording metric '{}' = {} for notification {}",
            metric_name, value, self.id
        );
        if let Some(tags) = tags {
            for (key, val) in tags {
                println!("  Tag: {} = {}", key, val);
            }
        }
    }

    fn increment_counter(&self, counter_name: &str, tags: Option<&HashMap<String, String>>) {
        // Basic implementation - in production this would integrate with metrics backend
        println!(
            "Incrementing counter '{}' for notification {}",
            counter_name, self.id
        );
        if let Some(tags) = tags {
            for (key, val) in tags {
                println!("  Tag: {} = {}", key, val);
            }
        }
    }
}

/// Default implementation of PlatformSupport for current platform
pub struct DefaultPlatformSupport;

impl PlatformSupport for DefaultPlatformSupport {
    fn supports_feature(&self, feature: &str) -> bool {
        // Basic feature detection - in production this would query actual platform capabilities
        match feature {
            "basic_notifications" => true,
            "rich_media" => cfg!(any(target_os = "macos", target_os = "windows")),
            "actions" => cfg!(any(
                target_os = "macos",
                target_os = "windows",
                target_os = "linux"
            )),
            "sound" => true,
            "custom_ui" => cfg!(target_os = "windows"),
            "background_activation" => cfg!(any(target_os = "macos", target_os = "windows")),
            _ => false,
        }
    }

    fn get_platform_limits(&self) -> HashMap<String, usize> {
        let mut limits = HashMap::new();

        #[cfg(target_os = "macos")]
        {
            limits.insert("max_title_length".to_string(), 256);
            limits.insert("max_body_length".to_string(), 2048);
            limits.insert("max_actions".to_string(), 4);
            limits.insert("max_image_size".to_string(), 10_485_760); // 10MB
        }

        #[cfg(target_os = "windows")]
        {
            limits.insert("max_title_length".to_string(), 128);
            limits.insert("max_body_length".to_string(), 1024);
            limits.insert("max_actions".to_string(), 5);
            limits.insert("max_image_size".to_string(), 204_800); // 200KB
        }

        #[cfg(target_os = "linux")]
        {
            limits.insert("max_title_length".to_string(), 512);
            limits.insert("max_body_length".to_string(), 4096);
            limits.insert("max_image_size".to_string(), 1_048_576); // 1MB
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            limits.insert("max_title_length".to_string(), 128);
            limits.insert("max_body_length".to_string(), 512);
        }

        limits
    }
}
