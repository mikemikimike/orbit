use futures::StreamExt;
use orbit_contracts::{OrbitEvent, SubjectFilter};
use orbit_events::{EventBus, NatsEventBus};

/// Requires a running NATS server with JetStream enabled:
///   ORBIT_TEST_NATS_URL=nats://localhost:4222 cargo test -p orbit-events -- --ignored
#[tokio::test]
#[ignore]
async fn nats_publish_subscribe_roundtrip() {
    let url =
        std::env::var("ORBIT_TEST_NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    let bus = NatsEventBus::connect(&url, "ORBIT").await.expect("connect");
    let mut stream = bus
        .subscribe(SubjectFilter::new("orbit.test.>"))
        .await
        .expect("subscribe");

    let event = OrbitEvent::new(
        "orbit.test.integration",
        "orbit-test",
        serde_json::json!({ "ok": true }),
    )
    .with_idempotency("test-key");
    bus.publish(event.clone()).await.expect("publish");

    let received = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .expect("timeout")
        .expect("stream item")
        .expect("event");
    assert_eq!(received.id, event.id);
    assert_eq!(received.subject, "orbit.test.integration");
    bus.health().await.expect("healthy");
}
