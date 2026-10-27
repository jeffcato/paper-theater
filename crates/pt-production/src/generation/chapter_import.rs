//! Independent replay of bounded v2 imported chapter units. No helper dispatch or authority.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeSet;
type Files = BTreeMap<String, (FileRef, Vec<u8>)>;
const META: u64 = 1024 * 1024;
const FILE: u64 = 16 * META;
const TOTAL: u64 = 64 * META;
const OUTPUT: &str = "private/chapter-import-validation.json";
fn need(ok: bool, message: &str) -> Result<(), StageError> {
    require(ok, "chapter_import", "evidence", message)?;
    Ok(())
}
fn recipe() -> Digest {
    digest(
        &[
            include_bytes!("../../../../tools/generation/chapter_import_helper.py").as_slice(),
            include_bytes!("../../../../tools/generation/chapter_import.py").as_slice(),
            include_bytes!("../../../../tools/generation/chapter_acoustic.py").as_slice(),
            include_bytes!("../../../../tools/generation/adapters.py").as_slice(),
        ]
        .concat(),
    )
}
fn text(v: &Value) -> Result<&str, StageError> {
    v.as_str()
        .ok_or_else(|| StageError::Evidence("import string required".into()))
}
fn list(v: &Value) -> Result<&Vec<Value>, StageError> {
    v.as_array()
        .ok_or_else(|| StageError::Evidence("import array required".into()))
}
fn num(v: &Value) -> Result<u64, StageError> {
    let n = v
        .as_u64()
        .ok_or_else(|| StageError::Evidence("import integer required".into()))?;
    need(n <= MAX_SAFE_INTEGER, "unsafe integer")?;
    Ok(n)
}
fn span(v: &Value) -> Result<[u64; 2], StageError> {
    need(list(v)?.len() == 2, "interval length")?;
    Ok([num(&v[0])?, num(&v[1])?])
}
fn reference(v: &Value) -> Result<FileRef, StageError> {
    let f: FileRef =
        serde_json::from_value(v.clone()).map_err(|e| StageError::Evidence(e.to_string()))?;
    f.validate()?;
    Ok(f)
}
fn get<'a>(files: &'a Files, path: &str) -> Result<&'a (FileRef, Vec<u8>), StageError> {
    files
        .get(path)
        .ok_or_else(|| StageError::Evidence(format!("missing chapter import input: {path}")))
}
fn read<'a>(files: &'a Files, v: &Value) -> Result<&'a [u8], StageError> {
    let f = reference(v)?;
    let actual = get(files, &f.path)?;
    need(f == actual.0, "import reference changed")?;
    Ok(&actual.1)
}
fn id(v: &Value) -> Result<&str, StageError> {
    let s = text(v)?;
    need(valid_id(s) && s.len() <= 128, "import id")?;
    Ok(s)
}
fn base64(s: &str) -> Result<Vec<u8>, StageError> {
    need(
        s.len() % 4 == 0 && s.len() <= META as usize * 2,
        "base64 bound",
    )?;
    let mut out = Vec::new();
    for (i, c) in s.as_bytes().chunks_exact(4).enumerate() {
        let digit = |b: u8| -> Result<u32, StageError> {
            Ok(match b {
                b'A'..=b'Z' => (b - b'A') as u32,
                b'a'..=b'z' => (b - b'a' + 26) as u32,
                b'0'..=b'9' => (b - b'0' + 52) as u32,
                b'+' => 62,
                b'/' => 63,
                _ => return Err(StageError::Evidence("invalid base64".into())),
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
        let n = (a << 18) | (b << 12) | (x << 6) | y;
        out.push((n >> 16) as u8);
        if c[2] != b'=' {
            out.push((n >> 8) as u8);
        }
        if c[3] != b'=' {
            out.push(n as u8);
        }
    }
    Ok(out)
}
fn lex(s: &str) -> Vec<(String, u64, u64)> {
    let c = s.chars().collect::<Vec<_>>();
    let mut i = 0;
    let mut out = vec![];
    while i < c.len() {
        if !c[i].is_alphanumeric() {
            i += 1;
            continue;
        }
        let a = i;
        let mut word = String::new();
        while i < c.len() {
            if c[i].is_alphanumeric() {
                word.extend(c[i].to_lowercase());
                i += 1;
            } else if matches!(c[i], '\'' | '’') && i + 1 < c.len() && c[i + 1].is_alphanumeric()
            {
                i += 1;
            } else {
                break;
            }
        }
        out.push((word, a as u64, i as u64));
    }
    out
}
fn tokens(s: &str) -> Vec<String> {
    lex(s).into_iter().map(|w| w.0).collect()
}
fn sample(s: &str, rate: u64, scale: u128) -> Result<u64, StageError> {
    let s = if scale == 1 {
        s.strip_suffix('s')
            .ok_or_else(|| StageError::Evidence("word timestamp suffix".into()))?
    } else {
        s
    };
    if scale == 1 {
        need(
            !s.contains(['e', 'E'])
                && s.split('.')
                    .nth(1)
                    .is_none_or(|v| !v.is_empty() && v.len() <= 9),
            "word offset precision",
        )?;
    }
    let mut exp = s.split(['e', 'E']);
    let mantissa = exp.next().unwrap_or("");
    let exponent = exp
        .next()
        .unwrap_or("0")
        .parse::<i32>()
        .map_err(|_| StageError::Evidence("timestamp exponent".into()))?;
    need(
        exp.next().is_none() && (-18..=18).contains(&exponent),
        "timestamp exponent",
    )?;
    let mut parts = mantissa.split('.');
    let whole = parts.next().unwrap_or("");
    let frac = parts.next().unwrap_or("");
    need(
        parts.next().is_none()
            && !whole.is_empty()
            && whole
                .bytes()
                .chain(frac.bytes())
                .all(|b| b.is_ascii_digit())
            && frac.len() <= 18,
        "decimal timestamp",
    )?;
    let n = format!("{whole}{frac}")
        .parse::<u128>()
        .map_err(|_| StageError::Evidence("timestamp numerator".into()))?;
    let power = frac.len() as i32 - exponent;
    let (n, d) = if power >= 0 {
        (n, scale.checked_mul(10u128.pow(power as u32)))
    } else {
        (
            n.checked_mul(10u128.pow((-power) as u32))
                .ok_or_else(|| StageError::Evidence("timestamp overflow".into()))?,
            Some(scale),
        )
    };
    let d = d.ok_or_else(|| StageError::Evidence("timestamp denominator".into()))?;
    let n = n
        .checked_mul(rate as u128)
        .ok_or_else(|| StageError::Evidence("timestamp overflow".into()))?;
    let r = n / d + u128::from(n % d != 0);
    need(r <= MAX_SAFE_INTEGER as u128, "sample domain")?;
    Ok(r as u64)
}
// Preserve decimal JSON number spelling before serde_json's binary-float representation.
fn exact_numbers(bytes: &[u8]) -> Result<Value, StageError> {
    let s = std::str::from_utf8(bytes).map_err(|_| StageError::Evidence("response UTF8".into()))?;
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        let a = i;
        if b[i] == b'"' {
            i += 1;
            while i < b.len() {
                if b[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push_str(&s[a..i.min(b.len())]);
        } else if b[i].is_ascii_digit() || b[i] == b'-' {
            i += 1;
            while i < b.len()
                && (b[i].is_ascii_digit() || matches!(b[i], b'.' | b'e' | b'E' | b'+' | b'-'))
            {
                i += 1;
            }
            out.push_str(&serde_json::to_string(&s[a..i]).unwrap());
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    parse(out.as_bytes())
}
#[derive(Default)]
struct Observation {
    words: Vec<Value>,
    tokens: Vec<String>,
    start: Option<u64>,
    end: Option<u64>,
    passed: bool,
}
fn observe(
    turn: &Value,
    role: &str,
    raw: &BTreeMap<String, Vec<u8>>,
    rate: u64,
) -> Result<Observation, StageError> {
    let obs = &turn[role];
    let mut out = Observation::default();
    if obs.is_null()
        || ["request", "response", "receipt", "recipe"]
            .iter()
            .any(|k| obs[*k].is_null())
    {
        return Ok(out);
    }
    id(&obs["provider"])?;
    id(&obs["model"])?;
    let body = |key: &str| -> Result<&[u8], StageError> {
        let p = text(&obs[key]["path"])?;
        raw.get(p)
            .map(|b| b.as_slice())
            .ok_or_else(|| StageError::Evidence("raw observation missing".into()))
    };
    let receipt: Value = parse(body("receipt")?)?;
    need(
        receipt["model"] == obs["model"]
            && (receipt["provider"].is_null() || receipt["provider"] == obs["provider"])
            && receipt["request_sha256"] == obs["request"]["sha256"]
            && receipt["response_sha256"] == obs["response"]["sha256"]
            && receipt["audio_sha256"] == turn["audio"]["sha256"],
        "receipt/request/model/audio binding",
    )?;
    if receipt["harness_sha256"].is_null() || receipt["provider"].is_null() {
        return Ok(out);
    }
    need(
        receipt["harness_sha256"] == obs["recipe"]["sha256"],
        "provider recipe binding",
    )?;
    let response: Value = parse(body("response")?)?;
    let [a, b] = span(&turn["frames"])?;
    let expected = lex(text(&turn["text"])?);
    if role == "primary" {
        let candidates = list(&response["candidates"])?;
        need(
            candidates.len() == 1 && candidates[0]["finishReason"] == "STOP",
            "incomplete primary response",
        )?;
        let mut measured = vec![];
        let mut found = false;
        for p in list(&candidates[0]["content"]["parts"])? {
            if !p["audioTranscription"].is_null() {
                measured.extend(list(&p["audioTranscription"]["words"])?.iter());
                found = true;
            }
        }
        if !found {
            return Ok(out);
        }
        let mut previous = [0, 0];
        for w in &measured {
            let x = sample(text(&w["startOffset"])?, rate, 1)?;
            let y = sample(text(&w["endOffset"])?, rate, 1)?;
            let heard = tokens(text(&w["word"])?);
            need(
                x <= y && y <= b - a && x >= previous[0] && y >= previous[1] && heard.len() == 1,
                "primary interval order/domain",
            )?;
            previous = [x, y];
            out.tokens.push(heard[0].clone());
        }
        if out.tokens != expected.iter().map(|w| w.0.clone()).collect::<Vec<_>>()
            || expected.is_empty()
        {
            return Ok(out);
        }
        let source = text(&turn["text"])?.chars().collect::<Vec<_>>();
        let origin = num(&turn["source_span"][0])?;
        for ((_, x, y), m) in expected.iter().zip(measured) {
            let start = a + sample(text(&m["startOffset"])?, rate, 1)?;
            let end = a + sample(text(&m["endOffset"])?, rate, 1)?;
            out.words.push(json!({"id":format!("source:{}:{}",origin+x,origin+y),"source_span":[origin+x,origin+y],"word":source[*x as usize..*y as usize].iter().collect::<String>(),"recognized_word":m["word"],"start_sample":start,"end_sample":end}));
        }
        out.start = Some(num(&out.words[0]["start_sample"])?);
        out.end = Some(num(&out.words.last().unwrap()["end_sample"])?);
    } else {
        if !response["transcript"].is_string()
            || response["turns"].as_array().is_none_or(|a| a.is_empty())
        {
            return Ok(out);
        }
        let exact = exact_numbers(body("response")?)?;
        let turns = list(&exact["turns"])?;
        let mut pieces = vec![];
        let mut previous = 0;
        for (i, t) in turns.iter().enumerate() {
            need(
                response["turns"][i]["startMs"].is_number()
                    && response["turns"][i]["endMs"].is_number(),
                "numeric millisecond offsets required",
            )?;
            let x = sample(text(&t["startMs"])?, rate, 1000)?;
            let y = sample(text(&t["endMs"])?, rate, 1000)?;
            need(
                x < y && y <= b - a && x >= previous,
                "independent interval order/domain",
            )?;
            previous = y;
            pieces.extend(tokens(text(&t["transcript"])?));
        }
        out.tokens = tokens(text(&response["transcript"])?);
        if out.tokens != pieces
            || out.tokens != expected.iter().map(|w| w.0.clone()).collect::<Vec<_>>()
            || out.tokens.is_empty()
        {
            return Ok(out);
        }
        out.start = Some(a + sample(text(&turns[0]["startMs"])?, rate, 1000)?);
        out.end = Some(a + sample(text(&turns.last().unwrap()["endMs"])?, rate, 1000)?);
    }
    out.passed = true;
    Ok(out)
}
fn wav_header(rate: u64, frames: u64) -> Result<Vec<u8>, StageError> {
    need(
        (8000..=192000).contains(&rate) && frames > 0 && frames <= (256 * META - 44) / 2,
        "native PCM domain",
    )?;
    let mut b = vec![];
    b.extend(b"RIFF");
    b.extend((36 + frames as u32 * 2).to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend((rate as u32).to_le_bytes());
    b.extend((rate as u32 * 2).to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend((frames as u32 * 2).to_le_bytes());
    Ok(b)
}
fn occurrences(stream: &[String], needle: &[String]) -> usize {
    if needle.is_empty() {
        0
    } else {
        stream
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }
}
fn clip(items: &[Value], [a, b]: [u64; 2]) -> Result<Vec<Value>, StageError> {
    let mut out = vec![];
    for w in items {
        let x = num(&w["start_sample"])?;
        let y = num(&w["end_sample"])?;
        if (x == y && a <= x && x < b) || (x < y && x < b && a < y) {
            out.push(json!({"original":w,"global_samples":[x,y],"local_samples":[a.max(x)-a,b.min(y)-a],"complete":a<=x&&y<=b}));
        }
    }
    Ok(out)
}
fn groups(words: &[Value]) -> Result<(Vec<Value>, Vec<String>), StageError> {
    let (mut out, mut unresolved, mut pending) = (vec![], vec![], vec![]);
    for w in words {
        let a = num(&w["start_sample"])?;
        let b = num(&w["end_sample"])?;
        if a == b {
            pending.push((w, a));
            continue;
        }
        let mut members = vec![];
        for (p, s) in pending.drain(..) {
            if s == a {
                members.push(p);
            } else {
                unresolved.push(text(&p["id"])?.to_owned());
            }
        }
        members.push(w);
        let ids = members
            .iter()
            .map(|v| text(&v["id"]).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        let names = members
            .iter()
            .map(|v| text(&v["word"]).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        out.push(json!({"id":format!("highlight:{}",ids.join("+")),"word":names.join(" "),"word_ids":ids,"words":names,"start_sample":a,"end_sample":b,"interval_word_id":w["id"]}));
    }
    for (w, _) in pending {
        unresolved.push(text(&w["id"])?.to_owned());
    }
    Ok((out, unresolved))
}
fn verify(files: &Files) -> Result<Value, StageError> {
    let index: Value = parse(&get(files, "chapter-import-index.json")?.1)?;
    let unit: Value = parse(&get(files, "chapter-import-unit.json")?.1)?;
    need(
        index["schema_version"] == 2
            && unit["schema_version"] == 2
            && index["recipe_sha256"] == recipe()
            && unit["recipe_sha256"] == recipe()
            && unit["index"]
                == serde_json::to_value(&get(files, "chapter-import-index.json")?.0).unwrap(),
        "import recipe/index binding",
    )?;
    let context: Value = parse(read(files, &index["context"])?)?;
    let spec = &context["spec"];
    need(
        context["schema_version"] == 2
            && context["recipe_sha256"] == recipe()
            && spec["schema_version"] == 2
            && matches!(
                text(&spec["provenance"])?,
                "synthetic" | "imported" | "diagnostic"
            ),
        "import context version/provenance",
    )?;
    id(&spec["source_scope_id"])?;
    let inventory = base64(text(&context["inventory_bytes"])?)?;
    let original = reference(&context["original_inventory"])?;
    original.verify(&inventory)?;
    need(
        parse::<Value>(&inventory)? == *spec,
        "original inventory identity",
    )?;
    let source_bytes = read(files, &index["source"])?;
    let source = std::str::from_utf8(source_bytes)
        .map_err(|_| StageError::Evidence("source UTF8".into()))?;
    let chars = source.chars().collect::<Vec<_>>();
    reference(&spec["source"])?.verify(source_bytes)?;
    let origin = num(&spec["source_origin"])?;
    need(
        origin <= MAX_SAFE_INTEGER - chars.len() as u64,
        "source origin",
    )?;
    let narration = reference(&spec["narration"])?;
    need(
        narration.bytes >= 46
            && narration.bytes <= 256 * META
            && (narration.bytes - 44) % 2 == 0
            && index["narration"] == spec["narration"],
        "narration domain",
    )?;
    let rate = num(&spec["sample_rate"])?;
    let frames = (narration.bytes - 44) / 2;
    wav_header(rate, frames)?;
    let turns = list(&spec["turns"])?;
    need(
        !turns.is_empty() && turns.len() <= 1024 && list(&spec["facts"])?.len() <= 1024,
        "turn/fact count",
    )?;
    let source_words = lex(source);
    let span_count = list(&spec["facts"])?
        .iter()
        .map(|f| list(&f["source_spans"]).map(Vec::len))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum::<usize>()
        .max(1);
    need(
        source_words
            .len()
            .checked_mul(span_count)
            .is_some_and(|n| n <= 2_000_000),
        "global ambiguity scan budget",
    )?;
    let mut refs = BTreeMap::new();
    for t in turns {
        for role in ["primary", "independent"] {
            if t[role].is_null() {
                continue;
            }
            for key in ["request", "response", "receipt", "recipe"] {
                if !t[role][key].is_null() {
                    let f = reference(&t[role][key])?;
                    need(f.bytes <= META, "raw metadata limit")?;
                    if let Some(old) = refs.insert(f.path.clone(), f.clone()) {
                        need(old == f, "raw identity conflict")?;
                    }
                }
            }
        }
    }
    let raw_object = context["raw"]
        .as_object()
        .ok_or_else(|| StageError::Evidence("raw inventory required".into()))?;
    need(refs.keys().eq(raw_object.keys()), "raw inventory coverage")?;
    let mut raw = BTreeMap::new();
    for (p, f) in &refs {
        let b = base64(text(&raw_object[p])?)?;
        f.verify(&b)?;
        raw.insert(p.clone(), b);
    }
    let (mut cursor, mut frame_cursor, mut ids, mut observations, mut source_units) =
        (origin, 0, BTreeSet::new(), vec![], vec![]);
    for t in turns {
        let key = id(&t["id"])?;
        need(
            ids.insert(key.to_owned()) && !text(&t["speaker"])?.trim().is_empty(),
            "turn id/speaker",
        )?;
        let [a, b] = span(&t["source_span"])?;
        let [x, y] = span(&t["frames"])?;
        need(
            a == cursor
                && a < b
                && b <= origin + chars.len() as u64
                && chars[(a - origin) as usize..(b - origin) as usize]
                    .iter()
                    .collect::<String>()
                    == text(&t["text"])?,
            "verbatim ordered source coverage",
        )?;
        let audio = reference(&t["audio"])?;
        need(
            x == frame_cursor
                && x < y
                && y <= frames
                && audio.bytes == 44 + (y - x) * 2
                && audio.bytes <= FILE,
            "complete turn PCM coverage/limit",
        )?;
        cursor = b;
        frame_cursor = y;
        observations.push((
            observe(t, "primary", &raw, rate)?,
            observe(t, "independent", &raw, rate)?,
        ));
        source_units.push(json!({"id":t["id"],"speaker":t["speaker"],"source_span":t["source_span"],"frames":t["frames"],"audio":t["audio"]}));
    }
    need(
        cursor == origin + chars.len() as u64
            && frame_cursor == frames
            && index["source_units"] == json!(source_units),
        "incomplete source/PCM tail/index",
    )?;
    let chunks = list(&index["chunks"])?;
    need(!chunks.is_empty() && chunks.len() <= 1024, "chunk count")?;
    let (mut next, mut chunk_ids, mut paths) = (0, BTreeMap::new(), BTreeSet::new());
    for c in chunks {
        let [a, b] = span(&c["frames"])?;
        let f = reference(&c["file"])?;
        let key = id(&c["id"])?;
        need(
            a == next
                && a < b
                && b <= frames
                && f.bytes == 44 + (b - a) * 2
                && f.bytes <= FILE
                && chunk_ids.insert(key.to_owned(), c).is_none()
                && paths.insert(f.path),
            "ordered complete chunk coverage",
        )?;
        next = b;
    }
    need(next == frames, "incomplete chunk tail")?;
    let present = list(&unit["included_chunks"])?
        .iter()
        .map(|v| text(v).map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    need(
        present.len() == list(&unit["included_chunks"])?.len()
            && present.contains(text(&unit["base_chunk"])?)
            && present.iter().all(|k| chunk_ids.contains_key(k)),
        "unit chunk selection",
    )?;
    let mut expected = vec![
        reference(&index["source"])?,
        reference(&index["context"])?,
        get(files, "chapter-import-index.json")?.0.clone(),
    ];
    for key in &present {
        let c = chunk_ids[key];
        let bytes = read(files, &c["file"])?;
        let [a, b] = span(&c["frames"])?;
        need(
            bytes.len() >= 44
                && bytes[..44] == wav_header(rate, b - a)?
                && digest(&bytes[44..]) == text(&c["pcm_sha256"])?,
            "chunk PCM identity",
        )?;
        expected.push(reference(&c["file"])?);
    }
    expected.sort_by(|a, b| a.path.cmp(&b.path));
    need(
        unit["files"] == serde_json::to_value(&expected).unwrap()
            && expected.len() + 5 <= 64
            && expected.iter().map(|f| f.bytes).sum::<u64>() + 5 * META <= TOTAL,
        "unit reference/aggregate limits",
    )?;
    let allowed = expected
        .iter()
        .map(|f| f.path.as_str())
        .chain([
            "chapter-import-unit.json",
            "chapter-import-helper-record.json",
            OUTPUT,
            "private/chapter-import-proposals.json",
            "private/chapter-import-evidence-inventory.json",
        ])
        .collect::<BTreeSet<_>>();
    need(
        files.keys().all(|p| allowed.contains(p.as_str())),
        "unlisted imported unit input",
    )?;
    for t in turns {
        let [a, b] = span(&t["frames"])?;
        let dependencies = chunks
            .iter()
            .filter(|c| {
                let x = c["frames"][0].as_u64().unwrap();
                let y = c["frames"][1].as_u64().unwrap();
                x < b && a < y
            })
            .collect::<Vec<_>>();
        if dependencies
            .iter()
            .all(|c| present.contains(c["id"].as_str().unwrap()))
        {
            let mut h = Sha256::new();
            h.update(wav_header(rate, b - a)?);
            for c in dependencies {
                let [x, y] = span(&c["frames"])?;
                let bytes = read(files, &c["file"])?;
                h.update(
                    &bytes[44 + (a.max(x) - x) as usize * 2..44 + (b.min(y) - x) as usize * 2],
                );
            }
            need(
                format!("{:x}", h.finalize()) == text(&t["audio"]["sha256"])?,
                "complete original turn PCM identity",
            )?;
        }
    }
    let complete = turns.iter().zip(&observations).all(|(t, (p, i))| {
        p.passed
            && i.passed
            && ["provider", "model"]
                .iter()
                .all(|k| t["primary"][*k] != t["independent"][*k])
            && ["request", "response"].iter().all(|k| {
                !t["primary"][*k].is_null()
                    && !t["independent"][*k].is_null()
                    && t["primary"][*k]["sha256"] != t["independent"][*k]["sha256"]
            })
    });
    let source_tokens = source_words.iter().map(|w| w.0.clone()).collect::<Vec<_>>();
    let primary_tokens = observations
        .iter()
        .flat_map(|(p, _)| p.tokens.clone())
        .collect::<Vec<_>>();
    let independent_tokens = observations
        .iter()
        .flat_map(|(_, i)| i.tokens.clone())
        .collect::<Vec<_>>();
    let mut fact_ids = BTreeSet::new();
    let mut facts = vec![];
    for f in list(&spec["facts"])? {
        let key = id(&f["id"])?;
        need(
            fact_ids.insert(key.to_owned()) && !list(&f["source_spans"])?.is_empty(),
            "fact identity/spans",
        )?;
        let mut support = BTreeSet::new();
        let mut ambiguous = false;
        for s in list(&f["source_spans"])? {
            let [a, b] = span(s)?;
            need(
                origin <= a && a < b && b <= origin + chars.len() as u64,
                "fact source domain",
            )?;
            let phrase = tokens(
                &chars[(a - origin) as usize..(b - origin) as usize]
                    .iter()
                    .collect::<String>(),
            );
            need(
                !phrase.is_empty()
                    && source_words.iter().all(|(_, x, y)| {
                        !(origin + x < a && a < origin + y || origin + x < b && b < origin + y)
                    }),
                "partial source word",
            )?;
            for (n, stream) in [&source_tokens, &primary_tokens, &independent_tokens]
                .into_iter()
                .enumerate()
            {
                for count in [
                    occurrences(stream, &phrase),
                    occurrences(stream, &phrase[phrase.len().saturating_sub(4)..]),
                ] {
                    ambiguous |= if n == 0 || complete {
                        count != 1
                    } else {
                        count > 1
                    };
                }
            }
            for (i, t) in turns.iter().enumerate() {
                let [x, y] = span(&t["source_span"])?;
                if x < b && a < y {
                    support.insert(i);
                }
            }
        }
        let mut required = BTreeSet::new();
        for i in &support {
            let [a, b] = span(&turns[*i]["frames"])?;
            for c in chunks {
                let [x, y] = span(&c["frames"])?;
                if x < b && a < y {
                    required.insert(text(&c["id"])?.to_owned());
                }
            }
        }
        let (mut outcome, mut reason, mut bound) = if ambiguous {
            ("uncertain", "global_source_or_terminal_ambiguity", None)
        } else if !complete {
            (
                "missing",
                "global_observations_or_independence_missing",
                None,
            )
        } else if support.iter().any(|i| {
            observations[*i]
                .0
                .end
                .unwrap()
                .abs_diff(observations[*i].1.end.unwrap())
                * 4
                > rate * 3
        }) {
            ("uncertain", "independent_endpoint_disagreement", None)
        } else if !required.is_subset(&present) {
            ("missing", "cross_cut_evidence_missing", None)
        } else {
            (
                "pass",
                "complete_global_observations_and_pcm_dependencies",
                support
                    .iter()
                    .map(|i| {
                        observations[*i]
                            .0
                            .end
                            .unwrap()
                            .max(observations[*i].1.end.unwrap())
                    })
                    .max(),
            )
        };
        if let Some(inherited) = spec["inherited_findings"].as_array() {
            for finding in inherited {
                let fid = text(&finding["id"])?;
                let o = text(&finding["outcome"])?;
                if [
                    format!("acoustic-fact:{key}"),
                    format!("chapter-import-fact:{key}"),
                ]
                .iter()
                .any(|s| s == fid)
                    && o != "pass"
                {
                    let rank = |s: &str| match s {
                        "fail" => 3,
                        "uncertain" => 2,
                        "missing" => 1,
                        _ => 0,
                    };
                    if rank(o) > rank(outcome) {
                        outcome = match o {
                            "fail" => "fail",
                            "uncertain" => "uncertain",
                            _ => "missing",
                        };
                    }
                    reason = "recorded_adverse_fact_finding";
                    bound = None;
                }
            }
        }
        facts.push(json!({"id":key,"source_spans":f["source_spans"],"turn_ids":support.iter().map(|i|turns[*i]["id"].clone()).collect::<Vec<_>>(),"required_chunks":required,"present_chunks":required.intersection(&present).cloned().collect::<Vec<_>>(),"outcome":outcome,"reason":reason,"not_before_sample":bound}));
    }
    let required = list(&spec["required_fact_ids"])?
        .iter()
        .map(|v| id(v).map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    need(
        required == fact_ids && required.len() == list(&spec["required_fact_ids"])?.len(),
        "fact inventory coverage",
    )?;
    let words = observations
        .iter()
        .flat_map(|(p, _)| p.words.clone())
        .collect::<Vec<_>>();
    let (groups, unresolved) = groups(&words)?;
    let global = span(&chunk_ids[text(&unit["base_chunk"])?]["frames"])?;
    Ok(
        json!({"schema_version":2,"provenance":"diagnostic","observation_provenance":spec["provenance"],"production_approved":false,"provider_calls":0,"outcome":"needs_repair","recipe_sha256":recipe(),"index_sha256":get(files,"chapter-import-index.json")?.0.sha256,"context_sha256":index["context"]["sha256"],"unit_sha256":get(files,"chapter-import-unit.json")?.0.sha256,"source_sha256":index["source"]["sha256"],"chapter_sha256":narration.sha256,"base_chunk":unit["base_chunk"],"global_frames":global,"checked_files":expected.iter().map(|f|(f.path.clone(),f.sha256.clone())).collect::<BTreeMap<_,_>>(),"complete_source_coverage":true,"global_observation_coverage":complete,"source_scalar_count":chars.len(),"word_count":words.len(),"highlight_group_count":groups.len(),"point_word_count":words.iter().filter(|w|w["start_sample"]==w["end_sample"]).count(),"unresolved_point_ids":unresolved,"words":clip(&words,global)?,"groups":clip(&groups,global)?,"facts":facts,"reveal_permissions":[],"acceptance_policy_authority":false}),
    )
}
pub(super) async fn execute(
    input: &StageInput,
    services: &dyn RunServices,
) -> Result<StageOutput, StageError> {
    need(
        input.inputs.len() <= 64
            && input.inputs.iter().all(|a| a.file.bytes <= FILE)
            && input.inputs.iter().map(|a| a.file.bytes).sum::<u64>() <= TOTAL,
        "bounded import envelope",
    )?;
    for a in &input.inputs {
        if a.file.media_type == "application/json"
            || [
                "chapter-import-unit.json",
                "chapter-import-index.json",
                "chapter-import-helper-record.json",
                "private/chapter-context.json",
                "private/chapter-import-proposals.json",
                "private/chapter-import-evidence-inventory.json",
                OUTPUT,
            ]
            .contains(&a.file.path.as_str())
        {
            need(a.file.bytes <= META, "import metadata limit")?;
        }
    }
    let files = inputs::load(input, services).await?.files;
    if !files.contains_key("chapter-import-helper-record.json") {
        return Ok(StageOutput {
            outcome: StageOutcome::NeedsRepair,
            artifacts: vec![],
            decisions: vec![],
            findings: vec![missing(
                "chapter-import-registration-pending",
                "Saved v2 record required; unregistered dispatch remains disabled.",
            )],
            cache_keys: vec![recipe()],
            library_updates: vec![],
            package_candidate: None,
        });
    }
    let report = verify(&files)?;
    let recorded: Value = parse(&get(&files, "chapter-import-helper-record.json")?.1)?;
    need(
        report == recorded,
        "recorded imported chapter verification changed",
    )?;
    let index: Value = parse(&get(&files, "chapter-import-index.json")?.1)?;
    let context: Value = parse(read(&files, &index["context"])?)?;
    let inherited: Vec<Finding> = if context["spec"]["inherited_findings"].is_null() {
        vec![]
    } else {
        serde_json::from_value(context["spec"]["inherited_findings"].clone())
            .map_err(|e| StageError::Evidence(e.to_string()))?
    };
    need(
        inherited.len() <= 1024 && inherited.iter().all(|f| valid_id(&f.id)),
        "inherited finding metadata",
    )?;
    let mut findings = vec![
        missing(
            "chapter-import-registration-pending",
            "Runtime owns whole-bundle import, v2 registration and bounded fanout.",
        ),
        missing(
            "acceptance-input-bindings",
            "No delivered presentation/player/checker/fact association bindings; acoustic observations cannot authorize public evidence or reveals.",
        ),
        missing(
            "production-verifier-pending",
            "Independent semantic verification remains required.",
        ),
    ];
    findings.extend(inherited.clone());
    for f in list(&report["facts"])? {
        findings.push(Finding {
            id: format!("chapter-import-fact:{}", text(&f["id"])?),
            field: "acoustic.chapter.import".into(),
            outcome: serde_json::from_value(f["outcome"].clone())
                .map_err(|e| StageError::Evidence(e.to_string()))?,
            message: text(&f["reason"])?.into(),
            evidence: input.inputs.clone(),
        });
    }
    let bindings = files
        .iter()
        .filter(|(p, _)| {
            ![
                OUTPUT,
                "private/chapter-import-evidence-inventory.json",
                "private/chapter-import-proposals.json",
            ]
            .contains(&p.as_str())
        })
        .map(|(p, (f, _))| (p.clone(), f.clone()))
        .collect::<BTreeMap<_, _>>();
    let validation = json!({"schema_version":2,"provenance":"diagnostic","outcome":"needs_repair","production_approved":false,"provider_calls":0,"helper_calls":0,"recipe_sha256":recipe(),"implementation_sha256":digest(include_bytes!("chapter_import.rs")),"bindings":bindings,"verified_import":report,"inherited_findings":inherited,"reveal_permissions":[],"runtime_policy_authority":false});
    let content_key = digest(&json(&validation)?);
    let outputs = [
        (OUTPUT, validation),
        (
            "private/chapter-import-proposals.json",
            json!(Vec::<AcceptanceEvidenceProposal>::new()),
        ),
        (
            "private/chapter-import-evidence-inventory.json",
            serde_json::to_value(AcceptanceEvidenceInventory {
                schema_version: 1,
                proposals: vec![],
            })
            .unwrap(),
        ),
    ];
    for (path, value) in &outputs {
        let b = json(value)?;
        need(b.len() as u64 <= META, "import output metadata limit")?;
        if input.mode == ExecutionMode::Replay
            && let Some((f, _)) = files.get(*path)
        {
            need(
                f.sha256 == digest(&b) && f.bytes == b.len() as u64,
                "stale imported chapter replay",
            )?;
        }
    }
    let mut artifacts = vec![];
    for (path, value) in outputs {
        artifacts.push(write(services, input, path, &value).await?);
    }
    services.check_cancelled().await?;
    Ok(StageOutput {
        outcome: StageOutcome::NeedsRepair,
        artifacts,
        decisions: vec![],
        findings,
        cache_keys: vec![recipe(), content_key],
        library_updates: vec![],
        package_candidate: None,
    })
}
