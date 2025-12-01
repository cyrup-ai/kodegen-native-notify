use bevy::prelude::*;
use ecs_notifications::*;

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
        .build();

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
fn test_bevy_ecs_systems_integration() {
    let mut app = App::new();

    // Add minimal Bevy plugins needed for testing
    app.add_plugins(MinimalPlugins);

    // Add our notification system
    app.add_plugins(NotificationSystemPlugin);

    // Spawn a test notification entity
    let entity = app
        .world_mut()
        .spawn(
            NotificationBuilder::new()
                .with_title("Bevy ECS Test")
                .with_body(RichText::plain("Testing ECS integration"))
                .build(),
        )
        .id();

    // Run one update cycle
    app.update();

    // Verify the entity exists and has our components
    let world = app.world();
    assert!(world.get::<NotificationIdentity>(entity).is_some());
    assert!(world.get::<NotificationContent>(entity).is_some());
    assert!(world.get::<NotificationLifecycle>(entity).is_some());
    assert!(world.get::<PlatformIntegration>(entity).is_some());
    assert!(world.get::<NotificationAnalytics>(entity).is_some());

    println!("✅ Bevy ECS systems integration test passed!");
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
