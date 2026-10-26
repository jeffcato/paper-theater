//! An administrative recipe fixes executable/arguments; model JSON is stdin data only.
use super::{Error,Result};
use std::{path::PathBuf,process::Stdio};
use tokio::{io::{AsyncRead,AsyncReadExt,AsyncWriteExt},process::Command};
#[derive(Clone)]
pub struct Recipe { pub name:String, pub executable:PathBuf, pub arguments:Vec<String>, pub max_input:usize, pub max_output:usize, pub max_decoded_media:usize, pub timeout_ms:u64 }
pub struct HelperOutput { pub stdout:Vec<u8>, pub stderr_bytes:usize }
async fn bounded<R:AsyncRead+Unpin>(reader:R,max:usize)->std::io::Result<Vec<u8>> {let mut bytes=Vec::new();reader.take(max as u64+1).read_to_end(&mut bytes).await?;Ok(bytes)}
pub async fn execute(recipe:&Recipe,attempt_id:&str,input:&[u8],decoded_media:usize,mut cancelled:impl FnMut()->bool)->Result<HelperOutput> {
    if recipe.name.is_empty() || !recipe.executable.is_absolute() || input.len()>recipe.max_input || decoded_media>recipe.max_decoded_media || recipe.max_output>32*1024*1024 || !(1..=300_000).contains(&recipe.timeout_ms) || attempt_id.len()!=64 {return Err(Error::new("helper_bounds","invalid bounded helper envelope"));}
    if cancelled(){return Err(Error::new("stale_epoch","helper cancelled"));}
    let mut command=Command::new(&recipe.executable);
    command.args(&recipe.arguments).env_clear().env("PATH","/usr/bin:/bin").env("PT_ATTEMPT_ID",attempt_id).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(unix)] command.process_group(0);
    let mut child=command.spawn()?;let pid=child.id();
    let mut stdin=child.stdin.take().unwrap();let stdout=child.stdout.take().unwrap();let stderr=child.stderr.take().unwrap();
    let task=async {
        let send=async {stdin.write_all(input).await?;stdin.shutdown().await};
        let (sent,out,err)=tokio::join!(send,bounded(stdout,recipe.max_output),bounded(stderr,8192));
        sent?;Ok::<_,std::io::Error>((out?,err?))
    };
    tokio::pin!(task);
    let deadline=tokio::time::sleep(std::time::Duration::from_millis(recipe.timeout_ms));tokio::pin!(deadline);
    let result=loop {tokio::select!{
        result=&mut task=>break result.map_err(Error::from),
        _=&mut deadline=>break Err(Error::new("helper_timeout","bounded helper exceeded deadline")),
        _=tokio::time::sleep(std::time::Duration::from_millis(25))=>if cancelled(){break Err(Error::new("stale_epoch","helper cancelled"));}
    }};
    let bytes=match result {Ok((out,err)) if out.len()<=recipe.max_output && err.len()<=8192=>Ok((out,err.len())),Ok(_)=>Err(Error::new("helper_output_limit","helper exceeded output bounds")),Err(e)=>Err(e)};
    // Kill the complete helper process group, including descendants retaining pipes.
    #[cfg(unix)] if let Some(pid)=pid {let _=Command::new("/bin/kill").arg("-KILL").arg("--").arg(format!("-{pid}")).stdout(Stdio::null()).stderr(Stdio::null()).status().await;}
    if bytes.is_err(){let _=child.kill().await;}
    let status=child.wait().await?;
    let (stdout,stderr_bytes)=bytes?;
    if !status.success(){return Err(Error::new("helper_failed","helper failed; raw stderr is not exposed"));}
    Ok(HelperOutput{stdout,stderr_bytes})
}
