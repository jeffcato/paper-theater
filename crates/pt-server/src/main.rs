use pt_production::runtime::{FixturePipeline, Runtime, Store};
use std::{path::PathBuf, sync::Arc};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = PathBuf::from("var");
    let mut port = 8787u16;
    let mut studio = PathBuf::from("apps/studio/dist");
    let mut fixture = PathBuf::from("fixtures/shared/two-scene-v1");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("option value required")?;
        match arg.as_str() {
            "--data" => data = value.into(),
            "--port" => port = value.parse()?,
            "--studio" => studio = value.into(),
            "--fixture" => fixture = value.into(),
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
    .with_offline_acoustic(PathBuf::from("tools/generation"))?;
    runtime.fixture_delay_ms = 120;
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
