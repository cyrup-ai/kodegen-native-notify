// Distributed tracing utilities for enterprise notification observability
// Integrates with OpenTelemetry patterns and provides correlation tracking

use std::collections::HashMap;
use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::time_wrapper::DefaultableInstant;
use super::{CorrelationId, NotificationId, SpanId, TraceId, TraceSpan};

/// Tracing context component for notification entities
/// Provides distributed tracing capabilities across service boundaries
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TracingContext {
    /// Active trace span for this notification
    pub active_span: Option<TraceSpan>,
    /// Trace breadcrumbs for debugging
    pub breadcrumbs: Vec<TraceBreadcrumb>,
    /// Correlation tracking across services
    pub correlation_data: CorrelationData,
    /// Performance markers for observability
    pub performance_markers: Vec<PerformanceMarker>,
    /// Trace sampling configuration
    pub sampling_config: SamplingConfig,
}

impl TracingContext {
    pub fn new(correlation_id: CorrelationId) -> Self {
        Self {
            active_span: None,
            breadcrumbs: Vec::new(),
            correlation_data: CorrelationData::new(correlation_id),
            performance_markers: Vec::new(),
            sampling_config: SamplingConfig::default(),
        }
    }

    /// Start a new trace span
    pub fn start_span(&mut self, operation_name: impl Into<String>) -> &TraceSpan {
        let span = TraceSpan::new(operation_name).with_attribute(
            "notification_id",
            self.correlation_data.notification_id.to_string(),
        );

        // Set parent span if active span exists
        if let Some(ref active_span) = self.active_span {
            let span = span.with_parent(active_span.span_id);
            self.active_span = Some(span);
        } else {
            self.active_span = Some(span);
        }

        self.active_span
            .as_ref()
            .unwrap_or_else(|| panic!("Critical error: active span should exist immediately after being set - this indicates a programming error in tracing logic"))
    }

    /// Finish the current active span
    pub fn finish_span(&mut self) {
        if let Some(span) = self.active_span.take() {
            self.add_breadcrumb(TraceBreadcrumb {
                timestamp: DefaultableInstant::now(),
                operation: span.operation_name.clone(),
                duration: span.duration(),
                success: true,
                metadata: span.attributes.clone(),
            });
        }
    }

    /// Add a trace breadcrumb for debugging
    pub fn add_breadcrumb(&mut self, breadcrumb: TraceBreadcrumb) {
        self.breadcrumbs.push(breadcrumb);

        // Limit breadcrumb history to prevent memory growth
        if self.breadcrumbs.len() > 100 {
            self.breadcrumbs.remove(0);
        }
    }

    /// Record a performance marker
    pub fn record_performance_marker(&mut self, marker: PerformanceMarker) {
        self.performance_markers.push(marker);
    }

    /// Check if this trace should be sampled
    pub fn should_sample(&self) -> bool {
        self.sampling_config.should_sample()
    }

    /// Get trace context for propagation to external services
    pub fn get_trace_context(&self) -> Option<TraceContext> {
        self.active_span.as_ref().map(|span| TraceContext {
            trace_id: span.trace_id,
            span_id: span.span_id,
            correlation_id: self.correlation_data.correlation_id.clone(),
            sampling_decision: self.should_sample(),
        })
    }
}

impl Default for TracingContext {
    fn default() -> Self {
        Self::new(CorrelationId::generate())
    }
}

/// Trace breadcrumb for debugging and audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceBreadcrumb {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub operation: String,
    pub duration: std::time::Duration,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

impl Default for TraceBreadcrumb {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            operation: String::default(),
            duration: Duration::default(),
            success: true,
            metadata: HashMap::default(),
        }
    }
}

/// Correlation data for cross-service tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationData {
    pub correlation_id: CorrelationId,
    pub notification_id: NotificationId,
    pub session_context: Option<String>,
    pub user_context: Option<String>,
    pub request_id: Option<String>,
}

impl CorrelationData {
    pub fn new(correlation_id: CorrelationId) -> Self {
        Self {
            correlation_id,
            notification_id: NotificationId::generate(),
            session_context: None,
            user_context: None,
            request_id: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_context = Some(session_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_context = Some(user_id.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
}

/// Performance marker for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceMarker {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub marker_type: PerformanceMarkerType,
    pub value: f64,
    pub unit: String,
    pub tags: HashMap<String, String>,
}

impl Default for PerformanceMarker {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            marker_type: PerformanceMarkerType::default(),
            value: 0.0,
            unit: String::default(),
            tags: HashMap::default(),
        }
    }
}

/// Types of performance markers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum PerformanceMarkerType {
    #[default]
    Latency,
    Throughput,
    ErrorRate,
    QueueDepth,
    MemoryUsage,
    CpuUsage,
    Custom(String),
}


/// Trace context for external service propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub correlation_id: CorrelationId,
    pub sampling_decision: bool,
}

impl TraceContext {
    /// Convert to HTTP headers for trace propagation
    pub fn to_http_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "X-Trace-Id".to_string(),
            format!("{:x}", self.trace_id.as_u128()),
        );
        headers.insert(
            "X-Span-Id".to_string(),
            format!("{:x}", self.span_id.as_u64()),
        );
        headers.insert(
            "X-Correlation-Id".to_string(),
            self.correlation_id.to_string(),
        );
        headers.insert(
            "X-Sampling-Decision".to_string(),
            self.sampling_decision.to_string(),
        );
        headers
    }

    /// Create from HTTP headers
    pub fn from_http_headers(headers: &HashMap<String, String>) -> Option<Self> {
        let trace_id = headers
            .get("X-Trace-Id")
            .and_then(|s| u128::from_str_radix(s, 16).ok())
            .map(TraceId::from_u128)?;

        let span_id = headers
            .get("X-Span-Id")
            .and_then(|s| u64::from_str_radix(s, 16).ok())
            .map(SpanId::from_u64)?;

        let correlation_id = headers
            .get("X-Correlation-Id")
            .map(|s| CorrelationId::from_string(s.clone()))?;

        let sampling_decision = headers
            .get("X-Sampling-Decision")
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        Some(Self {
            trace_id,
            span_id,
            correlation_id,
            sampling_decision,
        })
    }
}

/// Sampling configuration for trace collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    pub sample_rate: f64, // 0.0 to 1.0
    pub force_sample_high_priority: bool,
    pub force_sample_errors: bool,
    pub max_samples_per_second: Option<u32>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            sample_rate: 0.1, // 10% sampling by default
            force_sample_high_priority: true,
            force_sample_errors: true,
            max_samples_per_second: Some(100),
        }
    }
}

impl SamplingConfig {
    pub fn should_sample(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::rng();
        rng.random::<f64>() < self.sample_rate
    }

    pub fn with_sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }
}

/// Tracing system for managing notification observability
pub fn notification_tracing_system(mut query: Query<&mut TracingContext>) {
    for mut tracing_context in query.iter_mut() {
        // Clean up old breadcrumbs
        let cutoff_time = DefaultableInstant::now() - std::time::Duration::from_secs(3600); // 1 hour
        tracing_context
            .breadcrumbs
            .retain(|b| b.timestamp > cutoff_time);

        // Clean up old performance markers
        tracing_context
            .performance_markers
            .retain(|m| m.timestamp > cutoff_time);
    }
}

// Helper trait for duration operations
#[allow(dead_code)]
trait DurationExtensions {
    fn from_hours(hours: u64) -> std::time::Duration;
}

impl DurationExtensions for std::time::Duration {
    fn from_hours(hours: u64) -> std::time::Duration {
        std::time::Duration::from_secs(hours * 3600)
    }
}
