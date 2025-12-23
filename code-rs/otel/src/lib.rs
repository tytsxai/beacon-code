pub mod config;

pub mod otel_event_manager;

#[cfg(feature = "otel")]
pub mod metrics;
#[cfg(feature = "otel")]
pub mod otel_provider;

#[cfg(not(feature = "otel"))]
mod imp {
    use std::error::Error;
    use tracing_subscriber::layer::Identity;

    pub struct OtelProvider;

    impl OtelProvider {
        pub fn from(
            _settings: &crate::config::OtelSettings,
        ) -> Result<Option<Self>, Box<dyn Error>> {
            Ok(None)
        }

        pub fn layer(&self) -> Identity {
            Identity::default()
        }

        pub fn shutdown(&self) {
            // no-op when OTEL is disabled
        }
    }

    /// Shim metrics recorder when OTEL feature is disabled.
    /// All recording operations are no-ops with zero runtime cost.
    #[derive(Clone)]
    pub struct MetricsRecorder;

    impl MetricsRecorder {
        pub fn global() -> Self {
            Self
        }

        pub fn new(_meter: &()) -> Self {
            Self
        }

        pub fn record_api_request(&self, _endpoint: &str, _method: &str) {
            // no-op
        }

        pub fn record_request_latency(
            &self,
            _duration_ms: f64,
            _endpoint: &str,
            _status_code: u16,
        ) {
            // no-op
        }

        pub fn record_error(&self, _error_type: &str, _operation: &str) {
            // no-op
        }
    }
}

#[cfg(not(feature = "otel"))]
pub use imp::MetricsRecorder;
#[cfg(not(feature = "otel"))]
pub use imp::OtelProvider;
