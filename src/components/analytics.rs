// Comprehensive analytics and observability for enterprise notification systems
// Based on extensive study of Slack's distributed tracing, Discord's real-time metrics,
// VS Code's user behavior analytics, and Teams' performance monitoring patterns

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::time_wrapper::DefaultableInstant;
use super::{CorrelationId, NotificationId, Platform, SpanId, TraceId};

/// Comprehensive notification analytics component for enterprise observability
/// Incorporates patterns from Slack's performance monitoring, Discord's user engagement tracking,
/// Teams' client data layer analytics, and production notification effectiveness measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAnalytics {
    /// Performance metrics for delivery and interaction
    pub performance_metrics: PerformanceMetrics,
    /// User behavior and engagement tracking
    pub user_behavior: UserBehaviorMetrics,
    /// Platform-specific performance data
    pub platform_analytics: HashMap<Platform, PlatformAnalytics>,
    /// A/B testing data and experiment tracking
    pub experiment_data: Option<ExperimentData>,
    /// Distributed tracing integration
    pub trace_data: DistributedTraceData,
    /// Business metrics and effectiveness
    pub business_metrics: BusinessMetrics,
    /// Error tracking and failure analysis
    pub error_analytics: ErrorAnalytics,
    /// Content effectiveness metrics
    pub content_metrics: ContentEffectivenessMetrics,
}

impl NotificationAnalytics {
    pub fn new(_notification_id: NotificationId, correlation_id: CorrelationId) -> Self {
        Self {
            performance_metrics: PerformanceMetrics::new(),
            user_behavior: UserBehaviorMetrics::new(),
            platform_analytics: HashMap::new(),
            experiment_data: None,
            trace_data: DistributedTraceData::new(correlation_id),
            business_metrics: BusinessMetrics::new(),
            error_analytics: ErrorAnalytics::new(),
            content_metrics: ContentEffectivenessMetrics::new(),
        }
    }

    /// Record delivery performance metrics
    pub fn record_delivery_performance(
        &mut self,
        platform: Platform,
        latency: Duration,
        success: bool,
    ) {
        self.performance_metrics
            .record_delivery(platform, latency, success);

        // Update platform-specific analytics
        let platform_analytics = self
            .platform_analytics
            .entry(platform)
            .or_insert_with(|| PlatformAnalytics::new(platform));
        platform_analytics.record_delivery(latency, success);

        // Update trace data
        self.trace_data
            .record_delivery_event(platform, latency, success);
    }

    /// Record user interaction with comprehensive behavior tracking
    pub fn record_user_interaction(&mut self, interaction: UserInteraction) {
        let response_time = interaction.timestamp.duration_since(
            self.performance_metrics
                .delivered_at
                .unwrap_or(interaction.timestamp),
        );

        self.user_behavior.record_interaction(interaction.clone());
        self.business_metrics.record_user_engagement(&interaction);
        self.content_metrics.record_interaction(&interaction);

        // Update performance metrics
        self.performance_metrics.user_response_time = Some(response_time);
        self.performance_metrics.interaction_count += 1;

        // Platform-specific interaction tracking
        if let Some(platform_analytics) = self.platform_analytics.get_mut(&interaction.platform) {
            platform_analytics.record_interaction(&interaction);
        }
    }

    /// Record error for comprehensive failure analysis
    pub fn record_error(&mut self, error: AnalyticsError) {
        self.error_analytics.record_error(error.clone());
        self.performance_metrics.error_count += 1;

        // Platform-specific error tracking
        if let Some(platform_analytics) = self.platform_analytics.get_mut(&error.platform) {
            platform_analytics.record_error(&error);
        }

        // Update trace data with error
        self.trace_data.record_error(&error);
    }

    /// Set A/B testing experiment data
    pub fn set_experiment(&mut self, experiment: ExperimentData) {
        self.experiment_data = Some(experiment);
    }

    /// Record content effectiveness metrics
    pub fn record_content_effectiveness(&mut self, effectiveness: ContentEffectiveness) {
        self.content_metrics.record_effectiveness(effectiveness);
    }

    /// Record effectiveness calculation result
    pub fn record_effectiveness_calculation(&mut self, score: f64) {
        self.content_metrics.record_effectiveness_score(score);
    }

    /// Calculate overall notification effectiveness score
    pub fn calculate_effectiveness_score(&self) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // Delivery success rate (30% of score)
        if self.performance_metrics.delivery_attempts > 0 {
            let delivery_rate = self.performance_metrics.successful_deliveries as f64
                / self.performance_metrics.delivery_attempts as f64;
            score += delivery_rate * 0.3;
            factors += 1;
        }

        // User engagement rate (40% of score)
        if self.performance_metrics.delivery_attempts > 0 {
            let engagement_rate = self.performance_metrics.interaction_count as f64
                / self.performance_metrics.delivery_attempts as f64;
            score += (engagement_rate.min(1.0)) * 0.4;
            factors += 1;
        }

        // Response time quality (20% of score)
        if let Some(response_time) = self.performance_metrics.user_response_time {
            let time_score = if response_time < Duration::from_secs(5) {
                1.0 // Excellent response time
            } else if response_time < Duration::from_secs(30) {
                0.7 // Good response time
            } else if response_time < Duration::from_secs(5 * 60) {
                0.5 // Moderate response time
            } else {
                0.2 // Slow response time
            };
            score += time_score * 0.2;
            factors += 1;
        }

        // Error rate penalty (10% of score)
        if self.performance_metrics.delivery_attempts > 0 {
            let error_rate = self.performance_metrics.error_count as f64
                / self.performance_metrics.delivery_attempts as f64;
            score += (1.0 - error_rate.min(1.0)) * 0.1;
            factors += 1;
        }

        if factors > 0 { score } else { 0.0 }
    }

    /// Update all internal metrics and analytics
    pub fn update_metrics(&mut self) {
        // Update platform-specific metrics
        for (_platform, platform_analytics) in self.platform_analytics.iter_mut() {
            platform_analytics.update_metrics();
        }

        // Update performance metrics
        self.performance_metrics.update_derived_metrics();

        // Update business metrics based on current data
        self.business_metrics
            .update_from_performance(&self.performance_metrics);

        // Update content effectiveness metrics
        self.content_metrics.update_effectiveness_scores();
    }

    /// Get comprehensive analytics summary
    pub fn get_analytics_summary(&self) -> AnalyticsSummary {
        AnalyticsSummary {
            effectiveness_score: self.calculate_effectiveness_score(),
            total_deliveries: self.performance_metrics.delivery_attempts,
            successful_deliveries: self.performance_metrics.successful_deliveries,
            user_interactions: self.performance_metrics.interaction_count,
            average_response_time: self.performance_metrics.user_response_time,
            error_count: self.performance_metrics.error_count,
            platform_performance: self
                .platform_analytics
                .iter()
                .map(|(platform, analytics)| (*platform, analytics.get_summary()))
                .collect(),
            business_impact: self.business_metrics.calculate_impact(),
        }
    }
}

impl Default for NotificationAnalytics {
    fn default() -> Self {
        Self::new(NotificationId::generate(), CorrelationId::generate())
    }
}

/// Performance metrics for notification delivery and interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceMetrics {
    /// Delivery performance
    pub delivery_attempts: u32,
    pub successful_deliveries: u32,
    pub failed_deliveries: u32,
    pub average_delivery_latency: Option<Duration>,
    pub delivery_latency_p95: Option<Duration>,
    pub delivery_latency_p99: Option<Duration>,

    /// User interaction performance
    pub interaction_count: u32,
    pub user_response_time: Option<Duration>,
    pub first_interaction_time: Option<Duration>,

    /// System performance
    pub processing_time: Option<Duration>,
    pub queue_time: Option<Duration>,
    pub validation_time: Option<Duration>,

    /// Timing milestones
    #[serde(skip)]
    pub created_at: DefaultableInstant,
    #[serde(skip)]
    pub validated_at: Option<DefaultableInstant>,
    #[serde(skip)]
    pub queued_at: Option<DefaultableInstant>,
    #[serde(skip)]
    pub delivered_at: Option<DefaultableInstant>,
    #[serde(skip)]
    pub first_interaction_at: Option<DefaultableInstant>,
    #[serde(skip)]
    pub completed_at: Option<DefaultableInstant>,

    /// Error tracking
    pub error_count: u32,
    pub retry_count: u32,
    pub timeout_count: u32,

    /// Delivery latency history for percentile calculation
    pub latency_history: VecDeque<Duration>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            delivery_attempts: 0,
            successful_deliveries: 0,
            failed_deliveries: 0,
            average_delivery_latency: None,
            delivery_latency_p95: None,
            delivery_latency_p99: None,
            interaction_count: 0,
            user_response_time: None,
            first_interaction_time: None,
            processing_time: None,
            queue_time: None,
            validation_time: None,
            created_at: DefaultableInstant::now(),
            validated_at: None,
            queued_at: None,
            delivered_at: None,
            first_interaction_at: None,
            completed_at: None,
            error_count: 0,
            retry_count: 0,
            timeout_count: 0,
            latency_history: VecDeque::new(),
        }
    }

    pub fn record_delivery(&mut self, _platform: Platform, latency: Duration, success: bool) {
        self.delivery_attempts += 1;

        if success {
            self.successful_deliveries += 1;
            self.delivered_at = Some(DefaultableInstant::now());
        } else {
            self.failed_deliveries += 1;
        }

        // Update latency tracking
        self.latency_history.push_back(latency);
        if self.latency_history.len() > 100 {
            self.latency_history.pop_front();
        }

        // Calculate percentiles
        self.update_latency_percentiles();
    }

    fn update_latency_percentiles(&mut self) {
        if self.latency_history.is_empty() {
            return;
        }

        let mut sorted: Vec<_> = self.latency_history.iter().copied().collect();
        sorted.sort();

        let len = sorted.len();
        self.average_delivery_latency = Some(Duration::from_nanos(
            (sorted.iter().map(|d| d.as_nanos()).sum::<u128>() / len as u128) as u64,
        ));

        if len > 1 {
            let p95_index = (len as f64 * 0.95) as usize;
            self.delivery_latency_p95 = Some(sorted[p95_index.min(len - 1)]);

            let p99_index = (len as f64 * 0.99) as usize;
            self.delivery_latency_p99 = Some(sorted[p99_index.min(len - 1)]);
        }
    }

    pub fn calculate_success_rate(&self) -> f64 {
        if self.delivery_attempts == 0 {
            0.0
        } else {
            self.successful_deliveries as f64 / self.delivery_attempts as f64
        }
    }

    pub fn update_derived_metrics(&mut self) {
        // Update percentiles from current latency history
        self.update_latency_percentiles();

        // Calculate processing time if we have timing data
        if let (Some(delivered_at), Some(queued_at)) = (self.delivered_at, self.queued_at) {
            self.processing_time = Some(delivered_at.duration_since(queued_at));
        }

        // Calculate queue time if we have timing data
        if let (Some(queued_at), Some(validated_at)) = (self.queued_at, self.validated_at) {
            self.queue_time = Some(queued_at.duration_since(validated_at));
        }
    }
}

/// User behavior and engagement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehaviorMetrics {
    /// Interaction patterns
    pub interactions: Vec<UserInteraction>,
    pub interaction_types: HashMap<InteractionType, u32>,
    pub interaction_sequence: Vec<InteractionSequenceEvent>,

    /// Engagement quality
    pub attention_score: f64,
    pub engagement_depth: EngagementDepth,
    pub user_intent_signals: Vec<IntentSignal>,

    /// Context tracking
    pub device_context: Option<DeviceContext>,
    pub user_context: Option<UserContext>,
    pub attention_context: Option<AttentionContext>,

    /// Learning signals for personalization
    pub preference_signals: Vec<PreferenceSignal>,
    pub behavior_patterns: Vec<BehaviorPattern>,

    /// Timing analysis
    pub time_to_first_interaction: Option<Duration>,
    pub total_interaction_time: Duration,
    pub interaction_frequency: f64,
}

impl Default for UserBehaviorMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl UserBehaviorMetrics {
    pub fn new() -> Self {
        Self {
            interactions: Vec::new(),
            interaction_types: HashMap::new(),
            interaction_sequence: Vec::new(),
            attention_score: 0.0,
            engagement_depth: EngagementDepth::None,
            user_intent_signals: Vec::new(),
            device_context: None,
            user_context: None,
            attention_context: None,
            preference_signals: Vec::new(),
            behavior_patterns: Vec::new(),
            time_to_first_interaction: None,
            total_interaction_time: Duration::ZERO,
            interaction_frequency: 0.0,
        }
    }

    pub fn record_interaction(&mut self, interaction: UserInteraction) {
        // First interaction tracking
        if self.interactions.is_empty() {
            self.time_to_first_interaction = Some(interaction.response_time);
        }

        // Update interaction type counts
        *self
            .interaction_types
            .entry(interaction.interaction_type)
            .or_insert(0) += 1;

        // Add to sequence
        self.interaction_sequence.push(InteractionSequenceEvent {
            timestamp: interaction.timestamp,
            interaction_type: interaction.interaction_type,
            context: interaction.context.clone(),
        });

        // Update engagement depth
        self.update_engagement_depth(&interaction);

        // Record preference signals
        if let Some(preference) = self.extract_preference_signal(&interaction) {
            self.preference_signals.push(preference);
        }

        // Update total interaction time
        self.total_interaction_time += interaction.duration.unwrap_or(Duration::ZERO);

        // Store interaction
        self.interactions.push(interaction);

        // Calculate attention score
        self.calculate_attention_score();
    }

    fn update_engagement_depth(&mut self, interaction: &UserInteraction) {
        let new_depth = match interaction.interaction_type {
            InteractionType::Viewed => EngagementDepth::Viewed,
            InteractionType::Clicked => EngagementDepth::Clicked,
            InteractionType::ActionPressed => EngagementDepth::Interacted,
            InteractionType::InputSubmitted => EngagementDepth::Engaged,
            InteractionType::SharedContent => EngagementDepth::Advocated,
            _ => self.engagement_depth,
        };

        // Only increase engagement depth
        if new_depth as u8 > self.engagement_depth as u8 {
            self.engagement_depth = new_depth;
        }
    }

    fn extract_preference_signal(&self, interaction: &UserInteraction) -> Option<PreferenceSignal> {
        match interaction.interaction_type {
            InteractionType::Dismissed => Some(PreferenceSignal {
                signal_type: PreferenceType::ContentRelevance,
                strength: -0.3,
                context: interaction.context.clone(),
                timestamp: interaction.timestamp,
            }),
            InteractionType::ActionPressed => Some(PreferenceSignal {
                signal_type: PreferenceType::ActionPreference,
                strength: 0.8,
                context: interaction.context.clone(),
                timestamp: interaction.timestamp,
            }),
            _ => None,
        }
    }

    fn calculate_attention_score(&mut self) {
        let mut score = 0.0;

        // Base score from engagement depth
        score += match self.engagement_depth {
            EngagementDepth::None => 0.0,
            EngagementDepth::Delivered => 0.1,
            EngagementDepth::Viewed => 0.3,
            EngagementDepth::Clicked => 0.5,
            EngagementDepth::Interacted => 0.7,
            EngagementDepth::Engaged => 0.9,
            EngagementDepth::Advocated => 1.0,
        };

        // Bonus for quick response
        if let Some(first_response) = self.time_to_first_interaction {
            if first_response < Duration::from_secs(5) {
                score += 0.2;
            } else if first_response < Duration::from_secs(30) {
                score += 0.1;
            }
        }

        // Bonus for multiple interactions
        if self.interactions.len() > 1 {
            score += (self.interactions.len() as f64 * 0.1).min(0.3);
        }

        self.attention_score = score.min(1.0);
    }
}

/// User interaction event with comprehensive context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UserInteraction {
    pub interaction_type: InteractionType,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub platform: Platform,
    pub response_time: Duration,
    pub duration: Option<Duration>,
    pub context: Option<InteractionContext>,
    pub outcome: InteractionOutcome,
    pub metadata: HashMap<String, String>,
}

impl Default for UserInteraction {
    fn default() -> Self {
        Self {
            interaction_type: InteractionType::default(),
            timestamp: DefaultableInstant::now(),
            platform: Platform::default(),
            response_time: Duration::default(),
            duration: None,
            context: None,
            outcome: InteractionOutcome::default(),
            metadata: HashMap::default(),
        }
    }
}

/// Types of user interactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum InteractionType {
    #[default]
    Delivered,
    Viewed,
    Clicked,
    ActionPressed,
    InputSubmitted,
    MenuSelected,
    Dismissed,
    Closed,
    SharedContent,
    Copied,
    Saved,
    Forwarded,
}


/// Interaction outcome tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum InteractionOutcome {
    #[default]
    Success,
    Partial,
    Failed { reason: String },
    Abandoned,
}


/// Interaction context for behavior analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionContext {
    pub device_type: Option<String>,
    pub app_state: Option<String>,
    pub user_active: bool,
    pub notification_position: Option<usize>,
    pub concurrent_notifications: u32,
    pub time_since_last_notification: Option<Duration>,
}

/// Engagement depth levels (orderable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EngagementDepth {
    None = 0,
    Delivered = 1,
    Viewed = 2,
    Clicked = 3,
    Interacted = 4,
    Engaged = 5,
    Advocated = 6,
}

/// Platform-specific analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAnalytics {
    pub platform: Platform,
    pub delivery_success_rate: f64,
    pub average_delivery_latency: Option<Duration>,
    pub user_engagement_rate: f64,
    pub error_rate: f64,
    pub feature_usage: HashMap<String, u32>,
    pub performance_history: VecDeque<PlatformPerformancePoint>,
}

impl PlatformAnalytics {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            delivery_success_rate: 0.0,
            average_delivery_latency: None,
            user_engagement_rate: 0.0,
            error_rate: 0.0,
            feature_usage: HashMap::new(),
            performance_history: VecDeque::new(),
        }
    }

    pub fn record_delivery(&mut self, latency: Duration, success: bool) {
        let point = PlatformPerformancePoint {
            timestamp: DefaultableInstant::now(),
            latency,
            success,
            error: None,
        };

        self.performance_history.push_back(point);
        if self.performance_history.len() > 1000 {
            self.performance_history.pop_front();
        }

        self.update_metrics();
    }

    pub fn record_interaction(&mut self, interaction: &UserInteraction) {
        // Track feature usage
        let feature = format!("{:?}", interaction.interaction_type);
        *self.feature_usage.entry(feature).or_insert(0) += 1;

        self.update_metrics();
    }

    pub fn record_error(&mut self, error: &AnalyticsError) {
        let point = PlatformPerformancePoint {
            timestamp: DefaultableInstant::now(),
            latency: Duration::ZERO,
            success: false,
            error: Some(error.error_type.clone()),
        };

        self.performance_history.push_back(point);
        self.update_metrics();
    }

    pub fn update_metrics(&mut self) {
        if self.performance_history.is_empty() {
            return;
        }

        let total_points = self.performance_history.len();
        let successful_points = self
            .performance_history
            .iter()
            .filter(|p| p.success)
            .count();
        let error_points = self
            .performance_history
            .iter()
            .filter(|p| p.error.is_some())
            .count();

        self.delivery_success_rate = successful_points as f64 / total_points as f64;
        self.error_rate = error_points as f64 / total_points as f64;

        let successful_latencies: Vec<_> = self
            .performance_history
            .iter()
            .filter(|p| p.success)
            .map(|p| p.latency)
            .collect();

        if !successful_latencies.is_empty() {
            let total_latency: Duration = successful_latencies.iter().sum();
            self.average_delivery_latency = Some(total_latency / successful_latencies.len() as u32);
        }
    }

    pub fn get_summary(&self) -> PlatformSummary {
        PlatformSummary {
            success_rate: self.delivery_success_rate,
            average_latency: self.average_delivery_latency,
            engagement_rate: self.user_engagement_rate,
            error_rate: self.error_rate,
            total_interactions: self.feature_usage.values().sum(),
        }
    }
}

/// Platform performance data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatformPerformancePoint {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub latency: Duration,
    pub success: bool,
    pub error: Option<ErrorType>,
}

impl Default for PlatformPerformancePoint {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            latency: Duration::default(),
            success: true,
            error: None,
        }
    }
}

/// A/B testing and experimentation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentData {
    pub experiment_id: String,
    pub variant_id: String,
    pub experiment_type: ExperimentType,
    pub assignment_timestamp: SystemTime,
    pub control_group: bool,
    pub experiment_metadata: HashMap<String, String>,
}

/// Types of experiments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperimentType {
    ContentVariant,
    StyleVariant,
    TimingVariant,
    PlatformVariant,
    InteractionVariant,
    Custom(String),
}

/// Distributed tracing data integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTraceData {
    pub correlation_id: CorrelationId,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub service_hops: Vec<ServiceHop>,
    pub trace_events: Vec<TraceEvent>,
    pub trace_attributes: HashMap<String, String>,
}

impl DistributedTraceData {
    pub fn new(correlation_id: CorrelationId) -> Self {
        Self {
            correlation_id,
            trace_id: TraceId::generate(),
            span_id: SpanId::generate(),
            parent_span_id: None,
            service_hops: Vec::new(),
            trace_events: Vec::new(),
            trace_attributes: HashMap::new(),
        }
    }

    pub fn record_delivery_event(&mut self, platform: Platform, latency: Duration, success: bool) {
        let event = TraceEvent {
            timestamp: DefaultableInstant::now(),
            event_type: TraceEventType::Delivery,
            platform: Some(platform),
            latency: Some(latency),
            success: Some(success),
            metadata: HashMap::new(),
        };

        self.trace_events.push(event);
    }

    pub fn record_error(&mut self, error: &AnalyticsError) {
        let mut metadata = HashMap::new();
        metadata.insert("error_message".to_string(), error.message.clone());

        let event = TraceEvent {
            timestamp: DefaultableInstant::now(),
            event_type: TraceEventType::Error,
            platform: Some(error.platform),
            latency: None,
            success: Some(false),
            metadata,
        };

        self.trace_events.push(event);
    }
}

/// Service hop in distributed trace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceHop {
    pub service_name: String,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub duration: Duration,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

impl Default for ServiceHop {
    fn default() -> Self {
        Self {
            service_name: String::default(),
            timestamp: DefaultableInstant::now(),
            duration: Duration::default(),
            success: true,
            metadata: HashMap::default(),
        }
    }
}

/// Trace event for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TraceEvent {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub event_type: TraceEventType,
    pub platform: Option<Platform>,
    pub latency: Option<Duration>,
    pub success: Option<bool>,
    pub metadata: HashMap<String, String>,
}

impl Default for TraceEvent {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            event_type: TraceEventType::default(),
            platform: None,
            latency: None,
            success: None,
            metadata: HashMap::default(),
        }
    }
}

/// Types of trace events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum TraceEventType {
    #[default]
    Creation,
    Validation,
    PlatformRouting,
    Delivery,
    Interaction,
    Error,
    Completion,
}


/// Business metrics for ROI and effectiveness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessMetrics {
    pub conversion_events: Vec<ConversionEvent>,
    pub user_journey_stage: JourneyStage,
    pub business_value: f64,
    pub cost_metrics: CostMetrics,
    pub retention_impact: RetentionMetrics,
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl BusinessMetrics {
    pub fn new() -> Self {
        Self {
            conversion_events: Vec::new(),
            user_journey_stage: JourneyStage::Awareness,
            business_value: 0.0,
            cost_metrics: CostMetrics::default(),
            retention_impact: RetentionMetrics::default(),
        }
    }

    pub fn record_user_engagement(&mut self, interaction: &UserInteraction) {
        // Convert interactions to business events
        match interaction.interaction_type {
            InteractionType::ActionPressed => {
                self.conversion_events.push(ConversionEvent {
                    event_type: ConversionType::Engagement,
                    value: 1.0,
                    timestamp: interaction.timestamp,
                    metadata: interaction.metadata.clone(),
                });
            },
            InteractionType::SharedContent => {
                self.conversion_events.push(ConversionEvent {
                    event_type: ConversionType::Referral,
                    value: 5.0, // Higher value for sharing
                    timestamp: interaction.timestamp,
                    metadata: interaction.metadata.clone(),
                });
            },
            _ => {},
        }

        self.update_business_value();
    }

    fn update_business_value(&mut self) {
        self.business_value = self.conversion_events.iter().map(|event| event.value).sum();
    }

    pub fn calculate_impact(&self) -> BusinessImpact {
        BusinessImpact {
            total_value: self.business_value,
            conversion_rate: self.calculate_conversion_rate(),
            cost_per_conversion: self.calculate_cost_per_conversion(),
            roi: self.calculate_roi(),
        }
    }

    fn calculate_conversion_rate(&self) -> f64 {
        if self.conversion_events.is_empty() {
            0.0
        } else {
            self.conversion_events.len() as f64 // Simplified calculation
        }
    }

    fn calculate_cost_per_conversion(&self) -> f64 {
        if self.conversion_events.is_empty() {
            0.0
        } else {
            self.cost_metrics.total_cost / self.conversion_events.len() as f64
        }
    }

    fn calculate_roi(&self) -> f64 {
        if self.cost_metrics.total_cost == 0.0 {
            0.0
        } else {
            (self.business_value - self.cost_metrics.total_cost) / self.cost_metrics.total_cost
        }
    }

    pub fn update_from_performance(&mut self, performance: &PerformanceMetrics) {
        // Update cost metrics based on performance data
        let delivery_cost = performance.delivery_attempts as f64 * 0.01; // $0.01 per delivery attempt
        let retry_cost = performance.retry_count as f64 * 0.005; // $0.005 per retry
        let error_cost = performance.error_count as f64 * 0.02; // $0.02 per error

        self.cost_metrics.total_cost = delivery_cost + retry_cost + error_cost;

        // Update cost per delivery if we have successful deliveries
        if performance.successful_deliveries > 0 {
            self.cost_metrics.cost_per_delivery =
                self.cost_metrics.total_cost / performance.successful_deliveries as f64;
        }

        // Update business value based on successful deliveries
        if performance.successful_deliveries > 0 {
            self.business_value += performance.successful_deliveries as f64 * 0.1; // $0.10 value per successful delivery
        }
    }
}

/// Conversion events for business tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConversionEvent {
    pub event_type: ConversionType,
    pub value: f64,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub metadata: HashMap<String, String>,
}

impl Default for ConversionEvent {
    fn default() -> Self {
        Self {
            event_type: ConversionType::default(),
            value: 0.0,
            timestamp: DefaultableInstant::now(),
            metadata: HashMap::default(),
        }
    }
}

/// Types of conversion events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum ConversionType {
    #[default]
    View,
    Click,
    Engagement,
    Purchase,
    Signup,
    Referral,
    Custom(String),
}


/// User journey stages
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JourneyStage {
    Awareness,
    Interest,
    Consideration,
    Intent,
    Evaluation,
    Purchase,
    Advocacy,
}

/// Cost tracking metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostMetrics {
    pub total_cost: f64,
    pub cost_per_delivery: f64,
    pub platform_costs: HashMap<Platform, f64>,
}

/// Retention impact metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionMetrics {
    pub retention_score: f64,
    pub churn_risk: f64,
    pub engagement_trend: f64,
}

/// Error analytics for failure analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorAnalytics {
    pub errors: Vec<AnalyticsError>,
    pub error_patterns: HashMap<ErrorType, u32>,
    pub resolution_times: HashMap<ErrorType, Duration>,
    pub error_trends: Vec<ErrorTrendPoint>,
}

impl Default for ErrorAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorAnalytics {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            error_patterns: HashMap::new(),
            resolution_times: HashMap::new(),
            error_trends: Vec::new(),
        }
    }

    pub fn record_error(&mut self, error: AnalyticsError) {
        *self
            .error_patterns
            .entry(error.error_type.clone())
            .or_insert(0) += 1;

        self.error_trends.push(ErrorTrendPoint {
            timestamp: DefaultableInstant::now(),
            error_type: error.error_type.clone(),
            count: 1,
        });

        self.errors.push(error);
    }

    pub fn get_error_rate(&self, time_window: Duration) -> f64 {
        let cutoff = DefaultableInstant::now() - time_window;
        let recent_errors = self.errors.iter().filter(|e| e.timestamp > cutoff).count();

        recent_errors as f64 / self.errors.len().max(1) as f64
    }
}

/// Analytics error tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnalyticsError {
    pub error_type: ErrorType,
    pub platform: Platform,
    pub message: String,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub context: HashMap<String, String>,
}

impl Default for AnalyticsError {
    fn default() -> Self {
        Self {
            error_type: ErrorType::default(),
            platform: Platform::default(),
            message: String::default(),
            timestamp: DefaultableInstant::now(),
            context: HashMap::default(),
        }
    }
}

/// Error types for analytics
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum ErrorType {
    #[default]
    DeliveryFailure,
    ValidationError,
    AuthorizationError,
    NetworkError,
    PlatformError,
    TimeoutError,
    ConfigurationError,
    SystemError,
}


/// Error trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorTrendPoint {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub error_type: ErrorType,
    pub count: u32,
}

impl Default for ErrorTrendPoint {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            error_type: ErrorType::default(),
            count: 0,
        }
    }
}

/// Content effectiveness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEffectivenessMetrics {
    pub content_performance: HashMap<String, ContentPerformance>,
    pub a_b_test_results: Vec<ABTestResult>,
    pub content_variations: Vec<ContentVariation>,
}

impl Default for ContentEffectivenessMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentEffectivenessMetrics {
    pub fn new() -> Self {
        Self {
            content_performance: HashMap::new(),
            a_b_test_results: Vec::new(),
            content_variations: Vec::new(),
        }
    }

    pub fn record_interaction(&mut self, interaction: &UserInteraction) {
        // Track content performance by type/category
        let content_key = format!("{:?}", interaction.interaction_type);
        let performance = self
            .content_performance
            .entry(content_key)
            .or_default();

        performance.interaction_count += 1;
        performance.last_interaction = Some(interaction.timestamp);

        if matches!(interaction.outcome, InteractionOutcome::Success) {
            performance.success_count += 1;
        }

        performance.update_effectiveness_score();
    }

    pub fn record_effectiveness(&mut self, effectiveness: ContentEffectiveness) {
        let performance = self
            .content_performance
            .entry(effectiveness.content_id.clone())
            .or_default();
        performance.effectiveness_score = effectiveness.score;
        performance.feedback_count += 1;
    }

    pub fn record_effectiveness_score(&mut self, score: f64) {
        // Record overall effectiveness score across all content
        for performance in self.content_performance.values_mut() {
            performance.effectiveness_score = (performance.effectiveness_score + score) / 2.0;
        }
    }

    pub fn update_effectiveness_scores(&mut self) {
        // Update effectiveness scores for all content based on current data
        for performance in self.content_performance.values_mut() {
            performance.update_effectiveness_score();
        }
    }
}

/// Content performance tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentPerformance {
    pub interaction_count: u32,
    pub success_count: u32,
    pub effectiveness_score: f64,
    pub feedback_count: u32,
    #[serde(skip)]
    pub last_interaction: Option<DefaultableInstant>,
}

impl ContentPerformance {
    fn update_effectiveness_score(&mut self) {
        if self.interaction_count > 0 {
            self.effectiveness_score = self.success_count as f64 / self.interaction_count as f64;
        }
    }
}

/// Content effectiveness measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentEffectiveness {
    pub content_id: String,
    pub score: f64,
    pub feedback_type: FeedbackType,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
}

impl Default for ContentEffectiveness {
    fn default() -> Self {
        Self {
            content_id: String::default(),
            score: 0.0,
            feedback_type: FeedbackType::default(),
            timestamp: DefaultableInstant::now(),
        }
    }
}

/// Types of content feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum FeedbackType {
    Positive,
    Negative,
    #[default]
    Neutral,
    Engagement,
    Conversion,
}


// Supporting types for comprehensive analytics

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InteractionSequenceEvent {
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
    pub interaction_type: InteractionType,
    pub context: Option<InteractionContext>,
}

impl Default for InteractionSequenceEvent {
    fn default() -> Self {
        Self {
            timestamp: DefaultableInstant::now(),
            interaction_type: InteractionType::default(),
            context: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IntentSignal {
    pub intent_type: IntentType,
    pub confidence: f64,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
}

impl Default for IntentSignal {
    fn default() -> Self {
        Self {
            intent_type: IntentType::default(),
            confidence: 0.0,
            timestamp: DefaultableInstant::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum IntentType {
    Interested,
    Dismissive,
    Urgent,
    #[default]
    Informational,
    Actionable,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceContext {
    pub device_type: String,
    pub screen_size: Option<(u32, u32)>,
    pub battery_level: Option<f32>,
    pub network_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub session_id: String,
    pub user_segment: Option<String>,
    pub preferences: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionContext {
    pub focus_state: FocusState,
    pub concurrent_notifications: u32,
    pub notification_fatigue_score: f64,
    pub time_of_day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum FocusState {
    Focused,
    Distracted,
    DoNotDisturb,
    Away,
    #[default]
    Unknown,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PreferenceSignal {
    pub signal_type: PreferenceType,
    pub strength: f64, // -1.0 to 1.0
    pub context: Option<InteractionContext>,
    #[serde(skip)]
    pub timestamp: DefaultableInstant,
}

impl Default for PreferenceSignal {
    fn default() -> Self {
        Self {
            signal_type: PreferenceType::default(),
            strength: 0.0,
            context: None,
            timestamp: DefaultableInstant::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum PreferenceType {
    #[default]
    ContentRelevance,
    TimingPreference,
    PlatformPreference,
    ActionPreference,
    StylePreference,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_type: PatternType,
    pub frequency: f64,
    pub confidence: f64,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    TimeBasedEngagement,
    ContentTypePreference,
    InteractionStyle,
    ResponseLatency,
    PlatformUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub experiment_id: String,
    pub variant_id: String,
    pub metric_name: String,
    pub value: f64,
    pub confidence_interval: (f64, f64),
    pub statistical_significance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentVariation {
    pub variation_id: String,
    pub content_type: String,
    pub performance_score: f64,
    pub sample_size: u32,
}

/// Comprehensive analytics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub effectiveness_score: f64,
    pub total_deliveries: u32,
    pub successful_deliveries: u32,
    pub user_interactions: u32,
    pub average_response_time: Option<Duration>,
    pub error_count: u32,
    pub platform_performance: HashMap<Platform, PlatformSummary>,
    pub business_impact: BusinessImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSummary {
    pub success_rate: f64,
    pub average_latency: Option<Duration>,
    pub engagement_rate: f64,
    pub error_rate: f64,
    pub total_interactions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessImpact {
    pub total_value: f64,
    pub conversion_rate: f64,
    pub cost_per_conversion: f64,
    pub roi: f64,
}
