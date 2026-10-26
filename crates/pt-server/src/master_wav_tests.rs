use super::*;
use axum::{body::to_bytes, http::Request};
use pt_contracts as c;
use std::sync::Arc;
use tower::ServiceExt;
#[path = "../../../fixtures/runtime/chapter_wav.rs"]
mod wav;
struct Producer;
impl c::StageExecutor for Producer {
    fn execute<'life0, 'life1, 'a>(
        &'life0 self,
        input: c::StageInput,
        services: &'life1 dyn c::RunServices,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<c::StageOutput, c::StageError>>
                + Send
                + 'a,
        >,
    >
    where
        'life0: 'a,
        'life1: 'a,
        Self: 'a,
    {
        Box::pin(async move {
            let file = services
                .write_candidate(
                    c::CandidateWrite {
                        candidate_id: input.unit_id,
                        relative_path: "audio/master.wav".into(),
                        media_type: "audio/wav".into(),
                        max_bytes: 256 * 1024 * 1024,
                        dimensions: None,
                        provenance: c::Provenance::Synthetic,
                    },
                    wav::chapter(),
                )
                .await?;
            Ok(c::StageOutput {
                outcome: c::StageOutcome::Complete,
                artifacts: vec![file],
                decisions: vec![],
                findings: vec![],
                cache_keys: vec![],
                library_updates: vec![],
                package_candidate: None,
            })
        })
    }
}
#[tokio::test]
async fn chapter_master_http_is_hash_bound_and_supports_bounded_ranges() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let request = json!({"schema_version":1,"source_id":"pg-55-chapter-1","book_id":"pg-55","mode":"fixture","pins":{"recipe_sha256":c::digest(b"master-http"),"profile_sha256":c::digest(b"profile"),"profile_id":"diagnostic"},"limits":{"money_microusd":0,"requests":0,"tokens":0,"repairs":0,"wall_time_ms":60000}});
    let id = store.create("master-http", &request).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_, epoch) = store.claim().unwrap().unwrap();
    let a = pt_production::runtime::Runtime::new(store.clone(), Arc::new(Producer))
        .execute_stage(&id, epoch, c::Stage::MusicMix)
        .await
        .unwrap()
        .remove(0);
    let app = router(App::new(store, 8787, None, None));
    let get = |range: Option<&str>| {
        let mut r = Request::builder()
            .uri(format!("/api/v1/artifacts/{}", a.id))
            .header("host", "127.0.0.1:8787");
        if let Some(range) = range {
            r = r.header("range", range);
        }
        r.body(Body::empty()).unwrap()
    };
    let response = app.clone().oneshot(get(Some("bytes=0-43"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()["content-range"], "bytes 0-43/34560044");
    assert_eq!(response.headers()["content-length"], "44");
    assert_eq!(response.headers()["etag"], format!("\"{}\"", a.file.sha256));
    assert_eq!(
        &to_bytes(response.into_body(), 100).await.unwrap()[..],
        &wav::header(34_560_044)
    );
    let response = app.clone().oneshot(get(None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-length"], "34560044");
    assert_eq!(response.headers()["content-type"], "audio/wav");
    a.file
        .verify(&to_bytes(response.into_body(), 34_560_045).await.unwrap())
        .unwrap();
    let response = app.oneshot(get(Some("bytes=34560044-"))).await.unwrap();
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(response.headers()["content-range"], "bytes */34560044");
}

#[tokio::test]
async fn diagnostic_master_streams_only_exact_inventoried_wav_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("fixture");
    std::fs::create_dir_all(fixture.join("audio")).unwrap();
    let bytes = wav::chapter();
    let mut manifest: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/shared/two-scene-v1/manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    manifest["entries"]["audio"] = json!({"path":"audio/master.wav","sha256":c::digest(&bytes),"bytes":bytes.len(),"media_type":"audio/wav"});
    std::fs::write(
        fixture.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(fixture.join("audio/master.wav"), &bytes).unwrap();
    let app = router(App::new(
        Store::open(&temp.path().join("store")).unwrap(),
        8787,
        None,
        Some(fixture.clone()),
    ));
    let get = || {
        Request::builder()
            .uri("/diagnostic/audio/master.wav")
            .header("host", "127.0.0.1:8787")
            .header("range", "bytes=-4")
            .body(Body::empty())
            .unwrap()
    };
    let response = app.clone().oneshot(get()).await.unwrap();
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()["content-type"], "audio/wav");
    assert_eq!(
        &to_bytes(response.into_body(), 4).await.unwrap()[..],
        &bytes[bytes.len() - 4..]
    );
    // A valid WAV with changed PCM bytes must fail its inventory hash before headers.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.join("audio/master.wav"))
        .unwrap();
    use std::io::{Seek, SeekFrom, Write};
    file.seek(SeekFrom::End(-1)).unwrap();
    file.write_all(&[9]).unwrap();
    assert_eq!(
        app.clone().oneshot(get()).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    file.set_len(44).unwrap();
    assert_eq!(
        app.oneshot(get()).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn chapter_http_keeps_json_and_request_limits_tight() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("fixture");
    std::fs::create_dir(&fixture).unwrap();
    let mut manifest: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/shared/two-scene-v1/manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for (name, mime, size) in [
        ("oversized.wav", "audio/wav", 256 * 1024 * 1024 + 4),
        ("oversized.json", "application/json", 32 * 1024 * 1024 + 1),
    ] {
        std::fs::File::create(fixture.join(name))
            .unwrap()
            .set_len(size)
            .unwrap();
        manifest["assets"].as_array_mut().unwrap().push(
            json!({"path":name,"sha256":c::digest(b"unused"),"bytes":size,"media_type":mime}),
        );
    }
    std::fs::write(
        fixture.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let app = router(App::new(
        Store::open(&temp.path().join("store")).unwrap(),
        8787,
        None,
        Some(fixture),
    ));
    for name in ["oversized.wav", "oversized.json"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/diagnostic/{name}"))
                    .header("host", "127.0.0.1:8787")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let error: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024).await.unwrap()).unwrap();
        assert_eq!(error["code"], "artifact_too_large");
    }
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runs")
                .header("host", "127.0.0.1:8787")
                .header("origin", "http://127.0.0.1:8787")
                .header("content-type", "application/json")
                .header("idempotency-key", "large-json")
                .body(Body::from(vec![b' '; 65537]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
