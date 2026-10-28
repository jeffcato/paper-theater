use super::*;
#[derive(Clone, Serialize, Deserialize)]
pub struct SpeechBatch {
    pub id: String,
    pub audio: c::ArtifactRef,
    pub receipt_file: c::ArtifactRef,
    pub receipt: Value,
    pub committed_epoch: i64,
}
struct Context {
    manifest: c::ArtifactRef,
    plan: c::ArtifactRef,
    takes: Value,
    proposal: Value,
    limits: pcm::Limits,
    binding: String,
}
fn context(store: &Store, run: &str) -> Result<Context> {
    let frozen = load(store, run)?;
    let manifest = get_output(store, run, 2, "private/speech-takes.json")?;
    let plan = get_output(store, run, 2, "private/speech-transcription-plan.json")?;
    let takes = serde_json::from_slice(&storage::verify_owned(store, run, &manifest)?)?;
    let proposal = serde_json::from_slice(&storage::verify_owned(store, run, &plan)?)?;
    let ctx = original(&frozen, "private/speech-context.json")?;
    let ctx: Value = serde_json::from_slice(&storage::verify_owned(store, run, ctx)?)?;
    let limits: pcm::Limits = serde_json::from_value(ctx["limits"].clone())?;
    let binding = digest(&serde_json::to_vec(&(
        run,
        &manifest,
        &plan,
        &limits,
        pcm::recipe(),
        frozen.registration_sha256,
    ))?);
    Ok(Context {
        manifest,
        plan,
        takes,
        proposal,
        limits,
        binding,
    })
}
fn audio_bytes(store: &Store, run: &str, a: &c::ArtifactRef) -> Result<Vec<u8>> {
    let encoded: String = store.db()?.query_row(
        "SELECT reference FROM speech_refs WHERE run=?1 AND id=?2",
        params![run, a.id],
        |r| r.get(0),
    )?;
    if encoded != serde_json::to_string(a)?
        || a.file.bytes > 16 * META
        || a.file.media_type != "audio/wav"
    {
        return fail("speech_pcm_corrupt", "batch metadata changed");
    }
    let b = store.read_object(&a.file.sha256, a.file.bytes as usize)?;
    a.file.verify(&b).map_err(contract)?;
    c::inspect_pcm16_wav(&mut std::io::Cursor::new(&b), b.len() as u64).map_err(contract)?;
    Ok(b)
}
fn saved(store: &Store, run: &str, id: &str, binding: &str) -> Result<Option<SpeechBatch>> {
    let row: Option<(String, String, String)> = store
        .db()?
        .query_row(
            "SELECT binding,record,sha256 FROM speech_batches WHERE run=?1 AND id=?2",
            params![run, id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    row.map(|(original, b, h)| {
        if original != binding || h != digest(&serde_json::to_vec(&(run, id, binding, &b))?) {
            return fail("speech_pcm_corrupt", "batch checkpoint/recipe changed");
        }
        let record: SpeechBatch = serde_json::from_str(&b)?;
        if record.id != id {
            return fail("speech_pcm_corrupt", "batch identity changed");
        }
        let body = storage::verify_owned(store, run, &record.receipt_file)?;
        if body != serde_json::to_vec_pretty(&record.receipt)? {
            return fail("speech_pcm_corrupt", "copy receipt changed");
        }
        audio_bytes(store, run, &record.audio)?;
        Ok(record)
    })
    .transpose()
}
fn active(store: &Store, run: &str, epoch: Option<i64>) -> Result<()> {
    if let Some(e) = epoch {
        Store::active(&store.db()?, run, e)?;
    }
    Ok(())
}
async fn validated(
    store: &Store,
    run: &str,
    epoch: Option<i64>,
    ctx: &Context,
) -> Result<Vec<pcm::Batch>> {
    let s = store.clone();
    let r = run.to_owned();
    let plan = ctx.proposal.clone();
    let takes = ctx.takes.clone();
    let limits = ctx.limits.clone();
    tokio::task::spawn_blocking(move || {
        pcm::validate(
            &plan,
            &takes,
            &limits,
            |a| storage::verify_owned(&s, &r, a),
            || active(&s, &r, epoch),
        )
    })
    .await
    .map_err(|_| Error::new("speech_pcm_worker", "validation worker failed"))?
}
async fn copied(
    store: &Store,
    run: &str,
    epoch: Option<i64>,
    batch: pcm::Batch,
    limits: pcm::Limits,
) -> Result<Vec<u8>> {
    let s = store.clone();
    let r = run.to_owned();
    let work = tokio::task::spawn_blocking(move || {
        pcm::copy(
            &batch,
            &limits,
            |a| storage::verify_owned(&s, &r, a),
            || active(&s, &r, epoch),
        )
    });
    let ms = request(store, run)?.limits.wall_time_ms.min(30000);
    tokio::time::timeout(std::time::Duration::from_millis(ms), work)
        .await
        .map_err(|_| Error::new("speech_pcm_timeout", "copy deadline"))?
        .map_err(|_| Error::new("speech_pcm_worker", "copy worker failed"))?
}
fn receipt(ctx: &Context, b: &pcm::Batch, a: &c::ArtifactRef) -> Value {
    json!({"schema_version":1,"provenance":"diagnostic","recipe":"native-pcm16-frame-copy-v1","recipe_sha256":pcm::recipe(),"take_manifest":ctx.manifest,"copy_plan":ctx.plan,"source_sha256":ctx.takes["source_sha256"],"batch_id":b.id,"format":{"sample_rate":b.sample_rate,"channels":b.channels,"bits_per_sample":16,"frames":b.frames},"segments":b.segments,"unplanned_audio":ctx.proposal["unplanned_audio"],"audio":a,"sample_origin":"materialized-batch-local","narration_assembly":null,"measured_word_intervals":null,"fact_bounds":null,"model_payload":{"schema_version":1,"mode":"VERBATIM","word_timestamps":true,"audio":a},"dispatch_permitted":false,"bulk_speech_approved":false,"production_approved":false})
}
pub(super) async fn materialize(store: &Store, run: &str, epoch: i64) -> Result<()> {
    scope(store, run, epoch)?;
    let ctx = context(store, run)?;
    let batches = validated(store, run, Some(epoch), &ctx).await?;
    for b in batches {
        active(store, run, Some(epoch))?;
        if saved(store, run, &b.id, &ctx.binding)?.is_some() {
            continue;
        }
        let bytes = copied(store, run, Some(epoch), b.clone(), ctx.limits.clone()).await?;
        store.put_typed(&bytes, "audio/wav")?;
        let audio = artifact(
            run,
            "pcm",
            file(
                &format!("private/speech-batches/{}.wav", b.id),
                &bytes,
                "audio/wav",
            ),
        );
        let receipt = receipt(&ctx, &b, &audio);
        let body = serde_json::to_vec_pretty(&receipt)?;
        if body.len() as u64 > META {
            return fail("speech_bounds", "copy receipt cap");
        }
        store.put(&body)?;
        let rf = artifact(
            run,
            "pcm",
            file(
                &format!("private/speech-batches/{}.json", b.id),
                &body,
                "application/json",
            ),
        );
        let record = SpeechBatch {
            id: b.id.clone(),
            audio,
            receipt_file: rf,
            receipt,
            committed_epoch: epoch,
        };
        let body = serde_json::to_string(&record)?;
        let h = digest(&serde_json::to_vec(&(run, &b.id, &ctx.binding, &body))?);
        store.transaction(|tx|{Store::active(tx,run,epoch)?;if context(store,run)?.binding!=ctx.binding{return fail("speech_pcm_stale","consumed scope changed before copy commit");}for a in [&record.audio,&record.receipt_file]{a.file.verify(&store.read_object(&a.file.sha256,a.file.bytes as usize)?).map_err(contract)?;storage::insert(tx,run,a,"verified materialized native PCM copy; no narration/acceptance authority")?;}tx.execute("INSERT INTO speech_batches VALUES(?1,?2,?3,?4,?5)",params![run,b.id,ctx.binding,body,h])?;Store::emit(tx,run,"speech_pcm_checkpoint",&json!({"batch_id":b.id,"audio_sha256":record.audio.file.sha256,"frames":b.frames,"recipe_sha256":pcm::recipe(),"bulk_speech_approved":false}))?;Store::active(tx,run,epoch)})?;
    }
    Ok(())
}
pub(super) async fn replay(store: &Store, run: &str) -> Result<Vec<(c::FileRef, Vec<u8>)>> {
    let ctx = context(store, run)?;
    let batches = validated(store, run, None, &ctx).await?;
    let mut outputs = Vec::new();
    for b in batches {
        let old = saved(store, run, &b.id, &ctx.binding)?
            .ok_or_else(|| Error::new("speech_pcm_replay", "uncommitted batch"))?;
        let bytes = copied(store, run, None, b.clone(), ctx.limits.clone()).await?;
        if bytes != audio_bytes(store, run, &old.audio)?
            || receipt(&ctx, &b, &old.audio) != old.receipt
        {
            return fail("speech_pcm_replay", "materialized PCM/receipt changed");
        }
        outputs.push((old.audio.file, bytes));
        outputs.push((
            old.receipt_file.file,
            serde_json::to_vec_pretty(&old.receipt)?,
        ));
    }
    Ok(outputs)
}
impl Store {
    pub fn speech_batches(&self, run: &str) -> Result<Vec<SpeechBatch>> {
        let ctx = context(self, run)?;
        let ids = self
            .db()?
            .prepare("SELECT id FROM speech_batches WHERE run=?1 ORDER BY id")?
            .query_map([run], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ids.iter()
            .map(|id| {
                saved(self, run, id, &ctx.binding)?
                    .ok_or_else(|| Error::new("not_found", "batch missing"))
            })
            .collect()
    }
    pub fn speech_batch_audio(&self, run: &str, id: &str) -> Result<Vec<u8>> {
        let ctx = context(self, run)?;
        let record = saved(self, run, id, &ctx.binding)?
            .ok_or_else(|| Error::new("not_found", "batch missing"))?;
        audio_bytes(self, run, &record.audio)
    }
}
