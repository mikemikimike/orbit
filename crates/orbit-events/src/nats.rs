use async_nats::jetstream;
use futures::StreamExt;

use orbit_contracts::{OrbitEvent, SubjectFilter};

use crate::{EventBus, EventBusError, EventStream};

/// NATS JetStream transport.
///
/// Published events are persisted in a JetStream stream (`ORBIT` by default) so that
/// connector/trigger events survive restarts and can be replayed later. Live consumers
/// use a core NATS subscription for low latency; durable pull consumers are the Phase 2
/// addition for event replay. Execution work itself is never dependent on the bus: the
/// worker polls the durable PostgreSQL queue as a safety net.
pub struct NatsEventBus {
    client: async_nats::Client,
    js: jetstream::Context,
    stream_name: String,
}

impl NatsEventBus {
    pub async fn connect(url: &str, stream_name: &str) -> Result<Self, EventBusError> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| EventBusError::Unavailable(format!("NATS connect failed: {e}")))?;
        let js = jetstream::new(client.clone());
        js.get_or_create_stream(jetstream::stream::Config {
            name: stream_name.to_string(),
            subjects: vec!["orbit.>".to_string()],
            max_age: std::time::Duration::from_secs(7 * 24 * 3600),
            ..Default::default()
        })
        .await
        .map_err(|e| {
            EventBusError::Unavailable(format!(
                "could not create JetStream stream '{stream_name}': {e}"
            ))
        })?;
        Ok(Self {
            client,
            js,
            stream_name: stream_name.to_string(),
        })
    }

    pub fn stream_name(&self) -> &str {
        &self.stream_name
    }
}

#[async_trait::async_trait]
impl EventBus for NatsEventBus {
    fn driver(&self) -> &'static str {
        "nats"
    }

    async fn publish(&self, event: OrbitEvent) -> Result<(), EventBusError> {
        let payload = event
            .encode()
            .map_err(|e| EventBusError::Protocol(format!("could not encode event: {e}")))?;
        self.js
            .publish(event.subject.clone(), payload.into())
            .await
            .map_err(|e| EventBusError::Unavailable(format!("NATS publish failed: {e}")))?;
        Ok(())
    }

    async fn subscribe(&self, filter: SubjectFilter) -> Result<EventStream, EventBusError> {
        let mut subscriber = self
            .client
            .subscribe(filter.as_str().to_string())
            .await
            .map_err(|e| EventBusError::Unavailable(format!("NATS subscribe failed: {e}")))?;
        let pattern = filter.as_str().to_string();
        let stream = async_stream::stream! {
            while let Some(message) = subscriber.next().await {
                match OrbitEvent::decode(&message.payload) {
                    Ok(event) => yield Ok(event),
                    Err(err) => yield Err(EventBusError::Protocol(format!(
                        "invalid event payload on {}: {err}",
                        message.subject
                    ))),
                }
            }
            tracing::debug!(pattern = %pattern, "NATS subscription ended");
        };
        Ok(Box::pin(stream))
    }

    async fn health(&self) -> Result<(), EventBusError> {
        let mut account = self
            .js
            .get_stream(&self.stream_name)
            .await
            .map_err(|e| EventBusError::Unavailable(format!("NATS stream missing: {e}")))?;
        let _info = account
            .info()
            .await
            .map_err(|e| EventBusError::Unavailable(format!("NATS stream info failed: {e}")))?;
        Ok(())
    }
}
