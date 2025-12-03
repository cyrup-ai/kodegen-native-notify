//! Example: Terminal Notification (Replicates kodegen hook notify)
//!
//! This example creates the exact same notification that `kodegen hook notify`
//! sends when a terminal command completes.
//!
//! Run with: cargo run --example terminal_notification

use kodegen_native_notify::{
    ImageData, ImagePlacement, MediaAttachment, NotificationBuilder,
    NotificationManager, Platform, RichText, Url,
};

/// KODEGEN logo URL for branding in notifications
const LOGO_URL: &str = "https://kodegen.ai/assets/icon_128x128@2x.png";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // ========================================
    // MOCKED TERMINAL OUTPUT DATA
    // ========================================
    let command = "cargo build --release";
    let terminal_id = 0;
    let exit_code = Some(0);
    let duration_ms = 45230_u64; // 45.2 seconds
    let completed = true;
    let cwd = "/Volumes/samsung_t9/kodegen-workspace/packages/kodegen";
    let transcript_path = "/Users/davidmaple/.claude/projects/kodegen/transcript_abc123.jsonl";
    
    // Mocked terminal output (last 20 lines of a cargo build)
    let terminal_output = r#"   Compiling kodegen-mcp-schema v0.5.0
   Compiling kodegen-mcp-tool v0.5.0
   Compiling kodegen-utils v0.5.0
   Compiling kodegen-simd v0.5.0
   Compiling kodegen-config-manager v0.5.0
   Compiling kodegen-tools-filesystem v0.5.0
   Compiling kodegen-tools-terminal v0.5.0
   Compiling kodegen-tools-git v0.5.0
   Compiling kodegen-tools-github v0.5.0
   Compiling kodegen-tools-browser v0.5.0
   Compiling kodegen-tools-citescrape v0.5.0
   Compiling kodegen-tools-database v0.5.0
   Compiling kodegen-tools-reasoner v0.5.0
   Compiling kodegen-claude-agent v0.5.0
   Compiling kodegen-native-notify v0.5.2
   Compiling kodegen v0.5.0
    Finished `release` profile [optimized] target(s) in 45.23s"#;

    // ========================================
    // BUILD NOTIFICATION (exact same as hooks/notify.rs)
    // ========================================
    let cmd_short = truncate(command, 40);
    let duration = format_duration(duration_ms);
    let output_preview = truncate_output(terminal_output, 20);

    let (icon, status) = match (exit_code, completed) {
        (Some(0), true) => ("✓", "success".to_string()),
        (Some(code), true) => ("✗", format!("exit {}", code)),
        (None, false) => ("⏳", "running".to_string()),
        _ => ("•", "unknown".to_string()),
    };

    let title = format!("{} terminal {}: {}", icon, terminal_id, cmd_short);
    let transcript_link = format_transcript_link(transcript_path);
    
    let body_html = format!(
        r#"<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Ubuntu, sans-serif;">
            <img src="{}" width="24" height="24" alt="KODEGEN"/>
            <p><strong>{}</strong> in {}</p>
            <p><em>cwd:</em> {}</p>
            <pre style="background:#1e1e1e;color:#d4d4d4;padding:8px;border-radius:4px;font-size:12px;white-space:pre-wrap;word-wrap:break-word;">{}</pre>
            <p>{}</p>
        </div>"#,
        LOGO_URL,
        status,
        duration,
        html_escape(cwd),
        html_escape(&output_preview),
        transcript_link
    );

    println!("=== NOTIFICATION TITLE ===");
    println!("{}", title);
    println!();
    println!("=== NOTIFICATION BODY (HTML) ===");
    println!("{}", body_html);
    println!();

    // ========================================
    // BUILD AND SEND NOTIFICATION
    // ========================================
    let mut builder = NotificationBuilder::new()
        .with_title(&title)
        .with_body(RichText::html(&body_html))
        .with_platforms(vec![Platform::MacOS, Platform::Windows, Platform::Linux]);

    // Add KODEGEN logo as app icon
    if let Ok(logo_url) = Url::parse(LOGO_URL) {
        builder = builder.with_media(MediaAttachment::Image {
            data: ImageData::Url(logo_url),
            placement: ImagePlacement::AppIcon,
            alt_text: Some("KODEGEN".to_string()),
            dimensions: Some((128, 128)),
        });
    }

    let notification = builder.build()?;

    println!("=== SENDING NOTIFICATION ===");
    let manager = NotificationManager::new();
    let handle = manager.send(notification).await?;
    
    // Wait a moment for delivery
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Check status
    if let Some(status) = handle.status().await {
        println!("Notification ID: {}", status.id);
        println!("State: {:?}", status.state);
        println!("Target platforms: {:?}", status.platforms);
    }

    // Shutdown manager
    let result = manager.shutdown().await;
    println!("Shutdown result: {:?}", result);

    Ok(())
}

/// HTML escape special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

/// Format duration in human-readable form
fn format_duration(ms: u64) -> String {
    if ms >= 60000 {
        format!("{}m {}s", ms / 60000, (ms % 60000) / 1000)
    } else if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

/// Format transcript path as clickable file:// hyperlink
fn format_transcript_link(path: &str) -> String {
    format!(r#"<a href="file://{}">View Transcript</a>"#, path)
}

/// Truncate output to last N lines if more than max_lines
fn truncate_output(output: &str, max_lines: usize) -> String {
    let trimmed = output.trim();
    let lines: Vec<&str> = trimmed.lines().collect();

    if lines.len() <= max_lines {
        return trimmed.to_string();
    }

    // Take last max_lines
    let start = lines.len() - max_lines;
    let last_lines = &lines[start..];
    format!("...({} lines hidden)\n{}", start, last_lines.join("\n"))
}
