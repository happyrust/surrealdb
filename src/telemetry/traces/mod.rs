pub mod rpc;

use std::sync::{LazyLock, Mutex};

use anyhow::Result;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Subscriber;
use tracing_subscriber::Layer;

use crate::cli::validator::parser::tracing::CustomFilter;
use crate::cnf::{TELEMETRY_DISABLE_TRACING, TELEMETRY_PROVIDER};
use crate::telemetry::OTEL_DEFAULT_RESOURCE;

static ACTIVE_TRACER_PROVIDER: LazyLock<Mutex<Option<SdkTracerProvider>>> =
	LazyLock::new(|| Mutex::new(None));

// Returns a tracer provider based on the SURREAL_TELEMETRY_PROVIDER environment
// variable
pub fn new<S>(filter: CustomFilter) -> Result<Option<Box<dyn Layer<S> + Send + Sync>>>
where
	S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync,
{
	match TELEMETRY_PROVIDER.trim() {
		// The OTLP telemetry provider has been specified
		s if s.eq_ignore_ascii_case("otlp") && !*TELEMETRY_DISABLE_TRACING => {
			// Build a new span exporter which uses gRPC
			let span_exporter = SpanExporter::builder()
				.with_tonic()
				.build()?;
			// Create the provider with the Tokio runtime
			let provider = SdkTracerProvider::builder()
				.with_batch_exporter(span_exporter)
				.with_resource(OTEL_DEFAULT_RESOURCE.clone())
				.build();
			// Set it as the global tracer provider
			let _ = opentelemetry::global::set_tracer_provider(provider.clone());
			// Store it so we can flush on shutdown
			{
				let mut slot = ACTIVE_TRACER_PROVIDER.lock().unwrap();
				*slot = Some(provider.clone());
			}
			// Return the tracing layer with the specified filter
			Ok(Some(
				tracing_opentelemetry::layer()
					.with_tracer(provider.tracer("surealdb"))
					.with_filter(filter.env())
					.with_filter(filter.span_filter::<S>())
					.boxed(),
			))
		}
		// No matching telemetry provider was found
		_ => Ok(None),
	}
}

pub fn take_provider() -> Option<SdkTracerProvider> {
	ACTIVE_TRACER_PROVIDER.lock().unwrap().take()
}
