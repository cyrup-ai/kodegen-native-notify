//! Tests for components/content.rs

use ecs_notifications::components::{
    NotificationContent,
    RichText,
    Priority,
    NotificationAction,
    ActionId,
    ActionStyle,
    ActivationType,
};

#[test]
fn test_notification_content_builder() {
    let content = NotificationContent::new("Test Title", RichText::plain("Test body"))
        .with_subtitle("Test subtitle")
        .with_priority(Priority::High)
        .with_custom_data("key1", "value1");

    assert_eq!(content.title, "Test Title");
    assert_eq!(content.subtitle, Some("Test subtitle".to_string()));
    assert_eq!(content.priority, Priority::High);
    assert_eq!(content.custom_data.get("key1"), Some(&"value1".to_string()));
}

#[test]
fn test_rich_text_conversion() {
    let plain = RichText::plain("Hello world");
    assert_eq!(plain.to_plain_text(), "Hello world");

    let markdown = RichText::markdown("**Bold** and *italic*");
    let plain_from_md = markdown.to_plain_text();
    assert_eq!(plain_from_md, "Bold and italic");
}

#[test]
fn test_action_validation() {
    let valid_action = NotificationAction {
        id: ActionId::new("test"),
        label: "Test Action".to_string(),
        icon: None,
        style: ActionStyle::Default,
        activation_type: ActivationType::Foreground,
        url: None,
        payload: None,
        confirmation: None,
    };

    assert!(valid_action.validate().is_ok());

    let invalid_action = NotificationAction {
        label: "".to_string(),
        ..valid_action
    };

    assert!(invalid_action.validate().is_err());
}
