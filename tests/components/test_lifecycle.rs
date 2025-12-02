//! Tests for components/lifecycle.rs

use kodegen_native_notify::{
    NotificationLifecycle,
    TransitionReason,
    RetryPolicy,
    DeliveryProgress,
};
use kodegen_native_notify::components::lifecycle::NotificationState;
use std::time::Duration;

#[test]
fn test_lifecycle_state_transitions() {
    let mut lifecycle = NotificationLifecycle::default();

    // Test initial state
    assert_eq!(lifecycle.state, NotificationState::Created);

    // Test transition to Validating using proper transition_to method
    let result = lifecycle.transition_to(
        NotificationState::Validating,
        TransitionReason::Initial,
        None,
    );
    assert!(result.is_ok());
    assert_eq!(lifecycle.state, NotificationState::Validating);

    // Test transition to PlatformRouting
    let result = lifecycle.transition_to(
        NotificationState::PlatformRouting,
        TransitionReason::ValidationCompleted,
        None,
    );
    assert!(result.is_ok());
    assert_eq!(lifecycle.state, NotificationState::PlatformRouting);

    // Test transition to Queued
    let result = lifecycle.transition_to(
        NotificationState::Queued,
        TransitionReason::PlatformCapabilitiesResolved,
        None,
    );
    assert!(result.is_ok());
    assert_eq!(lifecycle.state, NotificationState::Queued);
}

#[test]
fn test_retry_policy_backoff() {
    let policy = RetryPolicy::default();

    // Test exponential backoff using calculate_next_delay
    let first_delay = policy.calculate_next_delay(0);
    let second_delay = policy.calculate_next_delay(1);
    let third_delay = policy.calculate_next_delay(2);

    assert!(second_delay >= first_delay);
    assert!(third_delay >= second_delay);
}

#[test]
fn test_expiration_checking() {
    let mut lifecycle = NotificationLifecycle::default();

    // Set TTL through expiration policy
    lifecycle.expiration.ttl = Some(Duration::from_secs(60));

    // Should not be expired immediately
    assert!(!lifecycle.is_expired());
}

#[test]
fn test_delivery_progress() {
    let lifecycle = NotificationLifecycle::default();

    // Test initial progress - should be NotStarted since state is Created
    let progress = lifecycle.delivery_progress();
    assert_eq!(progress, DeliveryProgress::NotStarted);
}

#[test]
fn test_should_retry() {
    let mut lifecycle = NotificationLifecycle::default();

    // In Created state, should_retry should be false
    assert!(!lifecycle.should_retry());

    // Transition to Failed state
    let error_details = kodegen_native_notify::LifecycleErrorDetails {
        error_type: kodegen_native_notify::LifecycleErrorType::NetworkError,
        message: "Test error".to_string(),
        retry_count: 0,
        last_attempt: None,
        platform_errors: std::collections::HashMap::new(),
    };

    // First need to go through valid transitions to get to a state that can fail
    let _ = lifecycle.transition_to(
        NotificationState::Validating,
        TransitionReason::Initial,
        None,
    );
    let _ = lifecycle.transition_to(
        NotificationState::Failed(error_details),
        TransitionReason::ValidationFailed,
        None,
    );

    // Now should_retry should be true (current_attempt < max_attempts)
    assert!(lifecycle.should_retry());
}

#[test]
fn test_state_history_tracking() {
    let mut lifecycle = NotificationLifecycle::default();

    // Initial state should be in history
    assert!(!lifecycle.state_history.is_empty());

    // Transition and verify history grows
    let _ = lifecycle.transition_to(
        NotificationState::Validating,
        TransitionReason::Initial,
        None,
    );

    assert!(lifecycle.state_history.len() >= 2);
}
