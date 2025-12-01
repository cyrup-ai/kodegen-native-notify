// Rich notification content component with enterprise-grade media and interaction support
// Based on comprehensive analysis of macOS UserNotifications, Windows Toast, Linux D-Bus patterns

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{NotificationCategory, NotificationError, NotificationResult, Priority};

/// Comprehensive notification content supporting rich media and complex interactions
/// Incorporates patterns from Slack's rich messaging, Discord's media handling,
/// and native platform capabilities (macOS attachments, Windows adaptive UI, Linux hints)
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct NotificationContent {
    /// Primary notification title (required)
    pub title: String,
    /// Optional subtitle (macOS specific, but supported cross-platform)
    pub subtitle: Option<String>,
    /// Rich text body with markup support
    pub body: RichText,
    /// Rich media attachments (images, audio, video)
    pub media: Vec<MediaAttachment>,
    /// Interactive elements (buttons, inputs, menus)
    pub interactions: InteractionSet,
    /// Notification category for templating and grouping
    pub category: NotificationCategory,
    /// Priority level for attention management
    pub priority: Priority,
    /// Custom data for application-specific handling
    pub custom_data: HashMap<String, String>,
    /// Localization data for i18n support
    pub localization: Option<LocalizationData>,
    /// Accessibility metadata
    pub accessibility: AccessibilityMetadata,
    /// Content validation state
    pub validation_state: ValidationState,
}

impl NotificationContent {
    pub fn new(title: impl Into<String>, body: impl Into<RichText>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            body: body.into(),
            media: Vec::new(),
            interactions: InteractionSet::default(),
            category: NotificationCategory::new("default", "Default"),
            priority: Priority::default(),
            custom_data: HashMap::new(),
            localization: None,
            accessibility: AccessibilityMetadata::default(),
            validation_state: ValidationState::Pending,
        }
    }

    /// Builder pattern methods for fluent API construction
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_category(mut self, category: NotificationCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_media(mut self, media: MediaAttachment) -> Self {
        self.media.push(media);
        self
    }

    pub fn with_interaction(mut self, interaction: NotificationInteraction) -> Self {
        match interaction {
            NotificationInteraction::Action(action) => {
                self.interactions.actions.push(*action);
            },
            NotificationInteraction::Input(input) => {
                self.interactions.inputs.push(input);
            },
            NotificationInteraction::QuickReply(reply) => {
                self.interactions.quick_replies.push(reply);
            },
        }
        self
    }

    pub fn with_custom_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_data.insert(key.into(), value.into());
        self
    }

    /// Validate content against platform constraints and security requirements
    pub fn validate(&mut self, platform_limits: &HashMap<String, usize>) -> NotificationResult<()> {
        // Title validation
        if self.title.is_empty() {
            return Err(NotificationError::ValidationError {
                field: "title".to_string(),
                message: "Title cannot be empty".to_string(),
            });
        }

        // Length constraints based on platform limits
        if let Some(&max_title_length) = platform_limits.get("max_title_length")
            && self.title.len() > max_title_length {
                return Err(NotificationError::ValidationError {
                    field: "title".to_string(),
                    message: format!(
                        "Title exceeds maximum length of {} characters",
                        max_title_length
                    ),
                });
            }

        // Body content validation
        self.body.validate(platform_limits)?;

        // Media validation
        for (index, media) in self.media.iter().enumerate() {
            media
                .validate(platform_limits)
                .map_err(|e| NotificationError::ValidationError {
                    field: format!("media[{}]", index),
                    message: e.to_string(),
                })?;
        }

        // Interaction validation
        self.interactions.validate(platform_limits)?;

        // Security validation - sanitize content
        self.sanitize_content()?;

        self.validation_state = ValidationState::Valid;
        Ok(())
    }

    /// Sanitize content for security (XSS prevention, injection protection)
    fn sanitize_content(&mut self) -> NotificationResult<()> {
        // HTML sanitization for body content
        if let RichText::Html(ref mut html) = self.body {
            *html = sanitize_html(html)?;
        }

        // Sanitize custom data values
        for (_, value) in self.custom_data.iter_mut() {
            *value = sanitize_string(value);
        }

        Ok(())
    }

    /// Check if content supports background activation
    pub fn supports_background_activation(&self) -> bool {
        self.interactions
            .actions
            .iter()
            .any(|action| matches!(action.activation_type, ActivationType::Background))
    }

    /// Get estimated content size for platform limits
    pub fn estimated_size(&self) -> usize {
        self.title.len()
            + self.subtitle.as_ref().map_or(0, |s| s.len())
            + self.body.estimated_size()
            + self.media.iter().map(|m| m.estimated_size()).sum::<usize>()
    }
}

impl Default for NotificationContent {
    fn default() -> Self {
        Self::new("Default Title", RichText::Plain(String::default()))
    }
}

/// Rich text content with cross-platform markup support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RichText {
    /// Plain text content
    Plain(String),
    /// Markdown content (converted to platform-appropriate format)
    Markdown(String),
    /// HTML content (sanitized, platform support varies)
    Html(String),
    /// Platform-specific formatted text
    PlatformSpecific(HashMap<String, String>),
}

impl RichText {
    pub fn plain(text: impl Into<String>) -> Self {
        Self::Plain(text.into())
    }

    pub fn markdown(text: impl Into<String>) -> Self {
        Self::Markdown(text.into())
    }

    pub fn html(text: impl Into<String>) -> Self {
        Self::Html(text.into())
    }

    /// Convert to plain text for platforms that don't support markup
    pub fn to_plain_text(&self) -> String {
        match self {
            RichText::Plain(text) => text.clone(),
            RichText::Markdown(md) => convert_markdown_to_plain(md),
            RichText::Html(html) => convert_html_to_plain(html),
            RichText::PlatformSpecific(map) => map
                .get("plain")
                .or_else(|| map.values().next())
                .cloned()
                .unwrap_or_default(),
        }
    }

    /// Convert to HTML for platforms that support it
    pub fn to_html(&self) -> String {
        match self {
            RichText::Plain(text) => html_escape(text),
            RichText::Markdown(md) => convert_markdown_to_html(md),
            RichText::Html(html) => html.clone(),
            RichText::PlatformSpecific(map) => {
                if let Some(html) = map.get("html") {
                    html.clone()
                } else if let Some(plain) = map.get("plain") {
                    html_escape(plain)
                } else {
                    String::default()
                }
            },
        }
    }

    pub fn validate(&self, platform_limits: &HashMap<String, usize>) -> NotificationResult<()> {
        let content = self.to_plain_text();

        if let Some(&max_body_length) = platform_limits.get("max_body_length")
            && content.len() > max_body_length {
                return Err(NotificationError::ValidationError {
                    field: "body".to_string(),
                    message: format!(
                        "Body exceeds maximum length of {} characters",
                        max_body_length
                    ),
                });
            }

        Ok(())
    }

    pub fn estimated_size(&self) -> usize {
        match self {
            RichText::Plain(text) => text.len(),
            RichText::Markdown(md) => md.len(),
            RichText::Html(html) => html.len(),
            RichText::PlatformSpecific(map) => map.values().map(|s| s.len()).sum(),
        }
    }
}

impl<T: Into<String>> From<T> for RichText {
    fn from(text: T) -> Self {
        RichText::Plain(text.into())
    }
}

/// Rich media attachments supporting cross-platform capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaAttachment {
    /// Image attachment with placement and metadata
    Image {
        data: ImageData,
        placement: ImagePlacement,
        alt_text: Option<String>,
        dimensions: Option<(u32, u32)>,
    },
    /// Audio attachment for notification sounds
    Audio {
        source: AudioSource,
        volume: f32,
        loop_audio: bool,
        duration: Option<Duration>,
    },
    /// Video attachment (platform support varies)
    Video {
        data: VideoData,
        thumbnail: Option<ImageData>,
        duration: Option<Duration>,
        auto_play: bool,
    },
    /// File attachment for documents/downloads
    File {
        path: PathBuf,
        filename: Option<String>,
        mime_type: Option<String>,
        size_bytes: Option<u64>,
    },
}

impl MediaAttachment {
    pub fn validate(&self, platform_limits: &HashMap<String, usize>) -> NotificationResult<()> {
        match self {
            MediaAttachment::Image { data, .. } => {
                data.validate(platform_limits)?;

                if let Some(&max_image_size) = platform_limits.get("max_image_size")
                    && data.estimated_size() > max_image_size {
                        return Err(NotificationError::ValidationError {
                            field: "image_size".to_string(),
                            message: format!(
                                "Image exceeds maximum size of {} bytes",
                                max_image_size
                            ),
                        });
                    }
            },
            MediaAttachment::Audio { source, .. } => {
                source.validate()?;
            },
            MediaAttachment::Video { data, .. } => {
                data.validate()?;
            },
            MediaAttachment::File {
                path, size_bytes, ..
            } => {
                if !path.exists() {
                    return Err(NotificationError::ResourceError {
                        resource_type: "file".to_string(),
                        resource_id: path.display().to_string(),
                        message: "File does not exist".to_string(),
                    });
                }

                if let Some(&max_file_size) = platform_limits.get("max_file_size")
                    && let Some(size) = size_bytes
                        && *size as usize > max_file_size {
                            return Err(NotificationError::ValidationError {
                                field: "file_size".to_string(),
                                message: format!(
                                    "File exceeds maximum size of {} bytes",
                                    max_file_size
                                ),
                            });
                        }
            },
        }
        Ok(())
    }

    pub fn estimated_size(&self) -> usize {
        match self {
            MediaAttachment::Image { data, .. } => data.estimated_size(),
            MediaAttachment::Audio { .. } => 1024, // Rough estimate
            MediaAttachment::Video { data, .. } => data.estimated_size(),
            MediaAttachment::File { size_bytes, .. } => size_bytes.unwrap_or(0) as usize,
        }
    }
}

/// Image data sources with cross-platform support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageData {
    /// Image from file path
    File(PathBuf),
    /// Image from URL (web or local)
    Url(Url),
    /// Embedded image data with format
    Embedded { data: Vec<u8>, format: ImageFormat },
    /// System icon identifier
    SystemIcon(String),
}

impl ImageData {
    pub fn validate(&self, _platform_limits: &HashMap<String, usize>) -> NotificationResult<()> {
        match self {
            ImageData::File(path) => {
                if !path.exists() {
                    return Err(NotificationError::ResourceError {
                        resource_type: "image_file".to_string(),
                        resource_id: path.display().to_string(),
                        message: "Image file does not exist".to_string(),
                    });
                }
            },
            ImageData::Url(url) => {
                if url.scheme() == "file" {
                    let path = PathBuf::from(url.path());
                    if !path.exists() {
                        return Err(NotificationError::ResourceError {
                            resource_type: "image_url".to_string(),
                            resource_id: url.to_string(),
                            message: "Image file does not exist".to_string(),
                        });
                    }
                }
            },
            ImageData::Embedded { data, format } => {
                if data.is_empty() {
                    return Err(NotificationError::ValidationError {
                        field: "image_data".to_string(),
                        message: "Embedded image data cannot be empty".to_string(),
                    });
                }

                // Basic format validation
                if !format.is_supported() {
                    return Err(NotificationError::ValidationError {
                        field: "image_format".to_string(),
                        message: format!("Unsupported image format: {:?}", format),
                    });
                }
            },
            ImageData::SystemIcon(_) => {
                // System icons are always valid
            },
        }
        Ok(())
    }

    pub fn estimated_size(&self) -> usize {
        match self {
            ImageData::File(path) => path.metadata().map(|m| m.len() as usize).unwrap_or(0),
            ImageData::Url(_) => 1024, // Rough estimate for URL
            ImageData::Embedded { data, .. } => data.len(),
            ImageData::SystemIcon(_) => 64, // Small system icon
        }
    }

    pub fn as_url(&self) -> Option<Url> {
        match self {
            ImageData::Url(url) => Some(url.clone()),
            ImageData::File(path) => Url::from_file_path(path).ok(),
            _ => None,
        }
    }
}

/// Image placement options for different UI contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePlacement {
    /// App icon replacement
    AppIcon,
    /// Small icon next to content
    Icon,
    /// Large hero image (Windows Toast, web notifications)
    Hero,
    /// Inline with content
    Inline,
    /// Background image (limited platform support)
    Background,
}

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    WebP,
    Svg,
    Ico,
}

impl ImageFormat {
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP
        )
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Gif => "image/gif",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::Ico => "image/x-icon",
        }
    }
}

/// Audio source options for notification sounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioSource {
    /// System sound by name
    System(SystemSound),
    /// Custom audio file
    File(PathBuf),
    /// Audio from URL
    Url(Url),
    /// Default system notification sound
    Default,
    /// No sound
    Silent,
}

impl AudioSource {
    pub fn validate(&self) -> NotificationResult<()> {
        match self {
            AudioSource::File(path) => {
                if !path.exists() {
                    return Err(NotificationError::ResourceError {
                        resource_type: "audio_file".to_string(),
                        resource_id: path.display().to_string(),
                        message: "Audio file does not exist".to_string(),
                    });
                }
            },
            AudioSource::Url(url) => {
                if url.scheme() == "file" {
                    let path = PathBuf::from(url.path());
                    if !path.exists() {
                        return Err(NotificationError::ResourceError {
                            resource_type: "audio_url".to_string(),
                            resource_id: url.to_string(),
                            message: "Audio file does not exist".to_string(),
                        });
                    }
                }
            },
            _ => {}, // System sounds and defaults are always valid
        }
        Ok(())
    }
}

/// Cross-platform system sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemSound {
    Default,
    Alert,
    Critical,
    Information,
    Question,
    Warning,
    Error,
    Success,
}

/// Video data for rich notifications (limited platform support)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoData {
    pub source: VideoSource,
    pub format: VideoFormat,
    pub loop_video: bool,
}

impl VideoData {
    pub fn validate(&self) -> NotificationResult<()> {
        self.source.validate()
    }

    pub fn estimated_size(&self) -> usize {
        match &self.source {
            VideoSource::File(path) => path.metadata().map(|m| m.len() as usize).unwrap_or(0),
            VideoSource::Embedded { data, .. } => data.len(),
            _ => 10240, // Rough estimate
        }
    }
}

/// Video source options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoSource {
    File(PathBuf),
    Url(Url),
    Embedded { data: Vec<u8>, mime_type: String },
}

impl VideoSource {
    pub fn validate(&self) -> NotificationResult<()> {
        match self {
            VideoSource::File(path) => {
                if !path.exists() {
                    return Err(NotificationError::ResourceError {
                        resource_type: "video_file".to_string(),
                        resource_id: path.display().to_string(),
                        message: "Video file does not exist".to_string(),
                    });
                }
            },
            VideoSource::Embedded { data, .. } => {
                if data.is_empty() {
                    return Err(NotificationError::ValidationError {
                        field: "video_data".to_string(),
                        message: "Embedded video data cannot be empty".to_string(),
                    });
                }
            },
            _ => {},
        }
        Ok(())
    }
}

/// Video format support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    WebM,
    Mov,
    Avi,
}

/// Comprehensive interaction system supporting complex user workflows
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionSet {
    pub actions: Vec<NotificationAction>,
    pub inputs: Vec<NotificationInput>,
    pub quick_replies: Vec<QuickReply>,
    pub context_menu: Vec<ContextMenuAction>,
}

impl InteractionSet {
    pub fn validate(&self, platform_limits: &HashMap<String, usize>) -> NotificationResult<()> {
        // Check action limits
        if let Some(&max_actions) = platform_limits.get("max_actions")
            && self.actions.len() > max_actions {
                return Err(NotificationError::ValidationError {
                    field: "actions".to_string(),
                    message: format!(
                        "Too many actions: {} (max: {})",
                        self.actions.len(),
                        max_actions
                    ),
                });
            }

        // Validate individual actions
        for (index, action) in self.actions.iter().enumerate() {
            action
                .validate()
                .map_err(|e| NotificationError::ValidationError {
                    field: format!("actions[{}]", index),
                    message: e.to_string(),
                })?;
        }

        // Validate inputs
        for (index, input) in self.inputs.iter().enumerate() {
            input
                .validate()
                .map_err(|e| NotificationError::ValidationError {
                    field: format!("inputs[{}]", index),
                    message: e.to_string(),
                })?;
        }

        Ok(())
    }

    pub fn find_action(&self, action_id: &ActionId) -> Option<&NotificationAction> {
        self.actions.iter().find(|action| &action.id == action_id)
    }

    pub fn find_input(&self, input_id: &InputId) -> Option<&NotificationInput> {
        self.inputs.iter().find(|input| input.id() == input_id)
    }
}

/// Unified interaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationInteraction {
    Action(Box<NotificationAction>),
    Input(NotificationInput),
    QuickReply(QuickReply),
}

/// Rich notification actions with complex behaviors (Slack/Discord/Teams patterns)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: ActionId,
    pub label: String,
    pub icon: Option<ActionIcon>,
    pub style: ActionStyle,
    pub activation_type: ActivationType,
    pub url: Option<Url>,
    pub payload: Option<ActionPayload>,
    pub confirmation: Option<ActionConfirmation>,
}

impl NotificationAction {
    pub fn validate(&self) -> NotificationResult<()> {
        if self.label.is_empty() {
            return Err(NotificationError::ValidationError {
                field: "label".to_string(),
                message: "Action label cannot be empty".to_string(),
            });
        }

        if self.label.len() > 64 {
            return Err(NotificationError::ValidationError {
                field: "label".to_string(),
                message: "Action label too long (max 64 characters)".to_string(),
            });
        }

        Ok(())
    }
}

/// User input elements for interactive notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationInput {
    Text {
        id: InputId,
        label: String,
        placeholder: String,
        validation: Option<InputValidation>,
        max_length: Option<usize>,
        multiline: bool,
    },
    Selection {
        id: InputId,
        label: String,
        options: Vec<SelectionOption>,
        multiple: bool,
        default_selection: Option<String>,
    },
    Number {
        id: InputId,
        label: String,
        min_value: Option<f64>,
        max_value: Option<f64>,
        step: Option<f64>,
        default_value: Option<f64>,
    },
    Date {
        id: InputId,
        label: String,
        min_date: Option<chrono::NaiveDate>,
        max_date: Option<chrono::NaiveDate>,
    },
}

impl NotificationInput {
    pub fn validate(&self) -> NotificationResult<()> {
        match self {
            NotificationInput::Text {
                label, max_length, ..
            } => {
                if label.is_empty() {
                    return Err(NotificationError::ValidationError {
                        field: "label".to_string(),
                        message: "Input label cannot be empty".to_string(),
                    });
                }

                if let Some(max_len) = max_length
                    && *max_len == 0 {
                        return Err(NotificationError::ValidationError {
                            field: "max_length".to_string(),
                            message: "Max length must be greater than 0".to_string(),
                        });
                    }
            },
            NotificationInput::Selection { label, options, .. } => {
                if label.is_empty() {
                    return Err(NotificationError::ValidationError {
                        field: "label".to_string(),
                        message: "Input label cannot be empty".to_string(),
                    });
                }

                if options.is_empty() {
                    return Err(NotificationError::ValidationError {
                        field: "options".to_string(),
                        message: "Selection input must have at least one option".to_string(),
                    });
                }
            },
            _ => {},
        }
        Ok(())
    }

    pub fn id(&self) -> &InputId {
        match self {
            NotificationInput::Text { id, .. }
            | NotificationInput::Selection { id, .. }
            | NotificationInput::Number { id, .. }
            | NotificationInput::Date { id, .. } => id,
        }
    }
}

/// Quick reply options for instant responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickReply {
    pub id: ActionId,
    pub text: String,
    pub payload: Option<String>,
    pub icon: Option<String>,
}

/// Context menu actions (right-click, long press)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuAction {
    pub id: ActionId,
    pub label: String,
    pub icon: Option<String>,
    pub separator_before: bool,
}

// Supporting types for actions and inputs

/// Unique action identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(String);

impl ActionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique input identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InputId(String);

impl InputId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Action visual styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStyle {
    Default,
    Primary,
    Secondary,
    Destructive,
    Success,
    Warning,
}

/// Action activation behaviors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationType {
    /// Bring app to foreground
    Foreground,
    /// Handle in background without UI
    Background,
    /// Launch external URL/app
    Protocol,
}

/// Action payload data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPayload {
    pub data: HashMap<String, String>,
    pub callback_url: Option<Url>,
}

/// Action confirmation dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfirmation {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
}

/// Action icon data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionIcon {
    System(String),
    File(PathBuf),
    Url(Url),
    Embedded(Vec<u8>),
}

/// Input validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidation {
    pub required: bool,
    pub pattern: Option<String>, // Regex pattern
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub error_message: Option<String>,
}

impl InputValidation {
    pub fn validate(&self, value: &str) -> Result<(), String> {
        if self.required && value.trim().is_empty() {
            return Err(self
                .error_message
                .clone()
                .unwrap_or_else(|| "This field is required".to_string()));
        }

        if let Some(min_len) = self.min_length
            && value.len() < min_len {
                return Err(format!("Minimum length is {} characters", min_len));
            }

        if let Some(max_len) = self.max_length
            && value.len() > max_len {
                return Err(format!("Maximum length is {} characters", max_len));
            }

        if let Some(pattern) = &self.pattern
            && let Ok(regex) = regex::Regex::new(pattern)
                && !regex.is_match(value) {
                    return Err(self
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "Invalid format".to_string()));
                }

        Ok(())
    }
}

/// Selection option for dropdowns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
    pub icon: Option<String>,
}

/// Localization data for i18n support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationData {
    pub locale: String,
    pub translations: HashMap<String, String>,
    pub rtl: bool, // Right-to-left text direction
}

/// Accessibility metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccessibilityMetadata {
    pub screen_reader_text: Option<String>,
    pub high_contrast_mode: bool,
    pub large_text_mode: bool,
    pub reduce_motion: bool,
}

/// Content validation state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationState {
    Pending,
    Valid,
    Invalid(String),
}

// Utility functions for content processing

fn sanitize_html(html: &str) -> NotificationResult<String> {
    use ammonia::Builder;

    // ammonia::Builder.clean() returns a Cow<str> (not Result)
    // It never fails - malicious content is stripped, not errored
    let cleaned = Builder::default()
        // Allow only safe formatting tags for notifications
        .add_tags(&["p", "br", "strong", "em", "b", "i", "u", "ul", "ol", "li", "a", "span", "div"])
        // Allow href and title attributes on links
        .add_tag_attributes("a", &["href", "title"])
        // Allow class attribute on spans and divs for styling
        .add_tag_attributes("span", &["class"])
        .add_tag_attributes("div", &["class"])
        // Add rel="noopener noreferrer" to all links for security
        .link_rel(Some("noopener noreferrer"))
        // Only allow http/https URLs (blocks javascript:, data:, vbscript:)
        .url_schemes(HashSet::from(["https", "http"]))
        // Clean the HTML (strips all disallowed tags, attributes, and scripts)
        .clean(html)
        .to_string();

    Ok(cleaned)
}

fn sanitize_string(input: &str) -> String {
    // Basic string sanitization
    input.replace(['<', '>', '"', '\''], "")
}

fn convert_markdown_to_plain(markdown: &str) -> String {
    use pulldown_cmark::{Parser, Event, Options};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    
    let parser = Parser::new_ext(markdown, options);
    let mut plain_text = String::new();

    // Extract only text content, ignore all formatting
    for event in parser {
        match event {
            Event::Text(text) | Event::Code(text) => {
                plain_text.push_str(&text);
            }
            Event::SoftBreak => {
                plain_text.push(' ');
            }
            Event::HardBreak => {
                plain_text.push('\n');
            }
            // Ignore all other events (tags, HTML, etc.)
            _ => {}
        }
    }

    plain_text
}

fn convert_markdown_to_html(markdown: &str) -> String {
    use pulldown_cmark::{Parser, html, Options, Event, Tag};

    // Enable safe CommonMark features
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    
    let parser = Parser::new_ext(markdown, options);

    // Filter events to remove dangerous content
    let safe_parser = parser.filter_map(|event| {
        match event {
            // Block ALL raw HTML from markdown
            Event::Html(_) | Event::InlineHtml(_) => None,

            // Sanitize link URLs
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                // Only allow http/https links
                if dest_url.starts_with("http://") || dest_url.starts_with("https://") {
                    Some(Event::Start(Tag::Link { link_type, dest_url, title, id }))
                } else if dest_url.starts_with("#") {
                    // Allow anchor links
                    Some(Event::Start(Tag::Link { link_type, dest_url, title, id }))
                } else {
                    // Block javascript:, data:, and other dangerous protocols
                    None
                }
            }

            // Block potentially dangerous image sources
            Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
                // Only allow http/https images
                if dest_url.starts_with("http://") || dest_url.starts_with("https://") {
                    Some(Event::Start(Tag::Image { link_type, dest_url, title, id }))
                } else {
                    // Block data: URIs and other potentially dangerous sources
                    None
                }
            }

            // Pass through all other safe markdown elements
            _ => Some(event),
        }
    });

    // Generate HTML from filtered events
    let mut html_output = String::new();
    html::push_html(&mut html_output, safe_parser);

    // Double-sanitize: run through ammonia to catch any edge cases
    // This handles any HTML that might have been generated by the markdown parser
    sanitize_html(&html_output).unwrap_or(html_output)
}

fn convert_html_to_plain(html: &str) -> String {
    // Basic HTML to plain text conversion
    // In production, use a proper HTML parser
    html.replace("<br>", "\n")
        .replace("<p>", "")
        .replace("</p>", "\n")
        .replace("<strong>", "")
        .replace("</strong>", "")
        .replace("<em>", "")
        .replace("</em>", "")
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
