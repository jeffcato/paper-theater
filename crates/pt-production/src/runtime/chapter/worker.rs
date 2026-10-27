//! Only the separately pinned G21 closure can be dispatched here.
use super::*;
use std::io::Write;
pub(super) async fn run(
    service: &services::ChapterServices,
    grant: &services::Grant,
    op: &c::HelperOperation,
    epoch: i64,
) -> Result<Vec<u8>> {
    let code = grant.config.code()?;
    if op.inputs.len() + 4 > 64
        || op.inputs.iter().any(|a| {
            a.file.bytes > FILE || (a.file.media_type == "application/json" && a.file.bytes > META)
        })
        || op.inputs.iter().map(|a| a.file.bytes).sum::<u64>() + 4 * META > TOTAL
    {
        return fail("chapter_bounds", "helper allowlist/envelope exceeded");
    }
    let operation = format!("helper:{}", op.id);
    let id = digest(format!("{}:{operation}", service.run).as_bytes());
    let descriptor = digest(&serde_json::to_vec(&(
        grant.unit_key.as_str(),
        grant.revision,
        op,
    ))?);
    let cached=service.store.transaction(|tx| {
        Store::active(tx,&service.run,epoch)?;
        let previous:Option<(String,String,Option<String>)>=tx.query_row("SELECT descriptor_hash,state,response FROM attempts WHERE run=?1 AND operation=?2",params![service.run,operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some((hash,state,response))=previous {
            if hash!=descriptor {return fail("operation_conflict","chapter helper descriptor changed");}
            if state!="completed" {return fail("attempt_unresolved","chapter helper attempt requires recovery evidence");}
            return response.map(Some).ok_or_else(||Error::new("chapter_attempt_corrupt","completed chapter helper response is missing"));
        }
        tx.execute("INSERT INTO attempts(id,run,epoch,operation,request_hash,descriptor_hash,scope,state,money,requests,tokens,repair,created) VALUES(?1,?2,?3,?4,?5,?5,'offline/product/chapter-import-v2','reserved',0,0,0,0,?6)",params![id,service.run,epoch,operation,descriptor,now()])?;
        Store::emit(tx,&service.run,"chapter_helper_authorized",&json!({"attempt_id":id,"unit_key":grant.unit_key,"revision":grant.revision,"recipe_sha256":RECIPE,"provider_calls":0}))?;Ok(None)
    })?;
    if let Some(encoded) = cached {
        let value: Value = serde_json::from_str(&encoded)?;
        let f: c::FileRef = serde_json::from_value(value["file"].clone())?;
        return verify::cas_bytes(&service.store, &f, META);
    }
    let data = tempfile::Builder::new()
        .prefix("chapter-helper-")
        .tempdir_in(service.store.root.join("scratch"))?;
    let code_root = tempfile::Builder::new()
        .prefix("chapter-code-")
        .tempdir_in(service.store.root.join("scratch"))?;
    for (name, body) in code {
        std::fs::write(code_root.path().join(name), body)?;
    }
    let total = op.inputs.iter().map(|a| a.file.bytes).sum::<u64>() as usize;
    for a in &op.inputs {
        active(&service.store, &service.run, epoch)?;
        let bytes = service.bytes(&a.file)?;
        let p = data.path().join(&a.file.path);
        std::fs::create_dir_all(p.parent().unwrap())?;
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(p)?;
        f.write_all(&bytes)?;
    }
    let input = serde_json::to_vec(
        &json!({"operation":"verify_imported_chapter_unit_v2","root":data.path(),"unit":"chapter-import-unit.json"}),
    )?;
    let wrapper = format!(
        "import sys;sys.path=[p for p in sys.path if p and (p.startswith(sys.base_prefix+'/') or p.startswith(sys.base_exec_prefix+'/'))];import os,resource,runpy;sys.dont_write_bytecode=True;resource.setrlimit(resource.RLIMIT_CPU,({0},{0}));sys.platform=='darwin' or resource.setrlimit(resource.RLIMIT_AS,(268435456,268435456));resource.setrlimit(resource.RLIMIT_FSIZE,(1048576,1048576));resource.setrlimit(resource.RLIMIT_NOFILE,(64,64));sys.path.insert(0,os.path.dirname(sys.argv[1]));runpy.run_path(sys.argv[1],run_name='__main__')",
        (op.timeout_ms / 1000 + 1).max(1)
    );
    let recipe = super::super::helper::Recipe {
        name: "offline-chapter-import-unit-verification-v2".into(),
        executable: "/usr/bin/python3".into(),
        arguments: vec![
            "-E".into(),
            "-s".into(),
            "-S".into(),
            "-c".into(),
            wrapper,
            code_root
                .path()
                .join("chapter_import_helper.py")
                .to_string_lossy()
                .into_owned(),
        ],
        max_input: 65536,
        max_output: op.max_output_bytes as usize,
        max_decoded_media: TOTAL as usize,
        timeout_ms: op.timeout_ms,
    };
    let store = service.store.clone();
    let run = service.run.clone();
    let worker = tokio::spawn(async move {
        let _scratch = (data, code_root);
        let result = super::super::helper::execute_observed(
            &recipe,
            &id,
            &input,
            total,
            || !store.is_active(&run, epoch).unwrap_or(false),
            |pid| {
                store.transaction(|tx| {
                    Store::active(tx, &run, epoch)?;
                    tx.execute(
                        "UPDATE attempts SET state='dispatched',response=?2 WHERE id=?1",
                        params![id, serde_json::to_string(&json!({"pid":pid}))?],
                    )?;
                    Store::emit(
                        tx,
                        &run,
                        "chapter_helper_started",
                        &json!({"attempt_id":id,"pid":pid}),
                    )?;
                    Ok(())
                })
            },
        )
        .await;
        let result = result.and_then(|out| {
            store.put(&out.stdout)?;
            Ok((out.stdout, out.stderr_bytes))
        });
        let receipt = match &result {
            Ok((b, err)) => {
                json!({"file":file("chapter-import-helper-record.json",b,"application/json"),"stderr_bytes":err})
            }
            Err(e) => json!({"code":e.code}),
        };
        store.transaction(|tx| {
            tx.execute(
                "UPDATE attempts SET state=?2,response=?3 WHERE id=?1",
                params![
                    id,
                    if result.is_ok() {
                        "completed"
                    } else {
                        "rejected"
                    },
                    serde_json::to_string(&receipt)?
                ],
            )?;
            Store::emit(
                tx,
                &run,
                if result.is_ok() {
                    "chapter_helper_completed"
                } else {
                    "chapter_helper_failed"
                },
                &json!({"attempt_id":id,"receipt":receipt}),
            )?;
            Ok(())
        })?;
        result.map(|(bytes, _)| bytes)
    });
    worker
        .await
        .map_err(|_| Error::new("chapter_helper", "bounded chapter helper worker failed"))?
}
