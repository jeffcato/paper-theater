use pt_production::runtime::{FixturePipeline, Runtime, Store};
use std::{path::PathBuf, sync::Arc};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = PathBuf::from("var");
    let mut port = 8787u16;
    let mut studio = PathBuf::from("apps/studio/dist");
    let mut fixture = PathBuf::from("fixtures/shared/two-scene-v1");
    let mut acceptance_player_registry: Option<PathBuf> = None;
    let mut chapter_inputs: Option<PathBuf> = None;
    let mut direction_pilot_inputs: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("option value required")?;
        match arg.as_str() {
            "--data" => data = value.into(),
            "--port" => port = value.parse()?,
            "--studio" => studio = value.into(),
            "--fixture" => fixture = value.into(),
            "--acceptance-player-registry" => acceptance_player_registry = Some(value.into()),
            "--chapter-inputs" => chapter_inputs = Some(value.into()),
            "--direction-pilot-inputs" => direction_pilot_inputs = Some(value.into()),
            _ => return Err("unknown option".into()),
        }
    }
    std::fs::create_dir_all(&data)?;
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data.join("server.lock"))?;
    lock.try_lock()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let port = listener.local_addr()?.port();
    let store = Store::open(&data)?;
    store.recover()?;
    let mut runtime = Runtime::new(
        store.clone(),
        Arc::new(FixturePipeline::new(Arc::new(
            pt_production::generation::GenerationExecutor,
        ))),
    )
    .with_saved_inputs(fixture.clone(), PathBuf::from("fixtures/generation/source"))
    .with_acoustic_inputs(PathBuf::from("fixtures/generation/acoustic"))
    .with_presentation_inputs(PathBuf::from("."))
    .with_offline_acoustic(PathBuf::from("tools/generation"))?;
    if let Some(path) = acceptance_player_registry {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct FrozenPlayerRegistry {
            file: pt_contracts::FileRef,
            player_version: String,
        }
        if path.is_symlink() {
            return Err("player registry must be a regular trusted configuration file".into());
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(&path)?
            .take(64 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 64 * 1024 {
            return Err("player registry exceeds its metadata envelope".into());
        }
        let registry: FrozenPlayerRegistry = serde_json::from_slice(&bytes)?;
        let root = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        runtime = runtime.with_acceptance_player(root, registry.file, registry.player_version);
    }
    runtime.fixture_delay_ms = 120;
    if let Some(root) = direction_pilot_inputs {
        runtime = runtime.with_saved_direction_pilot(root);
    }
    if let Some(root) = chapter_inputs {
        let stage_revision = pt_production::runtime::digest(
            &[
                include_bytes!("../../pt-production/src/generation/chapter_import.rs").as_slice(),
                include_bytes!("../../pt-production/src/generation/mod.rs"),
                include_bytes!("../../pt-production/src/generation/inputs.rs"),
                pt_contracts::acceptance_binding_checker().sha256.as_bytes(),
            ]
            .concat(),
        );
        runtime = runtime
            .with_chapter_helper(pt_production::runtime::ChapterHelper::new(
                PathBuf::from("tools/generation"),
                stage_revision,
            )?)
            .with_chapter_inputs(root);
    }
    let worker = tokio::spawn(async move {
        loop {
            if let Err(e) = runtime.work_once().await {
                eprintln!("runtime: {}", e.code);
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });
    let app = pt_server::router(pt_server::App::new(
        store,
        port,
        Some(studio),
        Some(fixture),
    ));
    println!("Paper Theater: http://127.0.0.1:{port} (synthetic fixture mode)");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    worker.abort();
    drop(lock);
    Ok(())
}
