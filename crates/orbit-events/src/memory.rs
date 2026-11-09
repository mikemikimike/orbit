use futures::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use orbit_contracts::{OrbitEvent, SubjectFilter};

use crate::{EventBus, EventBusError, EventStream};

/// In-process bus for tests and single-process development. Non-durable by design.
pub struct MemoryEventBus {
    sender: broadcast::Sender<OrbitEvent>,
}

impl MemoryEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(16));
        Self { sender }
    }
}

impl std::fmt::Debug for MemoryEventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryEventBus")
    }
}

#[async_trait::async_trait]
impl EventBus for MemoryEventBus {
    fn driver(&self) -> &'static str {
        "memory"
    }

    async fn publish(&self, event: OrbitEvent) -> Result<(), EventBusError> {
        // No subscribers is not an error: the durable row is the source of truth.
        let _ = self.sender.send(event);
        Ok(())
    }

    async fn subscribe(&self, filter: SubjectFilter) -> Result<EventStream, EventBusError> {
        let receiver = self.sender.subscribe();
        let stream = BroadcastStream::new(receiver).filter_map(move |item| {
            let filter = filter.clone();
            async move {
                match item {
                    Ok(event) if filter.matches(&event.subject) => Some(Ok(event)),
                    Ok(_) => None,
                    Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "event subscriber lagged");
                        None
                    }
                }
            }
        });
        Ok(Box::pin(stream))
    }

    async fn health(&self) -> Result<(), EventBusError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_contracts::subjects;

    #[tokio::test]
    async fn publish_subscribe_filtered() {
        let bus = MemoryEventBus::new(32);
        let mut stream = bus
            .subscribe(SubjectFilter::new("orbit.execution.*"))
            .await
            .unwrap();
        bus.publish(OrbitEvent::new(
            "orbit.trigger.fired",
            "test",
            serde_json::json!({}),
        ))
        .await
        .unwrap();
        let event = OrbitEvent::new(
            subjects::EXECUTION_REQUESTED,
            "test",
            serde_json::json!({ "executionId": "1" }),
        );
        bus.publish(event.clone()).await.unwrap();
        let received = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(received.subject, event.subject);
    }
}
