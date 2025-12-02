//! Tests for components/analytics.rs

use std::collections::HashMap;
use std::time::Duration;
use kodegen_native_notify::{
    NotificationAnalytics,
    NotificationId,
    CorrelationId,
    AnalyticsPerformanceMetrics as PerformanceMetrics,
    UserBehaviorMetrics,
    UserInteraction,
    InteractionType,
    InteractionOutcome,
    EngagementDepth,
    Platform,
};
use kodegen_native_notify::components::time_wrapper::DefaultableInstant;

#[test]
fn test_analytics_creation() {
    let notification_id = NotificationId::generate();
    let correlation_id = CorrelationId::generate();
    let analytics = NotificationAnalytics::new(notification_id, correlation_id);

    assert_eq!(analytics.performance_metrics.delivery_attempts, 0);
    assert_eq!(analytics.user_behavior.interactions.len(), 0);
    assert!(analytics.platform_analytics.is_empty());
}

#[test]
fn test_performance_metrics() {
    let mut metrics = PerformanceMetrics::new();

    metrics.record_delivery(Platform::MacOS, Duration::from_millis(100), true);
    metrics.record_delivery(Platform::MacOS, Duration::from_millis(200), false);

    assert_eq!(metrics.delivery_attempts, 2);
    assert_eq!(metrics.successful_deliveries, 1);
    assert_eq!(metrics.failed_deliveries, 1);
    assert_eq!(metrics.calculate_success_rate(), 0.5);
}

#[test]
fn test_user_behavior_tracking() {
    let mut behavior = UserBehaviorMetrics::new();

    let interaction = UserInteraction {
        interaction_type: InteractionType::Clicked,
        timestamp: DefaultableInstant::now(),
        platform: Platform::MacOS,
        response_time: Duration::from_secs(5),
        duration: Some(Duration::from_millis(500)),
        context: None,
        outcome: InteractionOutcome::Success,
        metadata: HashMap::new(),
    };

    behavior.record_interaction(interaction);

    assert_eq!(behavior.interactions.len(), 1);
    assert_eq!(behavior.engagement_depth, EngagementDepth::Clicked);
    assert_eq!(
        *behavior
            .interaction_types
            .get(&InteractionType::Clicked)
            .expect(
                "InteractionType::Clicked should be present in interaction_types after \
                 recording a clicked interaction"
            ),
        1
    );
}

#[test]
fn test_effectiveness_score() {
    let mut analytics =
        NotificationAnalytics::new(NotificationId::generate(), CorrelationId::generate());

    // Simulate successful delivery and interaction
    analytics.record_delivery_performance(Platform::MacOS, Duration::from_millis(100), true);

    let interaction = UserInteraction {
        interaction_type: InteractionType::ActionPressed,
        timestamp: DefaultableInstant::now(),
        platform: Platform::MacOS,
        response_time: Duration::from_secs(3),
        duration: Some(Duration::from_millis(1000)),
        context: None,
        outcome: InteractionOutcome::Success,
        metadata: HashMap::new(),
    };

    analytics.record_user_interaction(interaction);

    let score = analytics.calculate_effectiveness_score();
    assert!(score > 0.0);
    assert!(score <= 1.0);
}
