//! Tests for components/lifecycle.rs

use action_items_ecs_notifications::components::lifecycle::*;
use std::time::Duration;

#[test]
fn test_lifecycle_state_transitions() {
    let mut lifecycle = NotificationLifecycle::default();
    
    // Test initial state
    assert_eq!(lifecycle.state, LifecycleState::Pending);
    
    // Test transition to processing
    lifecycle.state = LifecycleState::Processing;
    assert_eq!(lifecycle.state, LifecycleState::Processing);
    
    // Test transition to delivered
    lifecycle.state = LifecycleState::Delivered;
    assert_eq!(lifecycle.state, LifecycleState::Delivered);
    
    // Test transition to failed
    lifecycle.state = LifecycleState::Failed;
    assert_eq!(lifecycle.state, LifecycleState::Failed);
}

#[test]
fn test_retry_policy_backoff() {
    let policy = RetryPolicy::default();
    
    // Test exponential backoff
    let first_delay = policy.next_retry_delay(0);
    let second_delay = policy.next_retry_delay(1);
    let third_delay = policy.next_retry_delay(2);
    
    assert!(second_delay > first_delay);
    assert!(third_delay > second_delay);
    
    // Test max retries
    assert!(policy.should_retry(2));
    assert!(!policy.should_retry(3));
}

#[test]
fn test_expiration_checking() {
    let mut lifecycle = NotificationLifecycle::default();
    lifecycle.ttl = Some(Duration::from_secs(60));
    
    // Should not be expired immediately
    assert!(!lifecycle.is_expired());
    
    // Test with past TTL
    lifecycle.created_at = std::time::SystemTime::now() - Duration::from_secs(120);
    assert!(lifecycle.is_expired());
}

#[test]
fn test_delivery_progress() {
    let mut lifecycle = NotificationLifecycle::default();
    
    // Test initial progress
    assert_eq!(lifecycle.delivery_progress.total, 0);
    assert_eq!(lifecycle.delivery_progress.successful, 0);
    assert_eq!(lifecycle.delivery_progress.failed, 0);
    
    // Test progress tracking
    lifecycle.delivery_progress.total = 3;
    lifecycle.delivery_progress.successful = 2;
    lifecycle.delivery_progress.failed = 1;
    
    assert_eq!(lifecycle.delivery_progress.total, 3);
    assert_eq!(lifecycle.delivery_progress.successful, 2);
    assert_eq!(lifecycle.delivery_progress.failed, 1);
}
