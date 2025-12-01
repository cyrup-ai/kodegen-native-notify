//! Tests for components/mod.rs

use ecs_notifications::components::{
    NotificationId,
    CorrelationId,
    Priority,
    NotificationCategory,
    CategoryAction,
    ActionOptions,
    ActionIcon,
    TraceSpan,
};

#[test]
fn test_notification_id_generation() {
    let id1 = NotificationId::generate();
    let id2 = NotificationId::generate();
    assert_ne!(id1, id2);

    // Test string conversion roundtrip
    let id_str = id1.to_string();
    let parsed_id: NotificationId = id_str.parse().unwrap();
    assert_eq!(id1, parsed_id);
}

#[test]
fn test_correlation_id_generation() {
    let corr1 = CorrelationId::generate();
    let corr2 = CorrelationId::generate();
    assert_ne!(corr1, corr2);
}

#[test]
fn test_priority_ordering() {
    assert!(Priority::Urgent > Priority::Critical);
    assert!(Priority::Critical > Priority::High);
    assert!(Priority::High > Priority::Normal);
    assert!(Priority::Normal > Priority::Low);
}

#[test]
fn test_priority_dnd_bypass() {
    assert!(Priority::Critical.bypasses_dnd());
    assert!(Priority::Urgent.bypasses_dnd());
    assert!(!Priority::High.bypasses_dnd());
    assert!(!Priority::Normal.bypasses_dnd());
    assert!(!Priority::Low.bypasses_dnd());
}

#[test]
fn test_notification_category_builder() {
    let category = NotificationCategory::new("test", "Test Category")
        .with_description("Test description")
        .with_action(CategoryAction {
            identifier: "action1".to_string(),
            title: "Action 1".to_string(),
            options: ActionOptions {
                foreground: true,
                ..Default::default()
            },
            icon: Some(ActionIcon::System("accept".to_string())),
        });

    assert_eq!(category.identifier, "test");
    assert_eq!(category.display_name, "Test Category");
    assert_eq!(category.description, Some("Test description".to_string()));
    assert_eq!(category.actions.len(), 1);
}

#[test]
fn test_trace_span_creation() {
    let span = TraceSpan::new("test_operation")
        .with_attribute("service", "test")
        .with_attribute("version", "1.0.0");

    assert_eq!(span.operation_name, "test_operation");
    assert_eq!(span.attributes.get("service"), Some(&"test".to_string()));
    assert_eq!(span.attributes.get("version"), Some(&"1.0.0".to_string()));
}
