//! An administrative recipe fixes executable/arguments; model JSON is stdin data only.
use super::{Error, Result};
use std::{path::PathBuf, process::Stdio};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
#[derive(Clone)]
pub struct Recipe {
    pub name: String,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub max_input: usize,
    pub max_output: usize,
    pub max_decoded_media: usize,
    pub timeout_ms: u64,
}
pub struct HelperOutput {
    pub stdout: Vec<u8>,
    pub stderr_bytes: usize,
}
async fn bounded<R: AsyncRead + Unpin>(reader: R, max: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(max as u64 + 1).read_to_end(&mut bytes).await?;
    Ok(bytes)
}
pub(crate) async fn execute(
    recipe: &Recipe,
    attempt_id: &str,
    input: &[u8],
    decoded_media: usize,
    cancelled: impl FnMut() -> bool,
) -> Result<HelperOutput> {
    execute_observed(recipe, attempt_id, input, decoded_media, cancelled, |_| {
        Ok(())
    })
    .await
}
pub(crate) async fn execute_observed(
    recipe: &Recipe,
    attempt_id: &str,
    input: &[u8],
    decoded_media: usize,
    mut cancelled: impl FnMut() -> bool,
    mut spawned: impl FnMut(u32) -> Result<()>,
) -> Result<HelperOutput> {
    if recipe.name.is_empty()
        || !recipe.executable.is_absolute()
        || input.len() > recipe.max_input
        || decoded_media > recipe.max_decoded_media
        || recipe.max_output > 32 * 1024 * 1024
        || !(1..=300_000).contains(&recipe.timeout_ms)
        || attempt_id.len() != 64
    {
        return Err(Error::new(
            "helper_bounds",
            "invalid bounded helper envelope",
        ));
    }
    if cancelled() {
        return Err(Error::new("stale_epoch", "helper cancelled"));
    }
    let mut command = Command::new(&recipe.executable);
    command
        .args(&recipe.arguments)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("PT_ATTEMPT_ID", attempt_id)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let pid = child.id();
    if let Some(pid) = pid {
        if let Err(error) = spawned(pid) {
            #[cfg(unix)]
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await;
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(error);
        }
    }
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let task = async {
        let send = async move {
            stdin.write_all(input).await?;
            stdin.shutdown().await?;
            drop(stdin);
            Ok::<_, std::io::Error>(())
        };
        let (sent, out, err) = tokio::join!(
            send,
            bounded(stdout, recipe.max_output),
            bounded(stderr, 8192)
        );
        sent?;
        Ok::<_, std::io::Error>((out?, err?))
    };
    tokio::pin!(task);
    let deadline = tokio::time::sleep(std::time::Duration::from_millis(recipe.timeout_ms));
    tokio::pin!(deadline);
    let result = loop {
        tokio::select! {
            result=&mut task=>break result.map_err(Error::from),
            _=&mut deadline=>break Err(Error::new("helper_timeout","bounded helper exceeded deadline")),
            _=tokio::time::sleep(std::time::Duration::from_millis(25))=>if cancelled(){break Err(Error::new("stale_epoch","helper cancelled"));}
        }
    };
    let bytes = match result {
        Ok((out, err)) if out.len() <= recipe.max_output && err.len() <= 8192 => {
            Ok((out, err.len()))
        }
        Ok(_) => Err(Error::new(
            "helper_output_limit",
            "helper exceeded output bounds",
        )),
        Err(e) => Err(e),
    };
    // Kill the complete helper process group, including descendants retaining pipes.
    #[cfg(unix)]
    if let Some(pid) = pid {
        let _ = Command::new("/bin/kill")
            .arg("-KILL")
            .arg("--")
            .arg(format!("-{pid}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
    }
    if bytes.is_err() {
        let _ = child.kill().await;
    }
    let status = child.wait().await?;
    let (stdout, stderr_bytes) = bytes?;
    if !status.success() {
        return Err(Error::new(
            "helper_failed",
            "helper failed; raw stderr is not exposed",
        ));
    }
    Ok(HelperOutput {
        stdout,
        stderr_bytes,
    })
}

#[derive(Clone)]
pub(crate) struct OfflineAcoustic {
    directory: PathBuf,
    pub(crate) sha256: String,
}
impl OfflineAcoustic {
    pub(crate) fn new(directory: PathBuf) -> Result<Self> {
        let mut bytes = std::fs::read(directory.join("helper.py"))?;
        bytes.extend(std::fs::read(directory.join("adapters.py"))?);
        Ok(Self {
            directory,
            sha256: super::digest(&bytes),
        })
    }
}
pub(crate) async fn acoustic(
    store: &super::Store,
    run: &str,
    epoch: i64,
    mode: pt_contracts::ExecutionMode,
    config: &OfflineAcoustic,
    op: &pt_contracts::HelperOperation,
) -> Result<Vec<u8>> {
    use super::store::{digest, fail, now};
    use rusqlite::{OptionalExtension, params};
    use serde_json::{Value, json};
    if mode != pt_contracts::ExecutionMode::Fixture
        || op.kind != pt_contracts::HelperKind::AlignAudio
        || op.id.is_empty()
        || op.id.len() > 128
        || op.inputs.len() > 128
        || op.timeout_ms == 0
        || op.timeout_ms > 30000
        || op.max_output_bytes == 0
        || op.max_output_bytes > 16 * 1024 * 1024
        || op.recipe_sha256 != config.sha256
        || OfflineAcoustic::new(config.directory.clone())?.sha256 != config.sha256
    {
        return fail(
            "helper_forbidden",
            "offline helper recipe/envelope is not authorized",
        );
    }
    let operation = format!("helper:{}", op.id);
    let id = digest(format!("{run}:{operation}").as_bytes());
    let descriptor = digest(&serde_json::to_vec(op)?);
    let cached=store.transaction(|tx|{super::Store::active(tx,run,epoch)?;
        let prior:Option<(String,String,Option<String>)>=tx.query_row("SELECT descriptor_hash,state,response FROM attempts WHERE run=?1 AND operation=?2",params![run,operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((hash,state,response))=prior {if hash!=descriptor{return fail("operation_conflict","helper identity is already bound");}if state!="completed"{return fail("attempt_unresolved","helper attempt needs recovery evidence");}return Ok(response);}
        tx.execute("INSERT INTO attempts(id,run,epoch,operation,request_hash,descriptor_hash,scope,state,money,requests,tokens,repair,created) VALUES(?1,?2,?3,?4,?5,?5,'offline/product/acoustic','dispatched',0,0,0,0,?6)",params![id,run,epoch,operation,descriptor,now()])?;
        super::Store::emit(tx,run,"helper_authorized",&json!({"attempt_id":id,"recipe_sha256":config.sha256,"money":0,"requests":0,"tokens":0,"repair":false}))?;Ok(None)
    })?;
    if let Some(cached) = cached {
        let response: Value = serde_json::from_str(&cached)?;
        return store.read_object(
            response["sha256"]
                .as_str()
                .ok_or_else(|| Error::new("corrupt_artifact", "helper response hash missing"))?,
            response["bytes"]
                .as_u64()
                .ok_or_else(|| Error::new("corrupt_artifact", "helper response size missing"))?
                as usize,
        );
    }
    let scratch = tempfile::Builder::new()
        .prefix("helper-")
        .tempdir_in(store.root.join("scratch"))?;
    let mut total = 0usize;
    for input in &op.inputs {
        input
            .file
            .validate()
            .map_err(|e| Error::new("invalid_input", e.to_string()))?;
        let bytes = store
            .artifact_bounded(&input.id, super::media::MAX_ARTIFACT_BYTES)?
            .0;
        input
            .file
            .verify(&bytes)
            .map_err(|e| Error::new("invalid_input", e.to_string()))?;
        total = total.saturating_add(bytes.len());
        if total > 64 * 1024 * 1024 {
            return fail("helper_bounds", "helper decoded PCM/input limit exceeded");
        }
        let path = scratch.path().join(&input.file.path);
        std::fs::create_dir_all(path.parent().unwrap())?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(&bytes)?;
    }
    let input = serde_json::to_vec(&json!({"operation":"replay_acoustic","root":scratch.path()}))?;
    let recipe = Recipe {
        name: "offline-acoustic-replay-v1".into(),
        executable: "/usr/bin/python3".into(),
        arguments: vec![
            "-E".into(),
            "-s".into(),
            "-S".into(),
            "-c".into(),
            format!(
                "import os,resource,runpy,sys; resource.setrlimit(resource.RLIMIT_CPU,({0},{0})); sys.platform=='darwin' or resource.setrlimit(resource.RLIMIT_AS,(268435456,268435456)); resource.setrlimit(resource.RLIMIT_FSIZE,(16777216,16777216)); resource.setrlimit(resource.RLIMIT_NOFILE,(64,64)); sys.path.insert(0,os.path.dirname(sys.argv[1])); runpy.run_path(sys.argv[1],run_name='__main__')",
                (op.timeout_ms / 1000 + 1).max(1)
            ),
            config
                .directory
                .join("helper.py")
                .canonicalize()?
                .to_str()
                .ok_or_else(|| Error::new("invalid_path", "UTF-8 helper path required"))?
                .into(),
        ],
        max_input: 65536,
        max_output: op.max_output_bytes as usize,
        max_decoded_media: 64 * 1024 * 1024,
        timeout_ms: op.timeout_ms,
    };
    let worker_store = store.clone();
    let worker_run = run.to_owned();
    let worker_id = id.clone();
    // Detach only the bounded runner: cancellation still kills its group and records the attempt.
    let worker = tokio::spawn(async move {
        let _scratch = scratch;
        let result = execute(&recipe, &worker_id, &input, total, || {
            !worker_store.is_active(&worker_run, epoch).unwrap_or(false)
        })
        .await;
        let stored = match result {
            Ok(output) => {
                let hash = worker_store.put(&output.stdout)?;
                let size = output.stdout.len();
                Ok((
                    output.stdout,
                    json!({"sha256":hash,"bytes":size,"stderr_bytes":output.stderr_bytes}),
                ))
            }
            Err(e) => Err(e),
        };
        let receipt = match &stored {
            Ok((bytes, response)) => {
                let mut response = response.clone();
                response["bytes"] = json!(bytes.len());
                response
            }
            Err(e) => json!({"code":e.code}),
        };
        worker_store.transaction(|tx| {
            let state = if stored.is_ok() {
                "completed"
            } else {
                "rejected"
            };
            tx.execute(
                "UPDATE attempts SET state=?2,response=?3 WHERE id=?1",
                params![worker_id, state, serde_json::to_string(&receipt)?],
            )?;
            super::Store::emit(
                tx,
                &worker_run,
                if stored.is_ok() {
                    "helper_completed"
                } else {
                    "helper_failed"
                },
                &json!({"attempt_id":worker_id,"receipt":receipt}),
            )?;
            Ok(())
        })?;
        stored.map(|(bytes, _)| bytes)
    });
    worker
        .await
        .map_err(|_| Error::new("helper_failed", "bounded helper task failed"))?
}
