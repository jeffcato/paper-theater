//! All optimal global monotonic matches; never disambiguate using a turn/take's source span.
use super::*;
pub(super) struct Matches {
    pub source: Vec<monotonic_align::SourceToken>,
    pub candidates: Vec<Vec<usize>>,
    pub forced: Vec<Option<usize>>,
    pub source_candidates: Vec<bool>,
}
pub(super) fn align(
    source: &str,
    words: &[words::Word],
    limits: &Limits,
) -> Result<Matches, StageError> {
    let source = monotonic_align::tokenize_source(source);
    let (n, m) = (source.len(), words.len());
    need(
        n <= limits.max_source_tokens as usize,
        "primary source token limit",
    )?;
    let cells = (n + 1)
        .checked_mul(m + 1)
        .ok_or_else(|| StageError::Evidence("primary matching cell overflow".into()))?;
    need(
        cells as u64 <= limits.max_match_cells,
        "primary global matching work limit",
    )?;
    let width = m + 1;
    let at = |i: usize, j: usize| i * width + j;
    let mut prefix = vec![0u16; cells];
    let mut suffix = vec![0u16; cells];
    for i in 0..n {
        for j in 0..m {
            prefix[at(i + 1, j + 1)] = if source[i].normalized == words[j].normalized {
                prefix[at(i, j)] + 1
            } else {
                prefix[at(i, j + 1)].max(prefix[at(i + 1, j)])
            };
        }
    }
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            suffix[at(i, j)] = if source[i].normalized == words[j].normalized {
                suffix[at(i + 1, j + 1)] + 1
            } else {
                suffix[at(i + 1, j)].max(suffix[at(i, j + 1)])
            };
        }
    }
    let length = prefix[at(n, m)];
    let mut candidates = vec![vec![]; m];
    let mut edges = 0;
    let mut source_counts = vec![0; n];
    for i in 0..n {
        for j in 0..m {
            if source[i].normalized == words[j].normalized
                && prefix[at(i, j)] + 1 + suffix[at(i + 1, j + 1)] == length
            {
                edges += 1;
                need(
                    edges <= limits.max_candidate_edges,
                    "primary occurrence candidate bound",
                )?;
                candidates[j].push(i);
                source_counts[i] += 1;
            }
        }
    }
    let mut forced = vec![None; m];
    for j in 0..m {
        if candidates[j].len() != 1 {
            continue;
        }
        let i = candidates[j][0];
        let skip_heard = (0..=n).any(|k| prefix[at(k, j)] + suffix[at(k, j + 1)] == length);
        let skip_source = (0..=m).any(|k| prefix[at(i, k)] + suffix[at(i + 1, k)] == length);
        if !skip_heard && !skip_source && source_counts[i] == 1 {
            forced[j] = Some(i);
        }
    }
    Ok(Matches {
        source,
        candidates,
        forced,
        source_candidates: source_counts.iter().map(|n| *n > 0).collect(),
    })
}
