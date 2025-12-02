use kodegen_native_notify::*;
use kodegen_native_notify::components::lifecycle::NotificationState;

#[test]
fn test_enterprise_notification_system_integration() {
    // Test basic notification creation with the builder pattern
    let notification = NotificationBuilder::new()
        .with_title("Test Notification")
        .with_body(RichText::plain(
            "This is a test notification from the ECS system",
        ))
        .with_priority(Priority::High)
        .with_platforms(vec![Platform::MacOS, Platform::Windows, Platform::Linux])
        .build()
        .expect("Valid notification should build successfully");

    // Verify the notification bundle components
    assert_eq!(notification.content.title, "Test Notification");
    assert_eq!(notification.content.priority, Priority::High);
    assert_eq!(notification.platform_integration.target_platforms.len(), 3);
    assert!(
        notification
            .platform_integration
            .target_platforms
            .contains(&Platform::MacOS)
    );
    assert!(
        notification
            .platform_integration
            .target_platforms
            .contains(&Platform::Windows)
    );
    assert!(
        notification
            .platform_integration
            .target_platforms
            .contains(&Platform::Linux)
    );

    // Verify lifecycle state
    assert_eq!(notification.lifecycle.state, NotificationState::Created);

    // Test platform capabilities
    let macos_caps = Platform::MacOS.default_capabilities();
    assert!(macos_caps.supports_actions);
    assert!(macos_caps.supports_rich_media);
    assert!(macos_caps.authorization_required);

    let windows_caps = Platform::Windows.default_capabilities();
    assert!(windows_caps.supports_markup);
    assert!(windows_caps.supports_progress);

    println!("✅ Enterprise ECS notification system integration test passed!");
}

#[test]
fn test_notification_lifecycle_state_machine() {
    let mut lifecycle = NotificationLifecycle::new();

    // Test initial state
    assert_eq!(lifecycle.state, NotificationState::Created);

    // Test valid state transition
    let result = lifecycle.transition_to(
        NotificationState::Validating,
        TransitionReason::Initial,
        None,
    );
    assert!(result.is_ok());
    assert_eq!(lifecycle.state, NotificationState::Validating);

    // Test invalid state transition
    let invalid_result = lifecycle.transition_to(
        NotificationState::Completed,
        TransitionReason::SystemEvent,
        None,
    );
    assert!(invalid_result.is_err());

    // Verify state history tracking
    assert_eq!(lifecycle.state_history.len(), 2); // Initial + one transition

    println!("✅ Notification lifecycle state machine test passed!");
}

#[test]
fn test_platform_feature_matrix() {
    let mut capabilities = std::collections::HashMap::new();
    capabilities.insert(Platform::MacOS, Platform::MacOS.default_capabilities());
    capabilities.insert(Platform::Windows, Platform::Windows.default_capabilities());
    capabilities.insert(Platform::Linux, Platform::Linux.default_capabilities());

    let feature_matrix = FeatureMatrix::from_capabilities(&capabilities);

    // Test feature support detection
    assert!(feature_matrix.is_supported("actions"));
    assert!(feature_matrix.is_supported("rich_media"));

    // Test best platform selection
    assert!(
        feature_matrix
            .best_platform_for_feature("actions")
            .is_some()
    );

    println!("✅ Platform feature matrix test passed!");
}

#[test]
fn test_builder_validation_missing_content() {
    // Should fail: no content
    let result = NotificationBuilder::new().build();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        NotificationBuildError::MissingContent
    ));

    println!("✅ Builder validation (missing content) test passed!");
}

#[test]
fn test_builder_validation_empty_title() {
    // Should fail: empty title
    let result = NotificationBuilder::new()
        .with_title("")
        .with_body(RichText::plain("Body"))
        .build();

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        NotificationBuildError::MissingTitle
    ));

    println!("✅ Builder validation (empty title) test passed!");
}

#[test]
fn test_builder_validation_valid_content() {
    // Should succeed: valid content
    let result = NotificationBuilder::new()
        .with_title("Test")
        .with_body(RichText::plain("Body"))
        .build();

    assert!(result.is_ok());
    let notification = result.unwrap();
    assert_eq!(notification.content.title, "Test");

    println!("✅ Builder validation (valid content) test passed!");
}

#[test]
fn test_builder_validation_title_too_long() {
    // Should fail: title too long for Windows (max 128)
    let long_title = "x".repeat(500);
    let result = NotificationBuilder::new()
        .with_title(long_title)
        .with_body(RichText::plain("Body"))
        .with_platforms(vec![Platform::Windows])
        .build();

    assert!(result.is_err());
    match result.unwrap_err() {
        NotificationBuildError::TitleTooLong { platform, length, max } => {
            assert_eq!(platform, "Windows");
            assert_eq!(length, 500);
            assert_eq!(max, 128);
        }
        _ => panic!("Expected TitleTooLong error"),
    }

    println!("✅ Builder validation (title too long) test passed!");
}

#[test]
fn test_builder_validation_no_platforms() {
    // Should fail: empty platforms list
    let result = NotificationBuilder::new()
        .with_title("Test")
        .with_body(RichText::plain("Body"))
        .with_platforms(vec![])
        .build();

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        NotificationBuildError::NoPlatforms
    ));

    println!("✅ Builder validation (no platforms) test passed!");
}

#[test]
fn test_builder_validation_platform_limits() {
    // Test that different platforms have different limits
    let title_256 = "x".repeat(256);

    // Should succeed for macOS (max 256)
    let result = NotificationBuilder::new()
        .with_title(title_256.clone())
        .with_body(RichText::plain("Body"))
        .with_platforms(vec![Platform::MacOS])
        .build();
    assert!(result.is_ok());

    // Should fail for Windows (max 128)
    let result = NotificationBuilder::new()
        .with_title(title_256)
        .with_body(RichText::plain("Body"))
        .with_platforms(vec![Platform::Windows])
        .build();
    assert!(result.is_err());

    println!("✅ Builder validation (platform limits) test passed!");
}
