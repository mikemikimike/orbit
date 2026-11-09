use axum::extract::{Request, State};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::state::AppState;

pub const HEADER: &str = "x-request-id";

tokio::task_local! {
    static REQUEST_ID: String;
}

pub fn current() -> String {
    REQUEST_ID.try_with(|id| id.clone()).unwrap_or_default()
}

/// Generates/propagates a request id, opens a tracing span and records metrics.
pub async fn instrument(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let incoming = request
        .headers()
        .get(HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .filter(|value| {
            !value.is_empty() && value.len() <= 128 && value.chars().all(|c| c.is_ascii_graphic())
        });
    let request_id = incoming.unwrap_or_else(|| Uuid::now_v7().to_string());

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let span = tracing::info_span!(
        "http.request",
        http.method = %method,
        http.path = %path,
        request.id = %request_id
    );

    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));
    state.metrics.inc_requests();

    let response = REQUEST_ID
        .scope(request_id.clone(), async move {
            let _guard = span.enter();
            next.run(request).await
        })
        .await;

    if response.status().is_client_error() || response.status().is_server_error() {
        state.metrics.inc_errors();
    }

    let mut response = response;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(HEADER, value);
    }
    response
}

/// Extracted request id.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl<S> axum::extract::FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestId>()
            .cloned()
            .unwrap_or_else(|| RequestId(current())))
    }
}
