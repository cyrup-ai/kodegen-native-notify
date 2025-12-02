//! Tests for components/tracing.rs

use kodegen_native_notify::{
    TracingContext,
    TraceContext,
    CorrelationId,
    TraceId,
    SpanId,
};

#[test]
fn test_tracing_context_creation() {
    let correlation_id = CorrelationId::generate();
    let context = TracingContext::new(correlation_id.clone());

    assert!(context.active_span.is_none());
    assert_eq!(context.breadcrumbs.len(), 0);
    assert_eq!(context.correlation_data.correlation_id, correlation_id);
}

#[test]
fn test_span_lifecycle() {
    let correlation_id = CorrelationId::generate();
    let mut context = TracingContext::new(correlation_id);

    context.start_span("test_operation");
    assert!(context.active_span.is_some());

    context.finish_span();
    assert!(context.active_span.is_none());
    assert_eq!(context.breadcrumbs.len(), 1);
}

#[test]
fn test_trace_context_headers() {
    let trace_context = TraceContext {
        trace_id: TraceId::generate(),
        span_id: SpanId::generate(),
        correlation_id: CorrelationId::generate(),
        sampling_decision: true,
    };

    let headers = trace_context.to_http_headers();
    assert!(headers.contains_key("X-Trace-Id"));
    assert!(headers.contains_key("X-Span-Id"));
    assert!(headers.contains_key("X-Correlation-Id"));
    assert!(headers.contains_key("X-Sampling-Decision"));

    let reconstructed = TraceContext::from_http_headers(&headers);
    assert!(reconstructed.is_some());
}
