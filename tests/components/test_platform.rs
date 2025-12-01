//! Tests for components/platform.rs

use std::collections::HashMap;
use ecs_notifications::components::{
    Platform,
    FeatureMatrix,
    AuthorizationState,
    PermissionLevel,
    DegradationStrategy,
    ActionFallback,
    FeatureDegradation,
};

#[test]
fn test_platform_capabilities() {
    let macos_caps = Platform::MacOS.default_capabilities();
    assert!(macos_caps.supports_actions);
    assert!(macos_caps.supports_rich_media);
    assert!(!macos_caps.supports_markup);

    let windows_caps = Platform::Windows.default_capabilities();
    assert!(windows_caps.supports_actions);
    assert!(windows_caps.supports_markup);
    assert!(windows_caps.supports_progress);
}

#[test]
fn test_feature_matrix() {
    let mut capabilities = HashMap::new();
    capabilities.insert(Platform::MacOS, Platform::MacOS.default_capabilities());
    capabilities.insert(Platform::Windows, Platform::Windows.default_capabilities());

    let matrix = FeatureMatrix::from_capabilities(&capabilities);

    assert!(matrix.is_supported("actions"));
    assert!(matrix.is_supported("rich_media"));

    // Both platforms support actions, so it should be universal
    assert!(matrix.universal_features.contains("actions"));
}

#[test]
fn test_authorization_state() {
    let auth = AuthorizationState::Authorized {
        granted_at: std::time::SystemTime::now(),
        permissions: vec![PermissionLevel::Display, PermissionLevel::Sound],
    };

    assert!(auth.is_authorized());
    assert!(!auth.can_request());

    let denied = AuthorizationState::Denied {
        denied_at: std::time::SystemTime::now(),
        can_retry: true,
    };

    assert!(!denied.is_authorized());
    assert!(denied.can_request());
}

#[test]
fn test_degradation_strategy() {
    let mut feature_matrix = FeatureMatrix::default();
    feature_matrix
        .universal_features
        .insert("actions".to_string());

    let strategy = DegradationStrategy::calculate_optimal_strategy(&feature_matrix);
    assert_eq!(strategy.action_fallback, ActionFallback::SimplifyActions);

    let requested_features = vec!["unsupported_feature".to_string()];
    let degradations = strategy.apply_degradations(&requested_features, &feature_matrix);

    assert_eq!(degradations.len(), 1);
    match &degradations[0] {
        FeatureDegradation::FeatureRemoved(feature) => {
            assert_eq!(feature, "unsupported_feature");
        },
        _ => panic!("Expected FeatureRemoved degradation"),
    }
}
