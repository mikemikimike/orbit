use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers;
use crate::openapi::ApiDoc;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state);

    let protected = Router::new()
        .route("/api/v1/me", get(handlers::account::me))
        .route(
            "/api/v1/organizations/current",
            get(handlers::account::current_organization),
        )
        .route("/api/v1/users", get(handlers::account::list_users))
        .route(
            "/api/v1/users/{user_id}",
            patch(handlers::account::update_user_role),
        )
        .route(
            "/api/v1/api-tokens",
            get(handlers::account::list_api_tokens).post(handlers::account::create_api_token),
        )
        .route(
            "/api/v1/api-tokens/{token_id}",
            delete(handlers::account::revoke_api_token),
        )
        .route(
            "/api/v1/audit-logs",
            get(handlers::account::list_audit_logs),
        )
        .route(
            "/api/v1/agents",
            get(handlers::agents::list_agents).post(handlers::agents::create_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}",
            get(handlers::agents::get_agent)
                .patch(handlers::agents::update_agent)
                .delete(handlers::agents::delete_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/activate",
            post(handlers::agents::activate_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/pause",
            post(handlers::agents::pause_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/versions",
            get(handlers::agents::list_versions),
        )
        .route(
            "/api/v1/agents/{agent_id}/export",
            get(handlers::agents::export_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/executions",
            get(handlers::executions::list_agent_executions)
                .post(handlers::executions::execute_agent),
        )
        .route(
            "/api/v1/agent-instances",
            get(handlers::agents::list_agent_instances),
        )
        .route(
            "/api/v1/executions",
            get(handlers::executions::list_executions),
        )
        .route(
            "/api/v1/executions/{execution_id}",
            get(handlers::executions::get_execution),
        )
        .route(
            "/api/v1/executions/{execution_id}/events",
            get(handlers::executions::list_execution_events),
        )
        .route(
            "/api/v1/executions/{execution_id}/events/stream",
            get(handlers::executions::stream_execution_events),
        )
        .route(
            "/api/v1/executions/{execution_id}/cancel",
            post(handlers::executions::cancel_execution),
        )
        .route("/api/v1/runtimes", get(handlers::catalog::list_runtimes))
        .route(
            "/api/v1/providers",
            get(handlers::catalog::list_providers).post(handlers::catalog::create_provider),
        )
        .route(
            "/api/v1/providers/{provider_id}",
            patch(handlers::catalog::update_provider).delete(handlers::catalog::delete_provider),
        )
        .route(
            "/api/v1/providers/{provider_id}/test",
            post(handlers::catalog::probe_provider),
        )
        .route(
            "/api/v1/connectors",
            get(handlers::catalog::list_connectors),
        )
        .route(
            "/api/v1/connections",
            get(handlers::catalog::list_connections).post(handlers::catalog::create_connection),
        )
        .route(
            "/api/v1/connections/{connection_id}",
            delete(handlers::catalog::delete_connection),
        )
        .route(
            "/api/v1/triggers",
            get(handlers::triggers::list_triggers).post(handlers::triggers::create_trigger),
        )
        .route(
            "/api/v1/triggers/{trigger_id}",
            patch(handlers::triggers::update_trigger).delete(handlers::triggers::delete_trigger),
        )
        .route(
            "/api/v1/approvals",
            get(handlers::approvals::list_approvals),
        )
        .route(
            "/api/v1/approvals/{approval_id}/approve",
            post(handlers::approvals::approve),
        )
        .route(
            "/api/v1/approvals/{approval_id}/reject",
            post(handlers::approvals::reject),
        )
        .route(
            "/api/v1/dashboard/summary",
            get(handlers::dashboard::dashboard_summary),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::auth::require_auth,
        ));

    let public = Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .route("/readyz", get(handlers::health::readyz))
        .route("/metrics", get(handlers::health::metrics))
        .route("/api/v1/meta", get(handlers::health::meta))
        .merge(SwaggerUi::new("/docs").url("/api/v1/openapi.json", ApiDoc::openapi()));

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::request_id::instrument,
        ))
        .layer(DefaultBodyLimit::max(state.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn build_cors(state: &AppState) -> CorsLayer {
    let origins = &state.auth_cors_origins();
    if origins.iter().any(|origin| origin == "*") {
        return CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(Any);
    }
    let parsed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::IF_MATCH,
            header::HeaderName::from_static("idempotency-key"),
            header::HeaderName::from_static("last-event-id"),
            header::HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([header::HeaderName::from_static("x-request-id")])
}
