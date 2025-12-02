// Shared image utilities for all platform backends
// Handles downloading remote images and converting to local temp files

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tempfile::NamedTempFile;
use tokio::sync::OnceCell;
use url::Url;

use crate::components::{ImageData, MediaAttachment, NotificationError};

/// Cached image entry with TTL tracking
#[derive(Debug, Clone)]
struct CachedImage {
    path: PathBuf,
    cached_at: Instant,
}

/// Cache TTL: 1 hour
const CACHE_TTL_SECS: u64 = 3600;

/// Maximum cache entries before forced eviction
const MAX_CACHE_ENTRIES: usize = 100;

/// Global HTTP client for image downloads (shared across all backends)
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

/// Cache of downloaded images with TTL tracking
/// Key: URL string, Value: CachedImage with timestamp
static IMAGE_CACHE: OnceCell<Arc<DashMap<String, CachedImage>>> = OnceCell::const_new();

/// Get or create the shared HTTP client
async fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| async {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .user_agent("KODEGEN-Notifications/1.0")
                .build()
                .expect("Failed to create HTTP client")
        })
        .await
}

/// Get or create the image cache
async fn get_image_cache() -> Arc<DashMap<String, CachedImage>> {
    IMAGE_CACHE
        .get_or_init(|| async { Arc::new(DashMap::new()) })
        .await
        .clone()
}

/// Evict stale entries from cache (TTL expired or over size limit)
fn evict_stale_entries(cache: &DashMap<String, CachedImage>) {
    let now = Instant::now();
    let ttl = Duration::from_secs(CACHE_TTL_SECS);

    // Remove expired entries
    cache.retain(|_, entry| {
        let is_valid = now.duration_since(entry.cached_at) < ttl;
        if !is_valid {
            // Cleanup the temp file when evicting
            let _ = std::fs::remove_file(&entry.path);
        }
        is_valid
    });

    // If still over limit, remove oldest entries
    while cache.len() > MAX_CACHE_ENTRIES {
        // Find and remove oldest entry
        let oldest = cache.iter()
            .min_by_key(|entry| entry.cached_at)
            .map(|entry| entry.key().clone());

        if let Some(key) = oldest {
            if let Some((_, entry)) = cache.remove(&key) {
                let _ = std::fs::remove_file(&entry.path);
            }
        } else {
            break;
        }
    }
}

/// Result of resolving an image to a local file path
#[derive(Debug, Clone)]
pub struct ResolvedImage {
    /// Local file path (either original file:// path or downloaded temp file)
    pub path: PathBuf,
    /// Whether this is a temporary file that should be cleaned up later
    pub is_temp: bool,
    /// Original URL for logging/debugging
    pub original_url: String,
}

/// Download a remote image to a temporary file
///
/// Supports http:// and https:// URLs. Returns the path to the downloaded temp file.
/// Uses caching to avoid re-downloading the same image multiple times.
pub async fn download_image(url: &Url) -> Result<ResolvedImage, NotificationError> {
    let url_string = url.to_string();

    // Check cache first (with TTL validation)
    let cache = get_image_cache().await;
    evict_stale_entries(&cache);

    if let Some(cached) = cache.get(&url_string) {
        if cached.path.exists() {
            return Ok(ResolvedImage {
                path: cached.path.clone(),
                is_temp: true,
                original_url: url_string,
            });
        } else {
            // Cached file was deleted, remove from cache
            drop(cached);
            cache.remove(&url_string);
        }
    }

    let client = get_http_client().await;

    // Download the image
    let response = client
        .get(url.as_str())
        .send()
        .await
        .map_err(|e| NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("Failed to download image: {}", e),
        })?;

    // Check response status
    if !response.status().is_success() {
        return Err(NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("HTTP {} downloading image", response.status()),
        });
    }

    // Validate Content-Length before downloading (10MB max)
    const MAX_IMAGE_SIZE: u64 = 10 * 1024 * 1024;
    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH)
        && let Ok(length_str) = content_length.to_str()
        && let Ok(size) = length_str.parse::<u64>()
        && size > MAX_IMAGE_SIZE
    {
        return Err(NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!(
                "Image too large: {} bytes exceeds {} byte limit",
                size, MAX_IMAGE_SIZE
            ),
        });
    }

    // Determine file extension from URL or content-type
    let extension = determine_extension(url, response.headers());

    // Get the bytes
    let bytes = response
        .bytes()
        .await
        .map_err(|e| NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("Failed to read image bytes: {}", e),
        })?;

    // Validate we got actual image data (basic check)
    if bytes.len() < 8 {
        return Err(NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: "Downloaded image is too small to be valid".to_string(),
        });
    }

    // Create temp file with appropriate extension
    let temp_file = NamedTempFile::with_suffix(format!(".{}", extension))
        .map_err(|e| NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("Failed to create temp file: {}", e),
        })?;

    // Write bytes to temp file
    let temp_path = temp_file.path().to_path_buf();
    tokio::fs::write(&temp_path, &bytes)
        .await
        .map_err(|e| NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("Failed to write temp file: {}", e),
        })?;

    // Keep the temp file alive by persisting it (don't auto-delete on drop)
    let persisted_path = temp_file.into_temp_path().keep()
        .map_err(|e| NotificationError::ResourceError {
            resource_type: "image".to_string(),
            resource_id: url_string.clone(),
            message: format!("Failed to persist temp file: {}", e),
        })?;

    // Cache the path with timestamp
    cache.insert(url_string.clone(), CachedImage {
        path: persisted_path.clone(),
        cached_at: Instant::now(),
    });

    tracing::debug!("Downloaded image {} -> {:?}", url_string, persisted_path);

    Ok(ResolvedImage {
        path: persisted_path,
        is_temp: true,
        original_url: url_string,
    })
}

/// Resolve an ImageData to a local file path
///
/// - File paths are returned as-is
/// - file:// URLs are converted to paths
/// - http:// and https:// URLs are downloaded to temp files
/// - Embedded data is written to temp files
/// - SystemIcon returns None (platform-specific handling needed)
pub async fn resolve_image_to_path(data: &ImageData) -> Result<Option<ResolvedImage>, NotificationError> {
    match data {
        ImageData::File(path) => {
            if !path.exists() {
                return Err(NotificationError::ResourceError {
                    resource_type: "image".to_string(),
                    resource_id: path.display().to_string(),
                    message: "Image file does not exist".to_string(),
                });
            }
            Ok(Some(ResolvedImage {
                path: path.clone(),
                is_temp: false,
                original_url: format!("file://{}", path.display()),
            }))
        }

        ImageData::Url(url) => {
            match url.scheme() {
                "file" => {
                    let path = PathBuf::from(url.path());
                    if !path.exists() {
                        return Err(NotificationError::ResourceError {
                            resource_type: "image".to_string(),
                            resource_id: url.to_string(),
                            message: "Image file does not exist".to_string(),
                        });
                    }
                    Ok(Some(ResolvedImage {
                        path,
                        is_temp: false,
                        original_url: url.to_string(),
                    }))
                }
                "http" | "https" => {
                    let resolved = download_image(url).await?;
                    Ok(Some(resolved))
                }
                scheme => {
                    tracing::warn!("Unsupported URL scheme for image: {}", scheme);
                    Ok(None)
                }
            }
        }

        ImageData::Embedded { data, format } => {
            let extension = match format {
                crate::components::ImageFormat::Png => "png",
                crate::components::ImageFormat::Jpeg => "jpg",
                crate::components::ImageFormat::Gif => "gif",
                crate::components::ImageFormat::WebP => "webp",
                crate::components::ImageFormat::Svg => "svg",
                crate::components::ImageFormat::Ico => "ico",
            };

            let temp_file = NamedTempFile::with_suffix(format!(".{}", extension))
                .map_err(|e| NotificationError::ResourceError {
                    resource_type: "image".to_string(),
                    resource_id: "embedded".to_string(),
                    message: format!("Failed to create temp file: {}", e),
                })?;

            let temp_path = temp_file.path().to_path_buf();
            tokio::fs::write(&temp_path, data)
                .await
                .map_err(|e| NotificationError::ResourceError {
                    resource_type: "image".to_string(),
                    resource_id: "embedded".to_string(),
                    message: format!("Failed to write temp file: {}", e),
                })?;

            let persisted_path = temp_file.into_temp_path().keep()
                .map_err(|e| NotificationError::ResourceError {
                    resource_type: "image".to_string(),
                    resource_id: "embedded".to_string(),
                    message: format!("Failed to persist temp file: {}", e),
                })?;

            Ok(Some(ResolvedImage {
                path: persisted_path,
                is_temp: true,
                original_url: "embedded://data".to_string(),
            }))
        }

        ImageData::SystemIcon(icon_name) => {
            // System icons need platform-specific handling
            tracing::debug!("SystemIcon '{}' requires platform-specific handling", icon_name);
            Ok(None)
        }
    }
}

/// Extract image attachments from media list and resolve them to local paths
///
/// Returns a list of (placement, resolved_image) tuples
pub async fn resolve_media_images(
    media: &[MediaAttachment],
) -> Vec<(crate::components::ImagePlacement, ResolvedImage)> {
    let mut resolved = Vec::new();

    for attachment in media {
        if let MediaAttachment::Image { data, placement, .. } = attachment {
            match resolve_image_to_path(data).await {
                Ok(Some(image)) => {
                    resolved.push((*placement, image));
                }
                Ok(None) => {
                    // SystemIcon or unsupported scheme, skip
                }
                Err(e) => {
                    tracing::warn!("Failed to resolve image: {}", e);
                }
            }
        }
    }

    resolved
}

/// Determine file extension from URL path or Content-Type header
fn determine_extension(url: &Url, headers: &reqwest::header::HeaderMap) -> String {
    // Try to get extension from URL path first
    if let Some(mut path_segments) = url.path_segments()
        && let Some(last_segment) = path_segments.next_back()
        && let Some(dot_pos) = last_segment.rfind('.')
    {
        let ext = &last_segment[dot_pos + 1..];
        if !ext.is_empty() && ext.len() <= 4 {
            return ext.to_lowercase();
        }
    }

    // Fall back to Content-Type header
    if let Some(content_type) = headers.get(reqwest::header::CONTENT_TYPE)
        && let Ok(ct) = content_type.to_str()
    {
        return match ct {
            ct if ct.contains("image/png") => "png",
            ct if ct.contains("image/jpeg") || ct.contains("image/jpg") => "jpg",
            ct if ct.contains("image/gif") => "gif",
            ct if ct.contains("image/webp") => "webp",
            ct if ct.contains("image/svg") => "svg",
            ct if ct.contains("image/x-icon") || ct.contains("image/vnd.microsoft.icon") => "ico",
            ct if ct.contains("image/bmp") => "bmp",
            ct if ct.contains("image/tiff") => "tiff",
            _ => "png", // Default to PNG
        }
        .to_string();
    }

    // Default to PNG if we can't determine
    "png".to_string()
}

/// Clean up a temp file if it was downloaded
pub fn cleanup_temp_image(image: &ResolvedImage) {
    if image.is_temp
        && let Err(e) = std::fs::remove_file(&image.path)
    {
        tracing::debug!("Failed to cleanup temp image {:?}: {}", image.path, e);
    }
}

/// Clean up all cached temp images
pub fn cleanup_all_cached_images() {
    if let Some(cache) = IMAGE_CACHE.get() {
        for entry in cache.iter() {
            if let Err(e) = std::fs::remove_file(&entry.value().path) {
                tracing::debug!("Failed to cleanup cached image {:?}: {}", entry.value().path, e);
            }
        }
        cache.clear();
    }
}
