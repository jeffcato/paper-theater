"""Pure saved-response adapters. No transport, credentials, retries or ledgers."""
import difflib, math, re
from bisect import bisect_right
from decimal import Decimal, ROUND_CEILING
NORMALIZATION_VERSION = "asr-spelling-v1"
GROUPING_POLICY = "point-following-same-boundary-v1"


def tokens(text):
    return [w.replace("'", "").replace("’", "").casefold()
            for w in re.findall(r"[^\W_]+(?:['’][^\W_]+)*", text, re.UNICODE)]


def word_check(words, duration):
    previous_start = previous_end = 0.0
    for w in words:
        a, b = w["start"], w["end"]
        if not all(type(v) in (int, float) and math.isfinite(v) for v in [a, b]):
            raise ValueError("nonfinite word interval")
        if not (0 <= a <= b <= duration) or a < previous_start or b < previous_end:
            raise ValueError("invalid, unordered or duration-exceeding word interval")
        previous_start, previous_end = a, b


def normalized_words(words):
    result = []
    for w in words:
        normalized = tokens(w["word"])
        if len(normalized) != 1:
            raise ValueError("a measured interval must identify exactly one source word")
        result.append(normalized[0])
    return result


def compare_text(expected, words, *, allow_em_initial=False):
    heard = normalized_words(words)
    raw = difflib.SequenceMatcher(None, expected, heard, autojunk=False)

    def groups(values):
        result, i = [], 0
        while i < len(values):
            size = 2 if values[i:i + 2] == ["trap", "door"] else 1
            result.append(("trapdoor" if size == 2 else values[i], i, i + size))
            i += size
        return result

    source_groups, heard_groups = groups(expected), groups(heard)
    matcher = difflib.SequenceMatcher(None, [g[0] for g in source_groups],
                                     [g[0] for g in heard_groups], autojunk=False)
    mappings, differences, normalizations = [], [], []
    for tag, a, b, c, d in matcher.get_opcodes():
        if tag == "equal" or (tag == "replace" and b - a == d - c):
            for source, acoustic in zip(source_groups[a:b], heard_groups[c:d]):
                name_variant = allow_em_initial and source[0] == "em" and acoustic[0] == "m"
                if source[0] != acoustic[0] and not name_variant:
                    differences.append(["replace", source[1], source[2], acoustic[1], acoustic[2]])
                    continue
                if expected[source[1]:source[2]] == heard[acoustic[1]:acoustic[2]]:
                    mappings.extend({"source_range": [si, si + 1], "heard_range": [hi, hi + 1], "kind": "exact"}
                                    for si, hi in zip(range(source[1], source[2]), range(acoustic[1], acoustic[2])))
                    continue
                kind = "initial_name" if name_variant else "compound" if source[2] - source[1] != acoustic[2] - acoustic[1] else "exact"
                mapping = {"source_range": list(source[1:]), "heard_range": list(acoustic[1:]), "kind": kind}
                mappings.append(mapping)
                if kind != "exact":
                    normalizations.append({**mapping, "source": expected[source[1]:source[2]],
                                           "heard": heard[acoustic[1]:acoustic[2]]})
        else:
            sr = [source_groups[a][1], source_groups[b - 1][2]] if a < b else [source_groups[a][1] if a < len(source_groups) else len(expected)] * 2
            hr = [heard_groups[c][1], heard_groups[d - 1][2]] if c < d else [heard_groups[c][1] if c < len(heard_groups) else len(heard)] * 2
            differences.append([tag, *sr, *hr])
    matched = sum(m["source_range"][1] - m["source_range"][0] for m in mappings)
    return {"coverage": matched / len(expected) if expected else 0, "exact": not differences and bool(expected),
        "differences": differences, "mappings": mappings, "normalizations": normalizations,
        "raw_exact": expected == heard,
        "raw_coverage": sum(block.size for block in raw.get_matching_blocks()) / len(expected) if expected else 0,
        "raw_differences": [list(op) for op in raw.get_opcodes() if op[0] != "equal"],
        "normalization_policy": {"version": NORMALIZATION_VERSION, "compounds": [["trap", "door"]],
                                 "em_initial_in_scope": allow_em_initial}}


def offset(value):
    if not isinstance(value, str) or not re.fullmatch(r"\d+(?:\.\d{1,9})?s", value):
        raise ValueError("missing or malformed measured word offset")
    return float(Decimal(value[:-1]))


def parse_words(response):
    candidates = response.get("candidates", [])
    if len(candidates) != 1 or candidates[0].get("finishReason") != "STOP":
        raise ValueError("missing, refused or incomplete transcription")
    words, found = [], False
    for part in candidates[0].get("content", {}).get("parts", []):
        if "audioTranscription" not in part:
            continue
        transcription = part["audioTranscription"]
        if not isinstance(transcription.get("words"), list):
            raise ValueError("word timing annotations are missing")
        found = True
        for word in transcription["words"]:
            words.append({"word": word["word"], "start": offset(word.get("startOffset")),
                          "end": offset(word.get("endOffset"))})
    if not found:
        text = "".join(p.get("text", "") for p in candidates[0].get("content", {}).get("parts", []))
        if not text.strip():
            return []  # Completed ASR with no recognized speech; never supplies source-word coverage.
        raise ValueError("model returned no measured word timing annotations")
    return words


def candidate_track(cases, results, sample_rate):
    track, shared = [], []
    for case, result in zip(cases, results):
        source = list(re.finditer(r"[^\W_]+(?:['’][^\W_]+)*", case["expected_text"], re.UNICODE))
        for mapping in result["mappings"]:
            a, b = mapping["source_range"]
            c, d = mapping["heard_range"]
            start = case["offset_sample"] + sample_at(result["words"][c]["start"], sample_rate)
            end = case["offset_sample"] + sample_at(result["words"][d - 1]["end"], sample_rate)
            if b - a != 1:
                shared.append({"case": case["id"], **mapping, "start_sample": start, "end_sample": end})
                continue
            span = [case["source_span_characters"][0] + source[a].start(),
                    case["source_span_characters"][0] + source[a].end()]
            track.append({"id": f"source:{span[0]}:{span[1]}", "source_span_characters": span,
                "word": source[a].group(), "recognized_words": [w["word"] for w in result["words"][c:d]],
                "start_sample": start, "end_sample": end,
                "timing_kind": "point" if start == end else "interval", "normalization": mapping["kind"]})
    return track, shared


def parse_turns(response, duration):
    if not isinstance(response.get("transcript"), str) or not response.get("turns"):
        raise ValueError("missing transcript or measured speech-turn timestamps")
    turns = []
    for turn in response["turns"]:
        a, b = turn.get("startMs"), turn.get("endMs")
        if not all(type(x) in [int, float] and math.isfinite(x) for x in [a, b]):
            raise ValueError("missing or invalid measured turn offsets")
        if not 0 <= a < b <= duration * 1000 or not isinstance(turn.get("transcript"), str):
            raise ValueError("turn outside actual audio or missing turn text")
        turns.append({"start": a / 1000, "end": b / 1000, "text": turn["transcript"]})
    return turns


def sample_at(seconds, sample_rate):
    return int((Decimal(str(seconds)) * sample_rate).to_integral_value(rounding=ROUND_CEILING))


class WordTrack:
    def __init__(self, words):
        self.words = tuple(dict(w) for w in words)
        previous_start = previous_end = 0
        for word in self.words:
            a, b = word["start_sample"], word["end_sample"]
            if (type(a) is not int or type(b) is not int or not 0 <= a <= b
                    or a < previous_start or b < previous_end):
                raise ValueError("invalid or unordered word samples")
            previous_start, previous_end = a, b
        self.starts = tuple(w["start_sample"] for w in self.words)

    def active_word(self, sample):
        index = bisect_right(self.starts, sample) - 1
        if index < 0:
            return None
        word = self.words[index]
        return word if word["start_sample"] <= sample < word["end_sample"] else None


def highlight_groups(words):
    """Attach points to the next positive interval at the same measured boundary."""
    words = WordTrack(words).words
    groups, pending, unresolved = [], [], []
    for index, word in enumerate(words):
        member = {"id": word.get("id", f"word:{index}"), "word": word["word"]}
        if word["start_sample"] == word["end_sample"]:
            pending.append((member, word["start_sample"]))
            continue
        attached = [member for member, sample in pending if sample == word["start_sample"]]
        unresolved.extend(member["id"] for member, sample in pending if sample != word["start_sample"])
        members = [*attached, member]
        groups.append({"id": "highlight:" + "+".join(m["id"] for m in members),
            "word": " ".join(m["word"] for m in members), "word_ids": [m["id"] for m in members],
            "words": [m["word"] for m in members], "start_sample": word["start_sample"],
            "end_sample": word["end_sample"], "interval_word_id": member["id"]})
        pending = []
    unresolved.extend(member["id"] for member, _ in pending)
    return groups, unresolved


class HighlightTrack(WordTrack):
    def active_group(self, sample):
        return self.active_word(sample)
