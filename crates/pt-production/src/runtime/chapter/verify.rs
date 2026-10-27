//! Independently verifies original source/turns/raw bytes and streamed whole PCM coverage.
use super::*;
use sha2::{Digest as _, Sha256};
use std::io::{Read, Seek, SeekFrom};
const CHAPTER: u64 = 256 * META;
pub(super) enum Tree<'a> {
    Local(&'a Path),
    Roots(&'a Path, &'a Path),
    Cas(&'a Store),
}
fn need(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        fail("chapter_bundle", message)
    }
}
fn number(v: &Value) -> Result<u64> {
    v.as_u64()
        .filter(|n| *n <= c::MAX_SAFE_INTEGER)
        .ok_or_else(|| Error::new("chapter_bundle", "safe nonnegative integer required"))
}
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| Error::new("chapter_bundle", "array required"))
}
fn string(v: &Value) -> Result<&str> {
    v.as_str()
        .ok_or_else(|| Error::new("chapter_bundle", "string required"))
}
fn span(v: &Value) -> Result<[u64; 2]> {
    let a = array(v)?;
    need(a.len() == 2, "interval length")?;
    Ok([number(&a[0])?, number(&a[1])?])
}
fn reference(v: &Value, max: u64) -> Result<c::FileRef> {
    let f: c::FileRef = serde_json::from_value(v.clone())?;
    f.validate()
        .map_err(|e| Error::new("chapter_bundle", e.to_string()))?;
    need(f.bytes <= max, "declared input envelope exceeded")?;
    Ok(f)
}
fn path(root: &Path, name: &str) -> Result<PathBuf> {
    need(c::safe_path(name), "unsafe private path")?;
    let root = root.canonicalize()?;
    let mut p = root.clone();
    for s in name.split('/') {
        p.push(s);
        need(!p.is_symlink(), "private symlink rejected")?;
    }
    need(
        p.canonicalize()?.starts_with(&root),
        "private path escaped root",
    )?;
    Ok(p)
}
pub(super) fn local(root: &Path, name: &str, max: u64) -> Result<Vec<u8>> {
    let p = path(root, name)?;
    need(
        p.metadata()?.is_file() && p.metadata()?.len() <= max,
        "bounded regular input required",
    )?;
    let mut b = Vec::new();
    std::fs::File::open(p)?.take(max + 1).read_to_end(&mut b)?;
    need(b.len() as u64 <= max, "input grew")?;
    Ok(b)
}
impl Tree<'_> {
    fn open(&self, pin: &Pin) -> Result<std::fs::File> {
        pin.file
            .validate()
            .map_err(|e| Error::new("chapter_bundle", e.to_string()))?;
        let p = match self {
            Self::Local(root) => path(&root.join(&pin.scope), &pin.file.path)?,
            Self::Roots(a, b) => path(if pin.scope == "original" { a } else { b }, &pin.file.path)?,
            Self::Cas(store) => store.object(&pin.file.sha256)?,
        };
        need(
            std::fs::symlink_metadata(&p)
                .map_err(|_| Error::new("corrupt_artifact", "chapter object missing"))?
                .is_file(),
            "chapter input must be regular",
        )?;
        let f = std::fs::File::open(p)?;
        need(
            f.metadata()?.len() == pin.file.bytes,
            "chapter input size changed",
        )?;
        Ok(f)
    }
    fn read(&self, scope: &str, f: &c::FileRef, max: u64) -> Result<Vec<u8>> {
        need(f.bytes <= max, "read envelope exceeded")?;
        let mut b = Vec::with_capacity(f.bytes as usize);
        self.open(&Pin {
            scope: scope.into(),
            file: f.clone(),
        })?
        .take(f.bytes + 1)
        .read_to_end(&mut b)?;
        f.verify(&b)
            .map_err(|e| Error::new("corrupt_artifact", e.to_string()))?;
        Ok(b)
    }
}
pub(super) fn cas_bytes(store: &Store, f: &c::FileRef, max: u64) -> Result<Vec<u8>> {
    Tree::Cas(store).read("bundle", f, max)
}
fn check(tree: &Tree<'_>, pin: &Pin, lease: Option<(&Store, &str, i64)>) -> Result<()> {
    let mut f = tree.open(pin)?;
    let mut h = Sha256::new();
    let mut size = 0;
    let mut block = [0u8; 65536];
    loop {
        let n = f.read(&mut block)?;
        if n == 0 {
            break;
        }
        if let Some((s, r, e)) = lease {
            active(s, r, e)?;
        }
        size += n as u64;
        need(size <= pin.file.bytes, "chapter input grew")?;
        h.update(&block[..n]);
    }
    need(
        size == pin.file.bytes && format!("{:x}", h.finalize()) == pin.file.sha256,
        "chapter input digest changed",
    )
}
fn decode(s: &str, max: usize) -> Result<Vec<u8>> {
    need(
        s.len() % 4 == 0 && s.len() / 4 * 3 <= max + 2,
        "base64 envelope",
    )?;
    let mut out = Vec::new();
    for (i, c) in s.as_bytes().chunks_exact(4).enumerate() {
        let digit = |b| -> Result<u32> {
            Ok(match b {
                b'A'..=b'Z' => (b - b'A') as u32,
                b'a'..=b'z' => (b - b'a' + 26) as u32,
                b'0'..=b'9' => (b - b'0' + 52) as u32,
                b'+' => 62,
                b'/' => 63,
                _ => return fail("chapter_bundle", "invalid base64"),
            })
        };
        let a = digit(c[0])?;
        let b = digit(c[1])?;
        let x = if c[2] == b'=' { 0 } else { digit(c[2])? };
        let y = if c[3] == b'=' { 0 } else { digit(c[3])? };
        need(
            (c[2] != b'=' || c[3] == b'=') && (c[3] != b'=' || i + 1 == s.len() / 4),
            "base64 padding",
        )?;
        need(c[2] != b'=' || b & 15 == 0, "base64 padding bits")?;
        need(c[3] != b'=' || x & 3 == 0, "base64 padding bits")?;
        let n = (a << 18) | (b << 12) | (x << 6) | y;
        out.push((n >> 16) as u8);
        if c[2] != b'=' {
            out.push((n >> 8) as u8);
        }
        if c[3] != b'=' {
            out.push(n as u8);
        }
    }
    need(out.len() <= max, "decoded metadata envelope")?;
    Ok(out)
}
fn raw_refs(spec: &Value) -> Result<BTreeMap<String, c::FileRef>> {
    let mut refs = BTreeMap::new();
    let mut total = 0u64;
    for t in array(&spec["turns"])? {
        for role in ["primary", "independent"] {
            if !t[role].is_null() {
                for field in ["request", "response", "receipt", "recipe"] {
                    if t[role][field].is_null() {
                        continue;
                    }
                    let f = reference(&t[role][field], META)?;
                    if let Some(old) = refs.get(&f.path) {
                        need(*old == f, "conflicting raw identity")?;
                    } else {
                        total = total
                            .checked_add(f.bytes)
                            .ok_or_else(|| Error::new("chapter_bounds", "raw size overflow"))?;
                        need(total <= META / 2, "original raw metadata exceeds 512 KiB")?;
                        refs.insert(f.path.clone(), f);
                    }
                }
            }
        }
    }
    Ok(refs)
}
fn words(text: &str) -> u64 {
    let mut count = 0;
    let mut inside = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() {
            if !inside {
                count += 1;
            }
            inside = true;
        } else if !(inside
            && matches!(ch, '\'' | '’')
            && chars.peek().is_some_and(|c| c.is_alphanumeric()))
        {
            inside = false;
        }
    }
    count
}
fn logical(
    tree: &Tree<'_>,
    spec_ref: &c::FileRef,
    index_ref: &c::FileRef,
) -> Result<(Value, Value)> {
    let raw = tree.read("original", spec_ref, META)?;
    let spec: Value = serde_json::from_slice(&raw)?;
    let index: Value = serde_json::from_slice(&tree.read("bundle", index_ref, META)?)?;
    need(
        spec["schema_version"] == 2
            && index["schema_version"] == 2
            && index["recipe_sha256"] == RECIPE,
        "chapter schema/recipe",
    )?;
    need(
        matches!(
            spec["provenance"].as_str(),
            Some("synthetic" | "imported" | "diagnostic")
        ) && c::valid_id(string(&spec["source_scope_id"])?),
        "chapter provenance/scope",
    )?;
    let turns = array(&spec["turns"])?;
    let facts = array(&spec["facts"])?;
    need(
        !turns.is_empty() && turns.len() <= 1024 && facts.len() <= 1024,
        "turn/fact count",
    )?;
    let source = reference(&spec["source"], FILE)?;
    let narration = reference(&spec["narration"], CHAPTER)?;
    need(
        source.media_type == "text/plain"
            && narration.media_type == "audio/wav"
            && narration.bytes > 44
            && (narration.bytes - 44) % 2 == 0,
        "source/native format",
    )?;
    let rate = number(&spec["sample_rate"])?;
    need((8000..=192000).contains(&rate), "native sample rate")?;
    let mut bytes = 0u64;
    for t in turns {
        bytes = bytes
            .checked_add(string(&t["text"])?.len() as u64)
            .ok_or_else(|| Error::new("chapter_bounds", "source size overflow"))?;
        reference(&t["audio"], FILE)?;
    }
    need(bytes == source.bytes, "complete verbatim source size")?;
    let refs = raw_refs(&spec)?;
    let source_bytes = tree.read("original", &source, FILE)?;
    let text = std::str::from_utf8(&source_bytes)
        .map_err(|_| Error::new("chapter_bundle", "source is not UTF-8"))?;
    let mut spans = 0u64;
    for f in facts {
        spans = spans
            .checked_add(array(&f["source_spans"])?.len() as u64)
            .ok_or_else(|| Error::new("chapter_bounds", "span count overflow"))?;
    }
    need(
        words(text)
            .checked_mul(spans.max(1))
            .is_some_and(|n| n <= 2_000_000),
        "global ambiguity work exceeds 2,000,000 token spans",
    )?;
    let origin = number(&spec["source_origin"])?;
    let end = origin
        .checked_add(text.chars().count() as u64)
        .ok_or_else(|| Error::new("chapter_bounds", "source origin overflow"))?;
    need(end <= c::MAX_SAFE_INTEGER, "source domain")?;
    let mut scalar = origin;
    let mut cursor = 0;
    let mut frame = 0;
    let mut ids = std::collections::BTreeSet::new();
    let mut source_units = Vec::new();
    for t in turns {
        let id = string(&t["id"])?;
        need(
            c::valid_id(id) && ids.insert(id) && !string(&t["speaker"])?.trim().is_empty(),
            "turn identity",
        )?;
        let s = span(&t["source_span"])?;
        let f = span(&t["frames"])?;
        let body = string(&t["text"])?;
        need(
            s[0] == scalar
                && s[1] == scalar + body.chars().count() as u64
                && s[0] < s[1]
                && text[cursor..].starts_with(body),
            "ordered verbatim Unicode coverage",
        )?;
        need(
            f[0] == frame
                && f[0] < f[1]
                && reference(&t["audio"], FILE)?.bytes == 44 + (f[1] - f[0]) * 2,
            "ordered turn PCM partition",
        )?;
        scalar = s[1];
        cursor += body.len();
        frame = f[1];
        source_units.push(json!({"id":t["id"],"speaker":t["speaker"],"source_span":t["source_span"],"frames":t["frames"],"audio":t["audio"]}));
    }
    need(
        scalar == end && cursor == text.len() && frame == (narration.bytes - 44) / 2,
        "complete source/PCM tail",
    )?;
    let required = array(&spec["required_fact_ids"])?;
    let mut required_ids = std::collections::BTreeSet::new();
    for x in required {
        need(
            c::valid_id(string(x)?) && required_ids.insert(string(x)?),
            "required fact identity",
        )?;
    }
    let mut fact_ids = std::collections::BTreeSet::new();
    for f in facts {
        let id = string(&f["id"])?;
        need(c::valid_id(id) && fact_ids.insert(id), "fact identity")?;
        for s in array(&f["source_spans"])? {
            let s = span(s)?;
            need(
                origin <= s[0] && s[0] < s[1] && s[1] <= end,
                "fact source scope",
            )?;
        }
    }
    need(required_ids == fact_ids, "fact inventory coverage")?;
    if !spec["inherited_findings"].is_null() {
        need(
            array(&spec["inherited_findings"])?.len() <= 1024,
            "inherited finding count",
        )?;
    }
    let context_ref = reference(&index["context"], META)?;
    let context: Value = serde_json::from_slice(&tree.read("bundle", &context_ref, META)?)?;
    need(
        context["schema_version"] == 2
            && context["recipe_sha256"] == RECIPE
            && context["spec"] == spec
            && reference(&context["original_inventory"], META)? == *spec_ref,
        "original context binding",
    )?;
    need(
        decode(string(&context["inventory_bytes"])?, META as usize)? == raw,
        "original inventory bytes changed",
    )?;
    let context_raw = context["raw"]
        .as_object()
        .ok_or_else(|| Error::new("chapter_bundle", "complete raw context required"))?;
    need(
        context_raw.len() == refs.len(),
        "raw context inventory differs",
    )?;
    for (name, f) in refs {
        need(
            decode(string(&context_raw[&name])?, (META / 2) as usize)?
                == tree.read("original", &f, META)?,
            "original raw observation bytes differ",
        )?;
    }
    let mut bundle_source = source.clone();
    bundle_source.path = reference(&index["source"], FILE)?.path;
    need(
        reference(&index["source"], FILE)? == bundle_source
            && tree.read("bundle", &bundle_source, FILE)? == source_bytes,
        "complete bundle source differs",
    )?;
    need(
        index["narration"] == spec["narration"] && index["source_units"] == json!(source_units),
        "original narration/turn index differs",
    )?;
    let chunks = array(&index["chunks"])?;
    need(!chunks.is_empty() && chunks.len() <= 1024, "chunk count")?;
    let mut next = 0;
    let mut chunk_ids = std::collections::BTreeSet::new();
    let mut paths = std::collections::BTreeSet::new();
    for chunk in chunks {
        let s = span(&chunk["frames"])?;
        let f = reference(&chunk["file"], FILE)?;
        let id = string(&chunk["id"])?;
        need(
            c::valid_id(id)
                && chunk_ids.insert(id)
                && paths.insert(f.path)
                && s[0] == next
                && s[0] < s[1]
                && 44 + (s[1] - s[0]) * 2 == f.bytes
                && f.media_type == "audio/wav",
            "ordered chunk partition",
        )?;
        next = s[1];
    }
    need(next == frame, "complete chunk tail")?;
    Ok((spec, index))
}
fn unit(tree: &Tree<'_>, u: &Unit, index: &Value, index_ref: &c::FileRef) -> Result<()> {
    let value: Value = serde_json::from_slice(&tree.read("bundle", &u.metadata, META)?)?;
    need(
        value["schema_version"] == 2
            && value["recipe_sha256"] == RECIPE
            && reference(&value["index"], META)? == *index_ref
            && value["base_chunk"] == u.label,
        "unit identity",
    )?;
    let included = array(&value["included_chunks"])?;
    let mut ids = std::collections::BTreeSet::new();
    let mut expected = vec![
        reference(&index["source"], FILE)?,
        reference(&index["context"], META)?,
        index_ref.clone(),
    ];
    for id in included {
        let id = string(id)?;
        need(ids.insert(id), "duplicate unit dependency")?;
        let chunk = array(&index["chunks"])?
            .iter()
            .find(|c| c["id"] == id)
            .ok_or_else(|| Error::new("chapter_bundle", "unknown chunk dependency"))?;
        expected.push(reference(&chunk["file"], FILE)?);
    }
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    need(
        ids.contains(u.label.as_str())
            && expected == u.inputs
            && value["files"] == serde_json::to_value(&expected)?
            && expected.len() + 5 <= 64
            && expected.iter().map(|f| f.bytes).sum::<u64>() + 5 * META <= TOTAL,
        "exact unit allowlist/replay slots/envelope",
    )
}
fn header(rate: u64, frames: u64) -> Vec<u8> {
    let size = (frames * 2) as u32;
    let mut b = Vec::new();
    b.extend(b"RIFF");
    b.extend((size + 36).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend((rate as u32).to_le_bytes());
    b.extend((rate as u32 * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(size.to_le_bytes());
    b
}
fn pcm(
    tree: &Tree<'_>,
    native: &c::FileRef,
    other: &c::FileRef,
    scope: &str,
    start: u64,
    end: u64,
    rate: u64,
    lease: Option<(&Store, &str, i64)>,
) -> Result<String> {
    let mut chapter = tree.open(&Pin {
        scope: "original".into(),
        file: native.clone(),
    })?;
    let mut part = tree.open(&Pin {
        scope: scope.into(),
        file: other.clone(),
    })?;
    let mut h = [0u8; 44];
    chapter.read_exact(&mut h)?;
    need(
        h.as_slice() == header(rate, (native.bytes - 44) / 2),
        "canonical whole native WAV required",
    )?;
    part.read_exact(&mut h)?;
    need(
        h.as_slice() == header(rate, end - start),
        "canonical bounded native WAV required",
    )?;
    chapter.seek(SeekFrom::Start(44 + start * 2))?;
    let mut left = (end - start) * 2;
    let mut a = [0u8; 65536];
    let mut b = [0u8; 65536];
    let mut hash = Sha256::new();
    while left > 0 {
        if let Some((s, r, e)) = lease {
            active(s, r, e)?;
        }
        let n = left.min(a.len() as u64) as usize;
        chapter.read_exact(&mut a[..n])?;
        part.read_exact(&mut b[..n])?;
        need(
            a[..n] == b[..n],
            "whole/turn/chunk PCM reconstruction differs",
        )?;
        hash.update(&b[..n]);
        left -= n as u64;
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub(super) fn whole(
    tree: &Tree<'_>,
    plan: &Plan,
    lease: Option<(&Store, &str, i64)>,
) -> Result<()> {
    let index = plan
        .pins
        .iter()
        .find(|p| p.scope == "bundle" && p.file.path == "chapter-import-index.json")
        .ok_or_else(|| Error::new("chapter_bundle", "index missing"))?;
    let (spec, value) = logical(tree, &plan.spec, &index.file)?;
    need(
        plan.units.len() == array(&value["chunks"])?.len(),
        "all bounded units required",
    )?;
    for (u, c) in plan.units.iter().zip(array(&value["chunks"])?) {
        need(c["id"] == u.label && u.revision == 1, "unit order/revision")?;
        unit(tree, u, &value, &index.file)?;
    }
    for pin in &plan.pins {
        check(tree, pin, lease)?;
    }
    let narration = reference(&spec["narration"], CHAPTER)?;
    let rate = number(&spec["sample_rate"])?;
    for t in array(&spec["turns"])? {
        let s = span(&t["frames"])?;
        pcm(
            tree,
            &narration,
            &reference(&t["audio"], FILE)?,
            "original",
            s[0],
            s[1],
            rate,
            lease,
        )?;
    }
    for c in array(&value["chunks"])? {
        let s = span(&c["frames"])?;
        need(
            pcm(
                tree,
                &narration,
                &reference(&c["file"], FILE)?,
                "bundle",
                s[0],
                s[1],
                rate,
                lease,
            )? == string(&c["pcm_sha256"])?,
            "chunk PCM digest differs",
        )?;
    }
    Ok(())
}
pub(super) fn snapshot(
    store: &Store,
    run: &str,
    epoch: i64,
    original: &Path,
    evidence: &str,
    bundle: &Path,
    target: &Path,
    request: &c::RunRequest,
    revision: &str,
) -> Result<Plan> {
    let spec_bytes = local(original, evidence, META)?;
    let spec_ref = file(evidence, &spec_bytes, "application/json");
    let index_bytes = local(bundle, "chapter-import-index.json", META)?;
    let index_ref = file(
        "chapter-import-index.json",
        &index_bytes,
        "application/json",
    );
    let tree = Tree::Roots(original, bundle);
    let (spec, index) = logical(&tree, &spec_ref, &index_ref)?;
    let mut pins = BTreeMap::new();
    let mut add = |scope: &str, f: c::FileRef| -> Result<()> {
        let key = (scope.to_owned(), f.path.clone());
        if let Some(old) = pins.get(&key) {
            need(*old == f, "conflicting private input path")?;
        }
        pins.insert(key, f);
        Ok(())
    };
    add("original", spec_ref.clone())?;
    add("original", reference(&spec["source"], FILE)?)?;
    add("original", reference(&spec["narration"], CHAPTER)?)?;
    for t in array(&spec["turns"])? {
        add("original", reference(&t["audio"], FILE)?)?;
    }
    for (_, f) in raw_refs(&spec)? {
        add("original", f)?;
    }
    add("bundle", index_ref.clone())?;
    add("bundle", reference(&index["source"], FILE)?)?;
    add("bundle", reference(&index["context"], META)?)?;
    let mut units = Vec::new();
    for chunk in array(&index["chunks"])? {
        add("bundle", reference(&chunk["file"], FILE)?)?;
        let label = string(&chunk["id"])?;
        let name = format!("units/{label}.json");
        let bytes = local(bundle, &name, META)?;
        let meta = file(&name, &bytes, "application/json");
        let v: Value = serde_json::from_slice(&bytes)?;
        let inputs = array(&v["files"])?
            .iter()
            .map(|v| reference(v, FILE))
            .collect::<Result<Vec<_>>>()?;
        for f in &inputs {
            if f.media_type == "application/json"
                || matches!(
                    f.path.as_str(),
                    "private/chapter-context.json" | "chapter-import-index.json"
                )
            {
                need(f.bytes <= META, "metadata alias exceeds envelope")?;
            }
        }
        need(
            inputs.len() + 5 <= 64
                && inputs.iter().map(|f| f.bytes).sum::<u64>() + 5 * META <= TOTAL,
            "reserve five replay metadata slots before reads",
        )?;
        let name = format!("records/{label}.json");
        let bytes = local(bundle, &name, META)?;
        let record = file(&name, &bytes, "application/json");
        add("bundle", meta.clone())?;
        add("bundle", record.clone())?;
        units.push(Unit {
            label: label.into(),
            metadata: meta,
            record,
            inputs,
            revision: 1,
        });
    }
    drop(add);
    need(
        pins.len() <= 16384
            && pins.values().map(|f| f.bytes).sum::<u64>() <= 2 * 1024 * 1024 * 1024,
        "whole snapshot envelope exceeded",
    )?;
    let plan = Plan {
        schema_version: 1,
        recipe: RECIPE.into(),
        stage_revision: revision.into(),
        runtime_revision: runtime_revision(),
        request_sha256: digest(&serde_json::to_vec(&serde_json::to_value(request)?)?),
        spec: spec_ref,
        source_origin: number(&spec["source_origin"])?,
        pins: pins
            .into_iter()
            .map(|((scope, _), file)| Pin { scope, file })
            .collect(),
        units,
    };
    whole(&tree, &plan, Some((store, run, epoch)))?;
    for pin in &plan.pins {
        active(store, run, epoch)?;
        let dest = target.join(&pin.scope).join(&pin.file.path);
        std::fs::create_dir_all(dest.parent().unwrap())?;
        let mut out = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dest)?;
        std::io::copy(&mut tree.open(pin)?.take(pin.file.bytes + 1), &mut out)?;
        out.sync_all()?;
        check(&Tree::Local(target), pin, Some((store, run, epoch)))?;
    }
    Ok(plan)
}
pub(super) fn freeze(store: &Store, tree: &Tree<'_>, pin: &Pin) -> Result<()> {
    let path = store.object(&pin.file.sha256)?;
    if !path.exists() {
        let mut tmp = tempfile::NamedTempFile::new_in(store.root.join("objects"))?;
        std::io::copy(&mut tree.open(pin)?.take(pin.file.bytes + 1), &mut tmp)?;
        tmp.as_file().sync_all()?;
        if let Err(e) = tmp.persist_noclobber(path) {
            if e.error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(e.error.into());
            }
        }
        std::fs::File::open(store.root.join("objects"))?.sync_all()?;
    }
    check(&Tree::Cas(store), pin, None)
}
