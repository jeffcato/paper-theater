//! Conservative source boundaries. An unsupported bound retains prose in an unresolved unit.
use super::*;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Unit {
    pub id: Id,
    pub turn_id: Option<Id>,
    pub speaker: Option<Id>,
    pub kind: String,
    pub span: SourceSpan,
    pub text: String,
    pub request_ready: bool,
}
#[derive(Serialize)]
pub(super) struct Acting {
    pub speaker: Id,
    pub span: SourceSpan,
    pub acting: String,
    pub support: Value,
}
pub(super) fn acting(pilot: &Value, source: &str) -> Vec<Acting> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut out = vec![];
    for performer in pilot["performers"].as_array().unwrap() {
        for p in performer["passages"].as_array().into_iter().flatten() {
            if p["acting"].as_str().unwrap_or("").is_empty() {
                continue;
            }
            let span: SourceSpan = serde_json::from_value(p["span"].clone()).unwrap();
            let c = &p["acting_support"];
            if let (Some(a), Some(b), Some(quote)) = (
                c["span"][0].as_u64(),
                c["span"][1].as_u64(),
                c["quote"].as_str(),
            ) {
                if span[0] <= a
                    && a < b
                    && b <= span[1]
                    && b <= chars.len() as u64
                    && chars[a as usize..b as usize].iter().collect::<String>() == quote
                {
                    out.push(Acting {
                        speaker: performer["speaker"].as_str().unwrap().into(),
                        span,
                        acting: p["acting"].as_str().unwrap().into(),
                        support: c.clone(),
                    });
                }
            }
        }
    }
    out
}
fn boundary(chars: &[char], i: usize) -> bool {
    if i == chars.len() {
        return true;
    }
    if chars[..i].ends_with(&['\n', '\n']) || chars[..i].ends_with(&['\r', '\n', '\r', '\n']) {
        return true;
    }
    if !chars[i].is_whitespace() {
        return false;
    }
    let prior = chars[i.saturating_sub(80)..i].iter().collect::<String>();
    let trimmed = prior.trim_end_matches(['”', '"', '’', '\'']);
    if trimmed.ends_with(['!', '?']) {
        return true;
    }
    if !trimmed.ends_with('.') {
        return prior.ends_with("\n\n");
    }
    let token = trimmed
        .trim_end_matches('.')
        .split_whitespace()
        .last()
        .unwrap_or("");
    token.chars().count() > 2
        && !matches!(
            token.to_lowercase().as_str(),
            "mr" | "mrs" | "dr" | "prof" | "st" | "etc" | "e.g" | "i.e"
        )
}
pub(super) fn compile(
    direction: &Value,
    limits: &Limits,
    acting: &[Acting],
    findings: &mut Vec<Finding>,
) -> Result<Vec<Unit>, StageError> {
    let source = direction["source_text"]
        .as_str()
        .ok_or_else(|| StageError::Evidence("missing verified prose".into()))?;
    let chars = source.chars().collect::<Vec<_>>();
    let n = chars.len();
    let mut units: Vec<Unit> = vec![];
    let mut cursor = 0;
    for turn in direction["turns"]
        .as_array()
        .ok_or_else(|| StageError::Evidence("missing verified turns".into()))?
    {
        let t = &turn["proposal"];
        let a = t["span"][0].as_u64().unwrap() as usize;
        let end = t["span"][1].as_u64().unwrap() as usize;
        need(
            a == cursor
                && a < end
                && end <= n
                && chars[a..end].iter().collect::<String>() == t["text"],
            "Direction turns cannot reconstruct complete ordered source",
        )?;
        let text = chars[a..end].to_vec();
        let mut sizes = vec![0u64];
        for c in &text {
            sizes.push(sizes.last().unwrap() + c.len_utf8() as u64);
        }
        let cuts = acting
            .iter()
            .filter(|p| json!(p.speaker) == t["speaker"])
            .flat_map(|p| p.span)
            .filter(|i| *i > a as u64 && *i < end as u64)
            .map(|i| i as usize - a)
            .collect::<BTreeSet<_>>();
        let options = (1..=text.len())
            .filter(|i| boundary(&text, *i) || cuts.contains(i))
            .collect::<Vec<_>>();
        let mut start = 0;
        while start < text.len() {
            if units.len() == limits.max_units as usize {
                let last = units.last_mut().unwrap();
                last.span[1] = n as u64;
                last.text = chars[last.span[0] as usize..].iter().collect();
                last.turn_id = None;
                last.speaker = None;
                last.kind = "unresolved".into();
                last.request_ready = false;
                findings.push(missing("speech-unit-limit","Configured unit count retains the complete remaining source in an unresolved unit."));
                return Ok(units);
            }
            let hard_end = cuts
                .range((start + 1)..)
                .next()
                .copied()
                .unwrap_or(text.len());
            let last = options
                .iter()
                .copied()
                .filter(|i| *i > start && *i <= hard_end)
                .take_while(|i| sizes[*i] - sizes[start] <= limits.max_unit_text_bytes)
                .last();
            let stop =
                last.unwrap_or_else(|| options.iter().copied().find(|i| *i > start).unwrap());
            let body = text[start..stop].iter().collect::<String>();
            let ready = last.is_some() && turn["speaker_source_supported"] == true;
            if !ready {
                findings.push(missing("speech-unit-unresolved","A complete source sentence/turn exceeds the bound or lacks supported speaker attribution; prose is retained without a synthesis request."));
            }
            units.push(Unit {
                id: format!("u{:04}", units.len()),
                turn_id: t["id"].as_str().map(String::from),
                speaker: t["speaker"].as_str().map(String::from),
                kind: t["kind"].as_str().unwrap().into(),
                span: [(a + start) as u64, (a + stop) as u64],
                text: body,
                request_ready: ready,
            });
            start = stop;
        }
        cursor = end;
    }
    need(
        cursor == n && !units.is_empty(),
        "source turn inventory is incomplete",
    )?;
    need(
        units.iter().map(|u| u.text.as_str()).collect::<String>() == source,
        "speech plan changed prose",
    )?;
    Ok(units)
}
