// Cross-platform integration with sophisticated capability negotiation
// Based on comprehensive analysis of macOS UserNotifications, Windows Toast, Linux D-Bus,
// and web notification standards with enterprise-grade platform abstraction patterns

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::NotificationResult;

/// Comprehensive platform integration component supporting dynamic capability negotiation
/// Incorporates patterns from Linux D-Bus capability detection, Windows adaptive UI,
/// macOS authorization flows, and web standards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformIntegration {
    /// Target platforms for notification delivery
    pub target_platforms: Vec<Platform>,
    /// Negotiated capabilities for each platform
    pub platform_capabilities: HashMap<Platform, PlatformCapabilities>,
    /// Native platform handles and context
    pub native_handles: HashMap<Platform, NativeHandleMetadata>,
    /// Feature support matrix across platforms
    pub feature_matrix: FeatureMatrix,
    /// Graceful degradation strategy
    pub degradation_strategy: DegradationStrategy,
    /// Platform-specific configuration
    pub platform_configs: HashMap<Platform, PlatformConfig>,
    /// Authorization state tracking
    pub authorization_states: HashMap<Platform, AuthorizationState>,
    /// Platform preferences and user settings
    pub user_preferences: PlatformPreferences,
}

impl PlatformIntegration {
    pub fn new(target_platforms: Vec<Platform>) -> Self {
        Self {
            target_platforms,
            platform_capabilities: HashMap::new(),
            native_handles: HashMap::new(),
            feature_matrix: FeatureMatrix::default(),
            degradation_strategy: DegradationStrategy::default(),
            platform_configs: HashMap::new(),
            authorization_states: HashMap::new(),
            user_preferences: PlatformPreferences::default(),
        }
    }

    /// Negotiate capabilities with all target platforms
    pub async fn negotiate_capabilities(
        &mut self,
        platform_manager: &PlatformManager,
    ) -> NotificationResult<()> {
        for platform in &self.target_platforms.clone() {
            let capabilities = platform_manager.get_capabilities(*platform).await?;
            self.platform_capabilities.insert(*platform, capabilities);
        }

        // Build feature matrix based on negotiated capabilities
        self.feature_matrix = FeatureMatrix::from_capabilities(&self.platform_capabilities);

        // Determine degradation strategy
        self.degradation_strategy =
            DegradationStrategy::calculate_optimal_strategy(&self.feature_matrix);

        Ok(())
    }

    /// Check if a specific feature is supported on any target platform
    pub fn supports_feature(&self, feature: &str) -> bool {
        self.feature_matrix.is_supported(feature)
    }

    /// Get the best platform for a specific feature
    pub fn best_platform_for_feature(&self, feature: &str) -> Option<Platform> {
        self.feature_matrix.best_platform_for_feature(feature)
    }

    /// Apply degradation strategy for unsupported features
    pub fn apply_degradation(&mut self, requested_features: &[String]) -> Vec<FeatureDegradation> {
        self.degradation_strategy
            .apply_degradations(requested_features, &self.feature_matrix)
    }

    /// Update authorization state for a platform
    pub fn update_authorization(&mut self, platform: Platform, state: AuthorizationState) {
        self.authorization_states.insert(platform, state);
    }

    /// Check if authorization is required and available
    pub fn is_authorized(&self, platform: Platform) -> bool {
        self.authorization_states
            .get(&platform)
            .is_some_and(|state| state.is_authorized())
    }

    /// Refresh platform capabilities
    pub fn refresh_capabilities(&mut self) {
        // Update capability cache timestamps to trigger refresh
        for platform in self.platform_capabilities.keys() {
            // Mark capabilities as needing refresh by updating metadata
            if let Some(config) = self.platform_configs.get_mut(platform) {
                config.settings.insert(
                    "last_refresh".to_string(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string(),
                );
            }
        }
    }

    /// Get platform-specific limits
    pub fn get_platform_limits(&self, platform: Platform) -> HashMap<String, usize> {
        self.platform_capabilities
            .get(&platform)
            .map(|caps| caps.get_limits())
            .unwrap_or_default()
    }

    /// Update native handle metadata (called by platform backends)
    pub fn update_native_handle(&mut self, platform: Platform, metadata: NativeHandleMetadata) {
        self.native_handles.insert(platform, metadata);
    }
}

impl Default for PlatformIntegration {
    fn default() -> Self {
        Self::new(vec![Platform::MacOS])
    }
}

/// Supported notification platforms with comprehensive coverage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum Platform {
    /// macOS with UserNotifications framework
    #[default]
    MacOS,
    /// Windows with Toast notifications and WinRT
    Windows,
    /// Linux with D-Bus org.freedesktop.Notifications
    Linux,
    /// Web with Service Worker and Push API
    Web,
    /// iOS (for future mobile support)
    IOs,
    /// Android (for future mobile support)  
    Android,
}


impl Platform {
    pub fn name(&self) -> &'static str {
        match self {
            Platform::MacOS => "macOS",
            Platform::Windows => "Windows",
            Platform::Linux => "Linux",
            Platform::Web => "Web",
            Platform::IOs => "iOS",
            Platform::Android => "Android",
        }
    }

    pub fn is_desktop(&self) -> bool {
        matches!(self, Platform::MacOS | Platform::Windows | Platform::Linux)
    }

    pub fn is_mobile(&self) -> bool {
        matches!(self, Platform::IOs | Platform::Android)
    }

    pub fn is_web(&self) -> bool {
        matches!(self, Platform::Web)
    }

    /// Get the default capabilities for this platform (fallback)
    pub fn default_capabilities(&self) -> PlatformCapabilities {
        match self {
            Platform::MacOS => PlatformCapabilities {
                supports_actions: true,
                supports_rich_media: true,
                supports_markup: false,
                supports_sound: true,
                supports_scheduling: true,
                supports_progress: false,
                supports_categories: true,
                supports_replies: true,
                max_actions: Some(4),
                max_title_length: Some(256),
                max_body_length: Some(2048),
                max_image_size: Some(10_485_760), // 10MB
                authorization_required: true,
                ..Default::default()
            },
            Platform::Windows => PlatformCapabilities {
                supports_actions: true,
                supports_rich_media: true,
                supports_markup: true, // XML markup
                supports_sound: true,
                supports_scheduling: false,
                supports_progress: true,
                supports_categories: false,
                supports_replies: false,
                max_actions: Some(5),
                max_title_length: Some(128),
                max_body_length: Some(1024),
                max_image_size: Some(204800), // 200KB
                authorization_required: false,
                ..Default::default()
            },
            Platform::Linux => PlatformCapabilities {
                supports_actions: true,    // Depends on server
                supports_rich_media: true, // Depends on server
                supports_markup: false,    // Depends on server capability
                supports_sound: true,
                supports_scheduling: false,
                supports_progress: false,
                supports_categories: false,
                supports_replies: false,
                max_actions: None, // Server-dependent
                max_title_length: Some(512),
                max_body_length: Some(4096),
                max_image_size: Some(1_048_576), // 1MB
                authorization_required: false,
                ..Default::default()
            },
            Platform::Web => PlatformCapabilities {
                supports_actions: true,
                supports_rich_media: true,
                supports_markup: false,
                supports_sound: false,      // Limited by browser policies
                supports_scheduling: false, // Handled by service worker
                supports_progress: false,
                supports_categories: false,
                supports_replies: false,
                max_actions: Some(2), // Browser limitation
                max_title_length: Some(256),
                max_body_length: Some(2048),
                max_image_size: Some(512000), // 500KB
                authorization_required: true,
                ..Default::default()
            },
            _ => PlatformCapabilities::default(),
        }
    }
}

/// Comprehensive platform capabilities with dynamic detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    // Core feature support
    pub supports_actions: bool,
    pub supports_rich_media: bool,
    pub supports_markup: bool,
    pub supports_sound: bool,
    pub supports_scheduling: bool,
    pub supports_progress: bool,
    pub supports_categories: bool,
    pub supports_replies: bool,

    // Advanced features
    pub supports_custom_ui: bool,
    pub supports_background_activation: bool,
    pub supports_update_content: bool,
    pub supports_persistent: bool,
    pub supports_priority: bool,
    pub supports_grouping: bool,
    pub supports_badges: bool,
    pub supports_vibration: bool,

    // Content limits
    pub max_actions: Option<usize>,
    pub max_title_length: Option<usize>,
    pub max_body_length: Option<usize>,
    pub max_image_size: Option<usize>,
    pub max_sound_duration: Option<Duration>,

    // Platform-specific features
    pub platform_features: HashMap<String, bool>,
    pub platform_limits: HashMap<String, usize>,

    // Authorization and permissions
    pub authorization_required: bool,
    pub permission_levels: Vec<PermissionLevel>,

    // Version and compatibility
    pub platform_version: Option<String>,
    pub api_version: Option<String>,
    pub compatibility_level: CompatibilityLevel,

    // Performance characteristics
    pub delivery_latency_estimate: Option<Duration>,
    pub supports_batching: bool,
    pub rate_limits: Option<RateLimit>,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            supports_actions: false,
            supports_rich_media: false,
            supports_markup: false,
            supports_sound: false,
            supports_scheduling: false,
            supports_progress: false,
            supports_categories: false,
            supports_replies: false,
            supports_custom_ui: false,
            supports_background_activation: false,
            supports_update_content: false,
            supports_persistent: false,
            supports_priority: false,
            supports_grouping: false,
            supports_badges: false,
            supports_vibration: false,
            max_actions: None,
            max_title_length: None,
            max_body_length: None,
            max_image_size: None,
            max_sound_duration: None,
            platform_features: HashMap::new(),
            platform_limits: HashMap::new(),
            authorization_required: false,
            permission_levels: Vec::new(),
            platform_version: None,
            api_version: None,
            compatibility_level: CompatibilityLevel::Full,
            delivery_latency_estimate: None,
            supports_batching: false,
            rate_limits: None,
        }
    }
}

impl PlatformCapabilities {
    /// Get all platform limits as a unified map
    pub fn get_limits(&self) -> HashMap<String, usize> {
        let mut limits = self.platform_limits.clone();

        if let Some(max_actions) = self.max_actions {
            limits.insert("max_actions".to_string(), max_actions);
        }
        if let Some(max_title) = self.max_title_length {
            limits.insert("max_title_length".to_string(), max_title);
        }
        if let Some(max_body) = self.max_body_length {
            limits.insert("max_body_length".to_string(), max_body);
        }
        if let Some(max_image) = self.max_image_size {
            limits.insert("max_image_size".to_string(), max_image);
        }

        limits
    }

    /// Calculate compatibility score with requested features
    pub fn compatibility_score(&self, requested_features: &[String]) -> f64 {
        if requested_features.is_empty() {
            return 1.0;
        }

        let supported_count = requested_features
            .iter()
            .filter(|feature| self.supports_feature(feature))
            .count();

        supported_count as f64 / requested_features.len() as f64
    }

    /// Check if a specific feature is supported
    pub fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "actions" => self.supports_actions,
            "rich_media" => self.supports_rich_media,
            "markup" => self.supports_markup,
            "sound" => self.supports_sound,
            "scheduling" => self.supports_scheduling,
            "progress" => self.supports_progress,
            "categories" => self.supports_categories,
            "replies" => self.supports_replies,
            "custom_ui" => self.supports_custom_ui,
            "background_activation" => self.supports_background_activation,
            "update_content" => self.supports_update_content,
            "persistent" => self.supports_persistent,
            "priority" => self.supports_priority,
            "grouping" => self.supports_grouping,
            "badges" => self.supports_badges,
            "vibration" => self.supports_vibration,
            _ => self
                .platform_features
                .get(feature)
                .copied()
                .unwrap_or(false),
        }
    }
}

/// Feature support matrix across platforms
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureMatrix {
    /// Features supported by each platform
    pub platform_features: HashMap<Platform, HashSet<String>>,
    /// Universal features supported by all platforms
    pub universal_features: HashSet<String>,
    /// Features with partial support
    pub partial_support: HashMap<String, Vec<Platform>>,
    /// Best platform for each feature
    pub best_platform_per_feature: HashMap<String, Platform>,
}

impl FeatureMatrix {
    pub fn from_capabilities(capabilities: &HashMap<Platform, PlatformCapabilities>) -> Self {
        let mut matrix = FeatureMatrix::default();
        let all_features = [
            "actions",
            "rich_media",
            "markup",
            "sound",
            "scheduling",
            "progress",
            "categories",
            "replies",
            "custom_ui",
            "background_activation",
            "update_content",
            "persistent",
            "priority",
            "grouping",
            "badges",
            "vibration",
        ];

        // Build platform feature sets
        for (platform, caps) in capabilities {
            let mut platform_features = HashSet::new();
            for feature in &all_features {
                if caps.supports_feature(feature) {
                    platform_features.insert(feature.to_string());
                }
            }
            matrix
                .platform_features
                .insert(*platform, platform_features);
        }

        // Find universal features (supported by all platforms)
        for feature in &all_features {
            let supported_platforms: Vec<_> = capabilities
                .iter()
                .filter(|(_, caps)| caps.supports_feature(feature))
                .map(|(platform, _)| *platform)
                .collect();

            if supported_platforms.len() == capabilities.len() {
                matrix.universal_features.insert(feature.to_string());
            } else if !supported_platforms.is_empty() {
                matrix
                    .partial_support
                    .insert(feature.to_string(), supported_platforms);
            }
        }

        // Determine best platform for each feature (highest compatibility)
        for feature in &all_features {
            if let Some(best_platform) = capabilities
                .iter()
                .filter(|(_, caps)| caps.supports_feature(feature))
                .max_by_key(|(_, caps)| {
                    // Score based on capability richness and performance
                    let mut score = 0;
                    if caps.supports_feature(feature) {
                        score += 10;
                    }
                    if caps.supports_background_activation {
                        score += 5;
                    }
                    if caps.supports_custom_ui {
                        score += 3;
                    }
                    if caps.rate_limits.is_none() {
                        score += 2;
                    } // No rate limits is better
                    score
                })
                .map(|(platform, _)| *platform)
            {
                matrix
                    .best_platform_per_feature
                    .insert(feature.to_string(), best_platform);
            }
        }

        matrix
    }

    pub fn is_supported(&self, feature: &str) -> bool {
        self.universal_features.contains(feature) || self.partial_support.contains_key(feature)
    }

    pub fn best_platform_for_feature(&self, feature: &str) -> Option<Platform> {
        self.best_platform_per_feature.get(feature).copied()
    }

    pub fn supported_platforms_for_feature(&self, feature: &str) -> Vec<Platform> {
        if self.universal_features.contains(feature) {
            self.platform_features.keys().copied().collect()
        } else {
            self.partial_support
                .get(feature)
                .cloned()
                .unwrap_or_default()
        }
    }
}

/// Graceful degradation strategy for unsupported features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationStrategy {
    /// How to handle unsupported actions
    pub action_fallback: ActionFallback,
    /// How to handle unsupported media
    pub media_fallback: MediaFallback,
    /// How to handle unsupported markup
    pub markup_fallback: MarkupFallback,
    /// Feature substitution rules
    pub feature_substitutions: HashMap<String, String>,
    /// Whether to fail or continue with degraded features
    pub fail_on_critical_unsupported: bool,
    /// Critical features that must be supported
    pub critical_features: HashSet<String>,
}

impl Default for DegradationStrategy {
    fn default() -> Self {
        Self {
            action_fallback: ActionFallback::RemoveActions,
            media_fallback: MediaFallback::TextDescription,
            markup_fallback: MarkupFallback::StripMarkup,
            feature_substitutions: HashMap::new(),
            fail_on_critical_unsupported: false,
            critical_features: HashSet::new(),
        }
    }
}

impl DegradationStrategy {
    pub fn calculate_optimal_strategy(feature_matrix: &FeatureMatrix) -> Self {
        let mut strategy = DegradationStrategy::default();

        // Determine fallback strategies based on feature support
        if feature_matrix.is_supported("actions") {
            strategy.action_fallback = ActionFallback::SimplifyActions;
        } else {
            strategy.action_fallback = ActionFallback::RemoveActions;
        }

        if feature_matrix.is_supported("rich_media") {
            strategy.media_fallback = MediaFallback::SimplifyMedia;
        } else {
            strategy.media_fallback = MediaFallback::TextDescription;
        }

        if feature_matrix.is_supported("markup") {
            strategy.markup_fallback = MarkupFallback::ConvertMarkup;
        } else {
            strategy.markup_fallback = MarkupFallback::StripMarkup;
        }

        strategy
    }

    pub fn apply_degradations(
        &self,
        requested_features: &[String],
        feature_matrix: &FeatureMatrix,
    ) -> Vec<FeatureDegradation> {
        let mut degradations = Vec::new();

        for feature in requested_features {
            if !feature_matrix.is_supported(feature) {
                if self.critical_features.contains(feature)
                    && self.fail_on_critical_unsupported {
                        degradations.push(FeatureDegradation::CriticalUnsupported {
                            feature: feature.clone(),
                            should_fail: true,
                        });
                        continue;
                    }

                // Apply feature-specific degradation
                let degradation = match feature.as_str() {
                    "actions" => FeatureDegradation::ActionFallback(self.action_fallback),
                    "rich_media" => FeatureDegradation::MediaFallback(self.media_fallback),
                    "markup" => FeatureDegradation::MarkupFallback(self.markup_fallback),
                    _ => {
                        if let Some(substitution) = self.feature_substitutions.get(feature) {
                            FeatureDegradation::FeatureSubstitution {
                                original: feature.clone(),
                                substitute: substitution.clone(),
                            }
                        } else {
                            FeatureDegradation::FeatureRemoved(feature.clone())
                        }
                    },
                };

                degradations.push(degradation);
            }
        }

        degradations
    }
}

/// Action fallback strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionFallback {
    /// Remove all actions
    RemoveActions,
    /// Convert actions to simpler forms
    SimplifyActions,
    /// Convert actions to URLs
    ConvertToUrls,
    /// Batch actions into menu
    BatchIntoMenu,
}

/// Media fallback strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaFallback {
    /// Remove all media
    RemoveMedia,
    /// Simplify media (e.g., images only)
    SimplifyMedia,
    /// Convert media to text descriptions
    TextDescription,
    /// Use placeholder media
    UsePlaceholder,
}

/// Markup fallback strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupFallback {
    /// Strip all markup
    StripMarkup,
    /// Convert to supported markup
    ConvertMarkup,
    /// Use plain text with formatting hints
    FormattingHints,
}

/// Feature degradation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureDegradation {
    ActionFallback(ActionFallback),
    MediaFallback(MediaFallback),
    MarkupFallback(MarkupFallback),
    FeatureSubstitution {
        original: String,
        substitute: String,
    },
    FeatureRemoved(String),
    CriticalUnsupported {
        feature: String,
        should_fail: bool,
    },
}

/// Authorization state tracking for platforms that require permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationState {
    /// Not yet requested
    NotRequested,
    /// Permission request in progress
    Requesting,
    /// Permission request pending user action
    Pending,
    /// User granted permission
    Authorized {
        granted_at: std::time::SystemTime,
        permissions: Vec<PermissionLevel>,
    },
    /// User denied permission
    Denied {
        denied_at: std::time::SystemTime,
        can_retry: bool,
    },
    /// Permission expired or revoked
    Revoked {
        revoked_at: std::time::SystemTime,
        reason: String,
    },
    /// Provisional authorization (iOS style)
    Provisional {
        granted_at: std::time::SystemTime,
        expires_at: Option<std::time::SystemTime>,
    },
}

impl AuthorizationState {
    pub fn is_authorized(&self) -> bool {
        matches!(
            self,
            AuthorizationState::Authorized { .. } | AuthorizationState::Provisional { .. }
        )
    }

    pub fn can_request(&self) -> bool {
        match self {
            AuthorizationState::NotRequested => true,
            AuthorizationState::Denied { can_retry, .. } => *can_retry,
            AuthorizationState::Revoked { .. } => true,
            AuthorizationState::Pending => false,
            _ => false,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let AuthorizationState::Provisional {
            expires_at: Some(expires),
            ..
        } = self
        {
            std::time::SystemTime::now() > *expires
        } else {
            false
        }
    }
}

/// Permission levels for fine-grained authorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Basic notification display
    Display,
    /// Sound playback
    Sound,
    /// Badge updates
    Badge,
    /// Rich media attachments
    Media,
    /// Interactive actions
    Actions,
    /// Background processing
    Background,
    /// Location-based notifications
    Location,
    /// Camera/microphone access
    MediaCapture,
    /// Critical alerts (bypass DnD)
    Critical,
}

/// Platform compatibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityLevel {
    /// Full feature support
    Full,
    /// Most features supported with minor limitations
    High,
    /// Basic features supported
    Medium,
    /// Limited functionality
    Low,
    /// Minimal or no support
    None,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
    pub cooldown_period: Duration,
}

/// Platform-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PlatformConfig {
    /// App identifier for platform registration
    pub app_identifier: Option<String>,
    /// Platform-specific settings
    pub settings: HashMap<String, String>,
    /// Feature toggles
    pub enabled_features: HashSet<String>,
    /// Custom platform limits (override defaults)
    pub custom_limits: HashMap<String, usize>,
}


/// Native handle metadata for platform integration tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeHandleMetadata {
    pub handle_type: String,
    pub handle_id: Option<String>,
    pub created_at: std::time::SystemTime,
    pub last_used: Option<std::time::SystemTime>,
    pub usage_count: u64,
    pub metadata: HashMap<String, String>,
}

/// User preferences for platform behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformPreferences {
    /// Preferred platform order for multi-platform delivery
    pub platform_priority: Vec<Platform>,
    /// Per-platform user settings
    pub platform_settings: HashMap<Platform, PlatformUserSettings>,
    /// Global preferences
    pub global_preferences: GlobalPreferences,
}

/// Platform-specific user settings
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformUserSettings {
    pub enabled: bool,
    pub preferred_style: Option<String>,
    pub custom_sound: Option<String>,
    pub do_not_disturb_override: bool,
    pub settings: HashMap<String, String>,
}

/// Global user preferences across platforms
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalPreferences {
    pub fallback_to_other_platforms: bool,
    pub prefer_native_ui: bool,
    pub respect_system_settings: bool,
    pub analytics_enabled: bool,
}

/// Platform manager resource for capability negotiation and management
pub struct PlatformManager {
    /// Platform backends
    backends: HashMap<Platform, Box<dyn PlatformBackend>>,
    /// Capability cache
    capability_cache: Arc<RwLock<HashMap<Platform, (PlatformCapabilities, std::time::SystemTime)>>>,
    /// Authorization manager
    auth_manager: Arc<dyn AuthorizationManager>,
}

impl Default for PlatformManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformManager {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
            capability_cache: Arc::new(RwLock::new(HashMap::new())),
            auth_manager: Arc::new(DefaultAuthorizationManager::new()),
        }
    }

    pub fn register_backend(&mut self, platform: Platform, backend: Box<dyn PlatformBackend>) {
        self.backends.insert(platform, backend);
    }

    /// Get platform capabilities with caching
    pub async fn get_capabilities(
        &self,
        platform: Platform,
    ) -> NotificationResult<PlatformCapabilities> {
        let cache_ttl = Duration::from_secs(60 * 5); // 5 minutes
        let now = std::time::SystemTime::now();

        // Check cache first
        {
            let cache = self.capability_cache.read().await;
            if let Some((capabilities, cached_at)) = cache.get(&platform)
                && now.duration_since(*cached_at).unwrap_or(Duration::MAX) < cache_ttl {
                    return Ok(capabilities.clone());
                }
        }

        // Negotiate capabilities with backend
        let capabilities = if let Some(backend) = self.backends.get(&platform) {
            backend.negotiate_capabilities().await?
        } else {
            platform.default_capabilities()
        };

        // Update cache
        {
            let mut cache = self.capability_cache.write().await;
            cache.insert(platform, (capabilities.clone(), now));
        }

        Ok(capabilities)
    }

    /// Request authorization for platform
    pub async fn request_authorization(
        &self,
        platform: Platform,
        permissions: Vec<PermissionLevel>,
    ) -> NotificationResult<AuthorizationState> {
        self.auth_manager
            .request_authorization(platform, permissions)
            .await
    }

    /// Check current authorization state
    pub async fn check_authorization(
        &self,
        platform: Platform,
    ) -> NotificationResult<AuthorizationState> {
        self.auth_manager.get_authorization_state(platform).await
    }
}

/// Platform backend trait for abstraction
pub trait PlatformBackend: Send + Sync {
    fn negotiate_capabilities(
        &self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<PlatformCapabilities>> + Send + '_>,
    >;
    fn deliver_notification(
        &self,
        request: &NotificationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<DeliveryReceipt>> + Send + '_>,
    >;
    fn update_notification(
        &self,
        id: &str,
        update: &NotificationUpdate,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>;
    fn cancel_notification(
        &self,
        id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>;
}

/// Authorization manager trait
pub trait AuthorizationManager: Send + Sync {
    fn request_authorization(
        &self,
        platform: Platform,
        permissions: Vec<PermissionLevel>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<AuthorizationState>> + Send + '_>,
    >;
    fn get_authorization_state(
        &self,
        platform: Platform,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<AuthorizationState>> + Send + '_>,
    >;
    fn revoke_authorization(
        &self,
        platform: Platform,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>;
}

/// Default authorization manager implementation
struct DefaultAuthorizationManager {
    authorization_cache: std::sync::RwLock<HashMap<Platform, AuthorizationState>>,
}

impl DefaultAuthorizationManager {
    fn new() -> Self {
        Self {
            authorization_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl AuthorizationManager for DefaultAuthorizationManager {
    fn request_authorization(
        &self,
        platform: Platform,
        permissions: Vec<PermissionLevel>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<AuthorizationState>> + Send + '_>,
    > {
        Box::pin(async move {
            // Check if we already have authorization cached
            if let Ok(cache) = self.authorization_cache.read()
                && let Some(existing_auth) = cache.get(&platform)
                    && existing_auth.is_authorized() {
                        return Ok(existing_auth.clone());
                    }

            // Platform-specific authorization logic using real backend APIs
            let auth_state = match platform {
                Platform::MacOS => {
                    // Use real macOS backend to check authorization
                    let backend = crate::backends::macos::MacOSBackend::new();
                    match backend.check_authorization().await {
                        Ok(is_authorized) => {
                            if is_authorized {
                                AuthorizationState::Authorized {
                                    granted_at: std::time::SystemTime::now(),
                                    permissions: permissions.clone(),
                                }
                            } else {
                                AuthorizationState::Denied {
                                    denied_at: std::time::SystemTime::now(),
                                    can_retry: true,
                                }
                            }
                        },
                        Err(_) => AuthorizationState::Denied {
                            denied_at: std::time::SystemTime::now(),
                            can_retry: true,
                        },
                    }
                },
                Platform::Windows => {
                    // Use real Windows backend to check authorization
                    let backend = crate::backends::windows::WindowsBackend::new();
                    match backend.check_authorization().await {
                        Ok(is_authorized) => {
                            if is_authorized {
                                AuthorizationState::Authorized {
                                    granted_at: std::time::SystemTime::now(),
                                    permissions: permissions.clone(),
                                }
                            } else {
                                AuthorizationState::Denied {
                                    denied_at: std::time::SystemTime::now(),
                                    can_retry: true,
                                }
                            }
                        },
                        Err(_) => AuthorizationState::Denied {
                            denied_at: std::time::SystemTime::now(),
                            can_retry: true,
                        },
                    }
                },
                Platform::Linux => {
                    // Use real Linux backend to check authorization
                    let backend = crate::backends::linux::LinuxBackend::new();
                    match backend.check_authorization().await {
                        Ok(is_authorized) => {
                            if is_authorized {
                                AuthorizationState::Authorized {
                                    granted_at: std::time::SystemTime::now(),
                                    permissions: permissions.clone(),
                                }
                            } else {
                                AuthorizationState::Denied {
                                    denied_at: std::time::SystemTime::now(),
                                    can_retry: true,
                                }
                            }
                        },
                        Err(_) => AuthorizationState::Denied {
                            denied_at: std::time::SystemTime::now(),
                            can_retry: true,
                        },
                    }
                },
                Platform::Web => {
                    // Web requires explicit user permission
                    AuthorizationState::Pending
                },
                _ => AuthorizationState::Denied {
                    denied_at: std::time::SystemTime::now(),
                    can_retry: false,
                },
            };

            // Cache the authorization state
            if let Ok(mut cache) = self.authorization_cache.write() {
                cache.insert(platform, auth_state.clone());
            }

            Ok(auth_state)
        })
    }

    fn get_authorization_state(
        &self,
        platform: Platform,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = NotificationResult<AuthorizationState>> + Send + '_>,
    > {
        Box::pin(async move {
            if let Ok(cache) = self.authorization_cache.read()
                && let Some(auth_state) = cache.get(&platform) {
                    return Ok(auth_state.clone());
                }

            // If not cached, request authorization
            self.request_authorization(platform, vec![
                PermissionLevel::Display,
                PermissionLevel::Sound,
            ])
            .await
        })
    }

    fn revoke_authorization(
        &self,
        platform: Platform,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = NotificationResult<()>> + Send + '_>>
    {
        Box::pin(async move {
            if let Ok(mut cache) = self.authorization_cache.write() {
                cache.insert(platform, AuthorizationState::Denied {
                    denied_at: std::time::SystemTime::now(),
                    can_retry: true,
                });
            }
            Ok(())
        })
    }
}

/// Notification request for platform delivery
#[derive(Debug, Clone)]
pub struct NotificationRequest {
    pub notification_id: String,
    pub content: NotificationContent,
    pub options: DeliveryOptions,
    pub correlation_id: String,
}

/// Notification update for content changes
#[derive(Debug, Clone)]
pub struct NotificationUpdate {
    pub content_changes: HashMap<String, String>,
    pub media_changes: Vec<MediaChange>,
    pub action_changes: Vec<ActionChange>,
}

/// Media change for updates
#[derive(Debug, Clone)]
pub enum MediaChange {
    Add(MediaAttachment),
    Remove(String),
    Update {
        id: String,
        new_data: MediaAttachment,
    },
}

/// Action change for updates
#[derive(Debug, Clone)]
pub enum ActionChange {
    Add(NotificationAction),
    Remove(String),
    Update {
        id: String,
        new_action: NotificationAction,
    },
}

/// Delivery options for platform-specific configuration
#[derive(Debug, Clone, Default)]
pub struct DeliveryOptions {
    pub priority: Option<i32>,
    pub ttl: Option<Duration>,
    pub replace_id: Option<String>,
    pub platform_specific: HashMap<String, String>,
}

/// Delivery receipt from successful notification
#[derive(Debug, Clone)]
pub struct DeliveryReceipt {
    pub platform: Platform,
    pub native_id: String,
    pub delivered_at: std::time::SystemTime,
    pub metadata: HashMap<String, String>,
}

// Re-export commonly used types from content module for convenience
pub use super::content::{MediaAttachment, NotificationAction, NotificationContent};
