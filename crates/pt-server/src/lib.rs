use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use pt_production::runtime::{Error, Store};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    convert::Infallible,
    path::{Path as FsPath, PathBuf},
    time::Duration,
};
#[derive(Clone)]
pub struct App {
    pub store: Store,
    pub port: u16,
    studio: Option<PathBuf>,
    fixture: Option<PathBuf>,
}
impl App {
    pub fn new(store: Store, port: u16, studio: Option<PathBuf>, fixture: Option<PathBuf>) -> Self {
        Self {
            store,
            port,
            studio,
            fixture,
        }
    }
}
struct ApiError(Error);
impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            "not_found" => StatusCode::NOT_FOUND,
            "idempotency_conflict" | "library_conflict" => StatusCode::CONFLICT,
            "live_disabled" | "invalid_origin" => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_REQUEST,
        };
        (
            status,
            Json(json!({"code":self.0.code,"message":safe_message(self.0.code),"retryable":false})),
        )
            .into_response()
    }
}
fn safe_message(code: &str) -> &str {
    match code {
        "idempotency_conflict" => "Idempotency key belongs to another request.",
        "not_found" => "Record not found.",
        "snapshot_required" => "Fetch a fresh snapshot.",
        "live_disabled" => "Live dispatch is unavailable; select fixture mode.",
        "invalid_origin" => "Same-origin loopback access required.",
        _ => "Request could not be completed.",
    }
}
fn error(code: &'static str) -> ApiError {
    ApiError(Error {
        code,
        message: safe_message(code).into(),
    })
}
async fn db<T: Send + 'static>(
    f: impl FnOnce() -> pt_production::runtime::Result<T> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| error("database_error"))?
        .map_err(Into::into)
}
pub fn router(app: App) -> Router {
    Router::new()
        .route("/api/v1/books", get(books))
        .route("/api/v1/runs", get(history).post(create))
        .route("/api/v1/runs/{id}", get(snapshot))
        .route("/api/v1/runs/{id}/cancel", post(cancel))
        .route("/api/v1/runs/{id}/diagnostic", get(diagnostic_ref))
        .route("/api/v1/runs/{id}/package/{*path}", get(package_file))
        .route("/api/v1/runs/{id}/journal", get(journal))
        .route("/api/v1/runs/{id}/events", get(events))
        .route("/api/v1/artifacts/{id}", get(artifact))
        .route("/diagnostic/{*path}", get(diagnostic))
        .route("/", get(index))
        .route("/{*path}", get(static_file))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(app.clone(), origin))
        .with_state(app)
}
async fn origin(State(app): State<App>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|s| s.to_str().ok());
    let allowed = [
        format!("127.0.0.1:{}", app.port),
        format!("localhost:{}", app.port),
    ];
    if !host.is_some_and(|host| allowed.iter().any(|s| s == host)) {
        return error("invalid_origin").into_response();
    }
    if request
        .headers()
        .get(header::ORIGIN)
        .is_some_and(|v| v.to_str().ok() != host.map(|h| format!("http://{h}")).as_deref())
        || request
            .headers()
            .get("sec-fetch-site")
            .is_some_and(|v| v == "cross-site")
    {
        return error("invalid_origin").into_response();
    }
    let mut response = next.run(request).await;
    let h = response.headers_mut();
    h.insert("x-content-type-options", "nosniff".parse().unwrap());
    h.insert("content-security-policy","default-src 'self'; img-src 'self' blob: data:; media-src 'self' blob:; style-src 'self' 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
    h.insert("cache-control", "no-store".parse().unwrap());
    response
}
async fn books(State(app): State<App>) -> Json<Value> {
    let diagnostic_pins=app.fixture.as_ref().and_then(|root|std::fs::read(root.join("profile.json")).ok()).and_then(|bytes|serde_json::from_slice::<Value>(&bytes).ok().map(|profile|json!({"recipe_sha256":pt_contracts::digest(b"saved-diagnostic-v1"),"profile_sha256":pt_contracts::digest(&bytes),"profile_id":profile["id"]})));
    let diagnostic=app.fixture.and_then(|root|std::fs::read(root.join("manifest.json")).ok()).map(|bytes|json!({"manifest_path":"/diagnostic/manifest.json","manifest_sha256":pt_contracts::digest(&bytes)}));
    Json(
        json!({"books":[{"id":"pg-55","title":"The Wonderful Wizard of Oz","source_id":"pg-55-chapter-1","chapter":1},{"id":"pg-36","title":"The War of the Worlds","source_id":"pg-36-book-1-chapter-1","part":1,"chapter":1}],"execution_modes":["fixture"],"provenance":"synthetic","diagnostic":diagnostic,"diagnostic_pins":diagnostic_pins}),
    )
}
async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<pt_contracts::RunSnapshot>), ApiError> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| error("invalid_idempotency_key"))?
        .to_owned();
    let value = db(move || {
        let run = app.store.create(&key, &body)?;
        app.store.api_snapshot(run["id"].as_str().unwrap())
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(value)))
}
#[derive(Deserialize, Default)]
struct Page {
    after: Option<i64>,
    before: Option<i64>,
    limit: Option<usize>,
}
async fn history(
    State(app): State<App>,
    Query(page): Query<Page>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        db(move || {
            app.store
                .api_history(page.before.unwrap_or(i64::MAX), page.limit.unwrap_or(30))
        })
        .await?,
    ))
}
async fn snapshot(
    State(app): State<App>,
    Path(id): Path<String>,
) -> Result<Json<pt_contracts::RunSnapshot>, ApiError> {
    Ok(Json(db(move || app.store.api_snapshot(&id)).await?))
}
async fn cancel(
    State(app): State<App>,
    Path(id): Path<String>,
) -> Result<Json<pt_contracts::RunSnapshot>, ApiError> {
    Ok(Json(
        db(move || {
            app.store.cancel(&id)?;
            app.store.api_snapshot(&id)
        })
        .await?,
    ))
}
async fn journal(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(page): Query<Page>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(
        json!({"events":db(move||app.store.api_journal(&id,page.after.unwrap_or(0),page.limit.unwrap_or(500))).await?}),
    ))
}
async fn events(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(page): Query<Page>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let cursor = match headers.get("last-event-id") {
        Some(v) => v
            .to_str()
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(-1),
        None => page.after.unwrap_or(0),
    };
    let check = app.store.clone();
    let run = id.clone();
    db(move || check.api_snapshot(&run)).await?;
    let stream = async_stream::stream! {
     let mut after=cursor;
     loop{
      let store=app.store.clone();let run=id.clone();
      let batch=tokio::task::spawn_blocking(move||store.api_journal(&run,after,500)).await;
      match batch{
       Ok(Ok(batch))=>{for event in batch{after=event.seq as i64;yield Ok(Event::default().id(event.seq.to_string()).json_data(event).unwrap());}}
       _=>{yield Ok(Event::default().event("snapshot_required").data("{}"));break;}
      }
      let store=app.store.clone();let run=id.clone();let snapshot=tokio::task::spawn_blocking(move||store.api_snapshot(&run)).await;
      if let Ok(Ok(s))=snapshot{if matches!(s.state,pt_contracts::RunState::Succeeded|pt_contracts::RunState::Failed|pt_contracts::RunState::Cancelled) && after>=s.last_event_seq as i64{break;}}
      // Notifications are hints. A bounded durable poll closes the query/subscribe race.
      tokio::select!{_=app.store.changed()=>{},_=tokio::time::sleep(Duration::from_millis(250))=>{}}
     }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(10))))
}
async fn diagnostic_ref(
    State(app): State<App>,
    Path(id): Path<String>,
) -> Result<Json<Option<pt_contracts::ReleaseRef>>, ApiError> {
    Ok(Json(
        db(move || {
            app.store.api_snapshot(&id)?;
            app.store.diagnostic_ref(&id)
        })
        .await?,
    ))
}
async fn package_file(
    State(app): State<App>,
    Path((id, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let (bytes, mime, hash) = db(move || app.store.package_artifact(&id, &path)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::ETAG, format!("\"{hash}\"")),
        ],
        Body::from(bytes),
    )
        .into_response())
}
async fn artifact(State(app): State<App>, Path(id): Path<String>) -> Result<Response, ApiError> {
    let (bytes, mime, hash) = db(move || app.store.artifact(&id)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::ETAG, format!("\"{hash}\"")),
        ],
        Body::from(bytes),
    )
        .into_response())
}
async fn index(State(app): State<App>) -> Result<Response, ApiError> {
    serve_static(app.studio, "index.html".into()).await
}
async fn static_file(
    State(app): State<App>,
    Path(path): Path<String>,
) -> Result<Response, ApiError> {
    if path.starts_with("api/") {
        return Err(error("not_found"));
    }
    serve_static(app.studio, path).await
}
async fn serve_static(root: Option<PathBuf>, path: String) -> Result<Response, ApiError> {
    let root = root.ok_or_else(|| error("not_found"))?;
    let mime = match FsPath::new(&path).extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => return Err(error("not_found")),
    };
    let bytes = db(move || read_file(&root, &path, 16 * 1024 * 1024)).await?;
    Ok(([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response())
}
async fn diagnostic(
    State(app): State<App>,
    Path(path): Path<String>,
) -> Result<Response, ApiError> {
    let root = app.fixture.ok_or_else(|| error("not_found"))?;
    let bytes = db(move || {
        let manifest_bytes = read_file(&root, "manifest.json", 1024 * 1024)?;
        let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
        let mut allowed = vec!["manifest.json".to_owned()];
        fn collect(v: &Value, allowed: &mut Vec<String>) {
            match v {
                Value::Object(map) => {
                    if let Some(path) = map.get("path").and_then(Value::as_str) {
                        if map.contains_key("sha256") && map.contains_key("bytes") {
                            allowed.push(path.to_owned());
                        }
                    }
                    for v in map.values() {
                        collect(v, allowed);
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        collect(v, allowed)
                    }
                }
                _ => {}
            }
        }
        collect(&manifest, &mut allowed);
        if !allowed.contains(&path) {
            return Err(Error {
                code: "not_found",
                message: "fixture path is not inventoried".into(),
            });
        }
        read_file(&root, &path, 32 * 1024 * 1024)
    })
    .await?;
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Body::from(bytes),
    )
        .into_response())
}
fn read_file(root: &FsPath, path: &str, limit: u64) -> pt_production::runtime::Result<Vec<u8>> {
    if !pt_contracts::safe_path(path) {
        return Err(Error {
            code: "not_found",
            message: "path denied".into(),
        });
    }
    let root = root.canonicalize()?;
    let file = root.join(path).canonicalize()?;
    if !file.starts_with(&root) || !file.is_file() || file.metadata()?.len() > limit {
        return Err(Error {
            code: "not_found",
            message: "file denied".into(),
        });
    }
    Ok(std::fs::read(file)?)
}
#[cfg(test)]
mod tests;
