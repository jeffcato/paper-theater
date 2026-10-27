use super::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
fn body() -> Value {
    json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":pt_contracts::digest(b"fixture-v1"),"profile_sha256":pt_contracts::digest(b"profile-v1"),"profile_id":"diagnostic-v1"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}})
}
#[tokio::test]
async fn start_is_idempotent_conflicts_fail_and_cancel_is_durable() {
    let temp = tempfile::tempdir().unwrap();
    let app = router(App::new(
        pt_production::runtime::Store::open(temp.path()).unwrap(),
        8787,
        None,
        None,
    ));
    async fn start(app: axum::Router, body: Value) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs")
                .header("host", "127.0.0.1:8787")
                .header("origin", "http://127.0.0.1:8787")
                .header("content-type", "application/json")
                .header("idempotency-key", "browser-one")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }
    let response = start(app.clone(), body()).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let first: pt_contracts::RunSnapshot =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    let response = start(app.clone(), body()).await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let next: pt_contracts::RunSnapshot =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(next.run_id, first.run_id);
    let mut changed = body();
    changed["limits"]["tokens"] = json!(1);
    assert_eq!(
        start(app.clone(), changed).await.status(),
        StatusCode::CONFLICT
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/runs/{}/cancel", first.run_id))
                .header("host", "127.0.0.1:8787")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cancelled: pt_contracts::RunSnapshot =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(cancelled.state, pt_contracts::RunState::Cancelled);
}
#[tokio::test]
async fn snapshot_stream_race_and_last_event_id_deliver_committed_events() {
    let temp = tempfile::tempdir().unwrap();
    let store = pt_production::runtime::Store::open(temp.path()).unwrap();
    let created = store.create("race", &body()).unwrap();
    let id = created["id"].as_str().unwrap();
    let n = created["last_event_seq"].as_i64().unwrap();
    store.claim().unwrap();
    store.cancel(id).unwrap();
    let app = router(App::new(store.clone(), 8787, None, None));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runs/{id}/events?after=0"))
                .header("host", "127.0.0.1:8787")
                .header("last-event-id", n.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let events: Vec<pt_contracts::RunEvent> = text
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect();
    let expected = store.api_journal(id, n, 500).unwrap();
    assert_eq!(
        events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        expected.iter().map(|e| e.seq).collect::<Vec<_>>()
    );
    assert_eq!(events.len(), 2);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runs/{id}/events?after=999999"))
                .header("host", "127.0.0.1:8787")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let text = String::from_utf8(
        to_bytes(response.into_body(), 65536)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(text.contains("snapshot_required"));
}
#[tokio::test]
async fn loopback_origin_and_artifact_allowlist_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("secret.txt"), "private").unwrap();
    let app = router(App::new(
        pt_production::runtime::Store::open(temp.path()).unwrap(),
        8787,
        None,
        None,
    ));
    for (host, origin) in [
        ("evil.example:8787", "http://evil.example:8787"),
        ("127.0.0.1:8787", "http://evil.example"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/runs")
                    .header("host", host)
                    .header("origin", origin)
                    .header("content-type", "application/json")
                    .header("idempotency-key", "forbidden")
                    .body(Body::from(body().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/artifacts/secret.txt")
                .header("host", "127.0.0.1:8787")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn diagnostic_endpoint_distinguishes_missing_candidate_from_unknown_run() {
    let temp = tempfile::tempdir().unwrap();
    let store = pt_production::runtime::Store::open(temp.path()).unwrap();
    let created = store.create("candidate-status", &body()).unwrap();
    let id = created["id"].as_str().unwrap();
    let app = router(App::new(store, 8787, None, None));
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runs/{id}/diagnostic"))
                .header("host", "127.0.0.1:8787")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        &to_bytes(response.into_body(), 1024).await.unwrap()[..],
        b"null"
    );
}

#[tokio::test]
async fn provider_v2_metadata_does_not_enable_http_live_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let app = router(App::new(store.clone(), 8787, None, None));
    let mut request = body();
    request["mode"] = json!("live");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs")
                .header("host", "127.0.0.1:8787")
                .header("origin", "http://127.0.0.1:8787")
                .header("content-type", "application/json")
                .header("idempotency-key", "no-live-adapter")
                .body(Body::from(request.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let value: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
    assert_eq!(value["code"], "live_disabled");
    assert_eq!(
        store.api_history(i64::MAX, 10).unwrap()["runs"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/provider-v2")
                .header("host", "127.0.0.1:8787")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
