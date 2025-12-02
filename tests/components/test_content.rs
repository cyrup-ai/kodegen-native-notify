//! Tests for components/content.rs

use kodegen_native_notify::{
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

// Comprehensive tests for HTML to plain text conversion
#[test]
fn test_html_to_plain_basic() {
    let html = RichText::html("<p>Hello</p>");
    assert_eq!(html.to_plain_text(), "Hello");
}

#[test]
fn test_html_to_plain_entity_decoding() {
    let html = RichText::html("a &amp; b");
    assert_eq!(html.to_plain_text(), "a & b");

    // Note: &lt;tag&gt; decodes to <tag>, which is then treated as an HTML tag
    // and stripped. To preserve literal angle brackets, use different encoding.
    let html2 = RichText::html("&lt;tag&gt;");
    assert_eq!(html2.to_plain_text(), "");

    let html3 = RichText::html("&quot;quoted&quot;");
    assert_eq!(html3.to_plain_text(), "\"quoted\"");

    let html4 = RichText::html("&#39;apostrophe&#39;");
    assert_eq!(html4.to_plain_text(), "'apostrophe'");
}

#[test]
fn test_html_to_plain_self_closing_br() {
    let html1 = RichText::html("<br/>");
    assert_eq!(html1.to_plain_text(), "");

    let html2 = RichText::html("<br />");
    assert_eq!(html2.to_plain_text(), "");

    let html3 = RichText::html("line1<br/>line2");
    assert_eq!(html3.to_plain_text(), "line1\nline2");
}

#[test]
fn test_html_to_plain_case_insensitivity() {
    let html1 = RichText::html("<BR>");
    assert_eq!(html1.to_plain_text(), "");

    let html2 = RichText::html("<P>text</P>");
    assert_eq!(html2.to_plain_text(), "text");

    let html3 = RichText::html("<STRONG>bold</STRONG>");
    assert_eq!(html3.to_plain_text(), "bold");
}

#[test]
fn test_html_to_plain_tags_with_attributes() {
    let html1 = RichText::html("<a href='#'>link</a>");
    assert_eq!(html1.to_plain_text(), "link");

    let html2 = RichText::html("<br class=\"foo\">");
    assert_eq!(html2.to_plain_text(), "");

    let html3 = RichText::html("<p style=\"color:red\">styled</p>");
    assert_eq!(html3.to_plain_text(), "styled");
}

#[test]
fn test_html_to_plain_lists() {
    let html = RichText::html("<ul><li>a</li><li>b</li></ul>");
    assert_eq!(html.to_plain_text(), "a\nb");
}

#[test]
fn test_html_to_plain_nested_tags() {
    let html = RichText::html("<div><p>text</p></div>");
    assert_eq!(html.to_plain_text(), "text");

    let html2 = RichText::html("<div><strong>bold</strong> normal</div>");
    assert_eq!(html2.to_plain_text(), "bold normal");
}

#[test]
fn test_html_to_plain_complex_example() {
    let html = RichText::html(
        r#"<div class="notification">
  <h1>Important!</h1>
  <p>Click <a href="https://example.com">here</a> for &amp; details.</p>
  <br/>
  <ul><li>Item 1</li><li>Item 2</li></ul>
</div>"#
    );
    let plain = html.to_plain_text();

    // Should contain the text content
    assert!(plain.contains("Important!"));
    assert!(plain.contains("Click here for & details."));
    assert!(plain.contains("Item 1"));
    assert!(plain.contains("Item 2"));

    // Should not contain HTML tags
    assert!(!plain.contains("<div"));
    assert!(!plain.contains("<h1>"));
    assert!(!plain.contains("<a href"));
    assert!(!plain.contains("&amp;"));
}

#[test]
fn test_html_to_plain_whitespace_normalization() {
    let html = RichText::html("<p>  Multiple   spaces  </p>");
    assert_eq!(html.to_plain_text(), "Multiple   spaces");

    let html2 = RichText::html("<p></p><p>text</p><p></p>");
    assert_eq!(html2.to_plain_text(), "text");
}

#[test]
fn test_html_to_plain_special_entities() {
    // &nbsp; at the beginning gets trimmed by whitespace normalization
    let html = RichText::html("&nbsp;&ndash;&mdash;&hellip;");
    assert_eq!(html.to_plain_text(), "–—…");

    let html2 = RichText::html("&copy; &reg; &trade;");
    assert_eq!(html2.to_plain_text(), "© ® ™");
}

#[test]
fn test_html_to_plain_headings() {
    let html = RichText::html("<h1>Title</h1><h2>Subtitle</h2>");
    let plain = html.to_plain_text();
    assert!(plain.contains("Title"));
    assert!(plain.contains("Subtitle"));
}

#[test]
fn test_html_to_plain_inline_elements() {
    let html = RichText::html("<strong>bold</strong> <em>italic</em> <span>span</span>");
    assert_eq!(html.to_plain_text(), "bold italic span");
}
