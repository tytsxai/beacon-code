use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::metrics::Counter;
use opentelemetry::metrics::Histogram;
use opentelemetry::metrics::Meter;

/// Metrics recorder for tracking API requests, latency, and errors.
///
/// This struct provides instrumentation for:
/// - API request counts (by endpoint and method)
/// - Request latency distribution
/// - Error counts (by error type)
#[derive(Clone)]
pub struct MetricsRecorder {
    api_request_counter: Counter<u64>,
    request_latency_histogram: Histogram<f64>,
    error_counter: Counter<u64>,
}

impl MetricsRecorder {
    /// Create a metrics recorder using the global meter provider.
    pub fn global() -> Self {
        let meter = global::meter("code");
        Self::new(&meter)
    }

    /// Create a new metrics recorder from a meter.
    ///
    /// # Arguments
    /// * `meter` - OpenTelemetry meter for creating instruments
    pub fn new(meter: &Meter) -> Self {
        let api_request_counter = meter
            .u64_counter("codex.api.requests")
            .with_description("Total number of API requests")
            .with_unit("{request}")
            .build();

        let request_latency_histogram = meter
            .f64_histogram("codex.api.request.duration")
            .with_description("API request latency in milliseconds")
            .with_unit("ms")
            .build();

        let error_counter = meter
            .u64_counter("codex.errors")
            .with_description("Total number of errors")
            .with_unit("{error}")
            .build();

        Self {
            api_request_counter,
            request_latency_histogram,
            error_counter,
        }
    }

    /// Record an API request.
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint path (e.g., "/v1/responses")
    /// * `method` - HTTP method (e.g., "POST", "GET")
    pub fn record_api_request(&self, endpoint: &str, method: &str) {
        self.api_request_counter.add(
            1,
            &[
                KeyValue::new("endpoint", endpoint.to_owned()),
                KeyValue::new("method", method.to_owned()),
            ],
        );
    }

    /// Record request latency.
    ///
    /// # Arguments
    /// * `duration_ms` - Request duration in milliseconds
    /// * `endpoint` - API endpoint path
    /// * `status_code` - HTTP status code
    pub fn record_request_latency(&self, duration_ms: f64, endpoint: &str, status_code: u16) {
        self.request_latency_histogram.record(
            duration_ms,
            &[
                KeyValue::new("endpoint", endpoint.to_owned()),
                KeyValue::new("status_code", status_code.to_string()),
            ],
        );
    }

    /// Record an error occurrence.
    ///
    /// # Arguments
    /// * `error_type` - Type of error (e.g., "network", "timeout", "parse")
    /// * `operation` - Operation that failed (e.g., "api_request", "sse_event")
    pub fn record_error(&self, error_type: &str, operation: &str) {
        self.error_counter.add(
            1,
            &[
                KeyValue::new("error_type", error_type.to_owned()),
                KeyValue::new("operation", operation.to_owned()),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::metrics::MeterProvider;
    use opentelemetry_sdk::metrics::SdkMeterProvider;

    #[test]
    fn test_metrics_recorder_creation() {
        let provider = SdkMeterProvider::builder().build();
        let meter = provider.meter("test");

        let recorder = MetricsRecorder::new(&meter);

        // Verify instruments can be used without panicking
        recorder.record_api_request("/v1/responses", "POST");
        recorder.record_request_latency(123.45, "/v1/responses", 200);
        recorder.record_error("network", "api_request");
    }

    #[test]
    fn test_record_api_request() {
        let provider = SdkMeterProvider::builder().build();
        let meter = provider.meter("test");
        let recorder = MetricsRecorder::new(&meter);

        // Should not panic with various inputs
        recorder.record_api_request("/v1/responses", "POST");
        recorder.record_api_request("/v1/responses", "GET");
        recorder.record_api_request("/health", "GET");
    }

    #[test]
    fn test_record_request_latency() {
        let provider = SdkMeterProvider::builder().build();
        let meter = provider.meter("test");
        let recorder = MetricsRecorder::new(&meter);

        // Should handle various latency values
        recorder.record_request_latency(0.0, "/v1/responses", 200);
        recorder.record_request_latency(123.45, "/v1/responses", 200);
        recorder.record_request_latency(5000.0, "/v1/responses", 500);
    }

    #[test]
    fn test_record_error() {
        let provider = SdkMeterProvider::builder().build();
        let meter = provider.meter("test");
        let recorder = MetricsRecorder::new(&meter);

        // Should handle various error types
        recorder.record_error("network", "api_request");
        recorder.record_error("timeout", "sse_event");
        recorder.record_error("parse", "response_processing");
    }
}
