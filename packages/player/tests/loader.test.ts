// loader group: hash-bound loading, diagnostic vs production acceptance.
// A diagnostic fixture must report missing acceptance IDs and be rejected
// by the production path.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  inspectRelease,
  loadDiagnostic,
  loadRelease,
  sha256Hex,
  MAX_AUDIO_BYTES,
  MAX_FILE_BYTES,
  MAX_TOTAL_BYTES,
} from "../src/index.js";

function fileRef(path: string, bytes: Uint8Array, media_type: string) {
  return { path, bytes: bytes.length, media_type };
}

async function diagnosticManifestBytes() {
  const acceptanceBytes = new TextEncoder().encode("acceptance-stub");
  const acceptance = {
    ...(fileRef("acceptance.json", acceptanceBytes, "application/json")),
    sha256: await sha256Hex(acceptanceBytes),
  };
  const manifest = {
    schema_version: 1,
    capability_version: "paper-player-v1",
    provenance: "diagnostic",
    title: "Diagnostic stub",
    author: "Agent 4",
    chapter_label: "two-scene-local",
    source_sha256: "0".repeat(64),
    sample_rate: 48000,
    sample_count: 96000,
    entries: {},
    assets: [],
    reveals: [
      {
        id: "rg-later",
        contributor_ids: ["actor-later"],
        not_before_sample: 48000,
        facts: [
          {
            source_span: [0, 5],
            image_attestation_id: "missing-attestation",
            primary_anchor_id: "missing-primary",
            independent_anchor_id: "missing-independent",
          },
        ],
      },
    ],
    acceptance,
  };
  const bytes = new TextEncoder().encode(JSON.stringify(manifest));
  return { bytes, digest: await sha256Hex(bytes), acceptanceBytes };
}

const policy = {
  player_sha256: "0".repeat(64),
  checker_sha256: "0".repeat(64),
  recipe_sha256: "0".repeat(64),
  allow_diagnostic: false,
};

describe("loader: diagnostic vs production acceptance", () => {
  it("production path rejects diagnostic provenance", async () => {
    const { bytes, digest, acceptanceBytes } = await diagnosticManifestBytes();
    const read = async (path: string) => {
      assert.equal(path, "acceptance.json");
      return acceptanceBytes;
    };
    await assert.rejects(loadRelease(bytes, digest, read, policy), /provenance/);
  });

  it("diagnostic entry reports missing acceptance IDs without accepting", async () => {
    const { bytes, digest, acceptanceBytes } = await diagnosticManifestBytes();
    const read = async () => acceptanceBytes;
    const handle = await loadDiagnostic(bytes, digest, read, policy);
    assert.equal(handle.accepted, false);
    assert.ok(handle.missing_acceptance_ids.includes("missing-attestation"));
    assert.ok(handle.missing_acceptance_ids.includes("missing-primary"));
    assert.ok(handle.missing_acceptance_ids.includes("missing-independent"));
  });

  it("production path rejects digest mismatch without reading further", async () => {
    const { bytes, acceptanceBytes } = await diagnosticManifestBytes();
    let reads = 0;
    const read = async () => {
      reads++;
      return acceptanceBytes;
    };
    await assert.rejects(loadRelease(bytes, "f".repeat(64), read, policy), /digest/i);
    assert.equal(reads, 0);
  });
});

describe("loader: chapter-sized media", () => {
  const FRAMES_180S = 180 * 48000;

  /** Real PCM16 stereo 48 kHz WAV bytes (ramp pattern, decodable). */
  function wavBytes(frames: number, channels = 2): Buffer {
    const data = frames * channels * 2;
    const buf = Buffer.alloc(44 + data);
    buf.write("RIFF", 0);
    buf.writeUInt32LE(36 + data, 4);
    buf.write("WAVE", 8);
    buf.write("fmt ", 12);
    buf.writeUInt32LE(16, 16);
    buf.writeUInt16LE(1, 20);
    buf.writeUInt16LE(channels, 22);
    buf.writeUInt32LE(48000, 24);
    buf.writeUInt32LE(48000 * channels * 2, 28);
    buf.writeUInt16LE(channels * 2, 32);
    buf.writeUInt16LE(16, 34);
    buf.write("data", 36);
    buf.writeUInt32LE(data, 40);
    for (let i = 0; i < frames * channels; i++) {
      buf.writeInt16LE((i % 65536) - 32768, 44 + i * 2);
    }
    return buf;
  }

  function entry(path: string, bytes: Uint8Array, media_type: string) {
    return { path, bytes: bytes.length, media_type };
  }

  async function chapterManifest(audioBytes: Uint8Array, frames: number, audioSha?: string) {
    const metaBytes = new TextEncoder().encode(JSON.stringify({ chapter: 1 }));
    const audio = {
      ...entry("audio/master.wav", audioBytes, "audio/wav"),
      sha256: audioSha ?? (await sha256Hex(audioBytes)),
    };
    const meta = {
      ...entry("meta.json", metaBytes, "application/json"),
      sha256: await sha256Hex(metaBytes),
    };
    const acceptanceBytes = new TextEncoder().encode("acceptance-stub");
    const manifest = {
      schema_version: 1,
      capability_version: "paper-player-v1",
      provenance: "diagnostic",
      title: "Chapter master stub",
      author: "Agent 4",
      chapter_label: "chapter-1",
      source_sha256: "0".repeat(64),
      sample_rate: 48000,
      sample_count: frames,
      entries: { audio, meta },
      assets: [],
      reveals: [],
      acceptance: {
        ...entry("acceptance.json", acceptanceBytes, "application/json"),
        sha256: await sha256Hex(acceptanceBytes),
      },
    };
    const bytes = new TextEncoder().encode(JSON.stringify(manifest));
    return { bytes, digest: await sha256Hex(bytes), files: new Map([["audio/master.wav", audioBytes], ["meta.json", metaBytes], ["acceptance.json", acceptanceBytes]]) };
  }

  it("loads a valid >32 MiB delivery master with exact bytes and hash", async () => {
    const audio = wavBytes(FRAMES_180S);
    assert.equal(audio.length, 34560044);
    assert.ok(audio.length > MAX_FILE_BYTES);
    const { bytes, digest, files } = await chapterManifest(audio, FRAMES_180S);
    // Handles stay byte-free by contract; the spy retains what the bounded
    // fetch delivered for the exact-bytes/hash proof below.
    const seen = new Map<string, Uint8Array>();
    const read = async (path: string) => {
      const chunk = files.get(path)!;
      seen.set(path, chunk);
      return chunk;
    };
    const inspection = await inspectRelease(bytes, digest, read, policy);
    assert.ok(!inspection.issues.some((i) => i.code === "invalid_wav" || i.code === "bounds"), JSON.stringify(inspection.issues));
    const handle = await loadDiagnostic(bytes, digest, read, policy);
    // Disposal-friendly: the 34 MiB never pins to the handle.
    assert.equal(handle.files.size, 0);
    const loaded = seen.get("audio/master.wav")!;
    assert.equal(loaded.length, 34560044);
    assert.equal(await sha256Hex(loaded), await sha256Hex(audio));
  });

  it("accepts audio at exactly the 256 MiB ceiling without reading past it", async () => {
    assert.equal(MAX_AUDIO_BYTES, 256 * 1024 * 1024);
    const zeros = Buffer.alloc(MAX_AUDIO_BYTES);
    const { bytes, digest, files } = await chapterManifest(zeros, 1, await sha256Hex(zeros));
    files.set("audio/master.wav", zeros);
    let reads = 0;
    const read = async (path: string) => {
      reads++;
      return files.get(path)!;
    };
    const inspection = await inspectRelease(bytes, digest, read, policy);
    assert.ok(!inspection.issues.some((i) => i.code === "bounds"), JSON.stringify(inspection.issues));
    assert.ok(reads > 0);
  });

  it("rejects audio over the ceiling before reading a byte", async () => {
    const audio = wavBytes(48);
    const { bytes, digest, files } = await chapterManifest(audio, 48);
    const manifest = JSON.parse(Buffer.from(bytes).toString("utf8"));
    manifest.entries.audio.bytes = MAX_AUDIO_BYTES + 1;
    const over = new TextEncoder().encode(JSON.stringify(manifest));
    let reads = 0;
    const read = async (path: string) => {
      reads++;
      return files.get(path)!;
    };
    await assert.rejects(loadDiagnostic(over, await sha256Hex(over), read, policy), /bounds/);
    assert.equal(reads, 0);
  });

  it("keeps tight non-audio bounds", async () => {
    const audio = wavBytes(48);
    const { bytes, digest, files } = await chapterManifest(audio, 48);
    const manifest = JSON.parse(Buffer.from(bytes).toString("utf8"));
    manifest.entries.meta.bytes = MAX_FILE_BYTES + 1;
    manifest.entries.meta.media_type = "application/json";
    // Bad ref first: the bound must fire before any artifact is fetched.
    manifest.entries = { meta: manifest.entries.meta, audio: manifest.entries.audio };
    const over = new TextEncoder().encode(JSON.stringify(manifest));
    let reads = 0;
    const read = async (path: string) => {
      reads++;
      return files.get(path)!;
    };
    await assert.rejects(loadDiagnostic(over, await sha256Hex(over), read, policy), /bounds/);
    assert.equal(reads, 0);
    assert.equal(MAX_TOTAL_BYTES, 256 * 1024 * 1024);
  });

  it("keeps non-master audio under the 32 MiB file cap", async () => {
    const audio = wavBytes(480);
    const { bytes, digest, files } = await chapterManifest(audio, 480);
    const manifest = JSON.parse(Buffer.from(bytes).toString("utf8"));
    // A 100 MiB sidecar asset labeled audio/wav is not the delivery master,
    // so the chapter ceiling must not cover it.
    manifest.assets.push({
      path: "audio/b.wav",
      bytes: 100 * 1024 * 1024,
      media_type: "audio/wav",
      sha256: "e".repeat(64),
    });
    const over = new TextEncoder().encode(JSON.stringify(manifest));
    let reads = 0;
    const read = async (path: string) => {
      reads++;
      return files.get(path)!;
    };
    await assert.rejects(loadDiagnostic(over, await sha256Hex(over), read, policy), /bounded/);
  });

  it("caps the non-audio total without spending the audio budget", async () => {
    const audio = wavBytes(480);
    const { bytes, digest, files } = await chapterManifest(audio, 480);
    const manifest = JSON.parse(Buffer.from(bytes).toString("utf8"));
    // Seven real 32 MiB JSON sidecars followed by one more 32 MiB claim push
    // the shared non-audio total past 256 MiB. The final claim is never read:
    // the bound fires on the declared bytes first.
    const filler = Buffer.alloc(32 * 1024 * 1024);
    const fillerSha = await sha256Hex(filler);
    manifest.assets = [];
    for (let i = 0; i < 7; i++) {
      manifest.assets.push({
        path: `meta/${i}.json`,
        bytes: filler.length,
        media_type: "application/json",
        sha256: fillerSha,
      });
      files.set(`meta/${i}.json`, filler);
    }
    manifest.assets.push({
      path: "meta/7.json",
      bytes: filler.length,
      media_type: "application/json",
      sha256: "e".repeat(64),
    });
    const over = new TextEncoder().encode(JSON.stringify(manifest));
    const readPaths: string[] = [];
    const read = async (path: string) => {
      readPaths.push(path);
      return files.get(path)!;
    };
    await assert.rejects(loadDiagnostic(over, await sha256Hex(over), read, policy), /total/i);
    assert.ok(!readPaths.includes("meta/7.json"), `over-budget ref was read: ${readPaths.join(",")}`);
  });

  it("reports malformed and truncated audio without accepting it blindly", async () => {
    const garbage = new TextEncoder().encode("not a wav at all, just text bytes...........");
    const { bytes, digest, files } = await chapterManifest(garbage, 48);
    const read = async (path: string) => files.get(path)!;
    const inspection = await inspectRelease(bytes, digest, read, policy);
    assert.ok(inspection.issues.some((i) => i.code === "invalid_wav" && i.target === "audio"), JSON.stringify(inspection.issues));
    await assert.rejects(loadRelease(bytes, digest, read, policy), /invalid_wav/);

    const header = wavBytes(48);
    const truncated = header.subarray(0, 100);
    const second = await chapterManifest(truncated, 48);
    const read2 = async (path: string) => second.files.get(path)!;
    const inspection2 = await inspectRelease(second.bytes, second.digest, read2, policy);
    assert.ok(inspection2.issues.some((i) => i.code === "invalid_wav"), JSON.stringify(inspection2.issues));
  });

  it("rejects wrong hash and wrong size", async () => {
    const audio = wavBytes(480);
    const { bytes, digest, files } = await chapterManifest(audio, 480);
    const short = audio.subarray(0, audio.length - 2);
    const readShort = async (path: string) => (path === "audio/master.wav" ? short : files.get(path)!);
    await assert.rejects(loadDiagnostic(bytes, digest, readShort, policy), /identity/i);
    const flipped = Buffer.from(audio);
    flipped[60]! ^= 0xff;
    const readFlipped = async (path: string) => (path === "audio/master.wav" ? flipped : files.get(path)!);
    await assert.rejects(loadDiagnostic(bytes, digest, readFlipped, policy), /identity/i);
  });

  it("surfaces read failures with no partial handle", async () => {
    const audio = wavBytes(480);
    const { bytes, digest, files } = await chapterManifest(audio, 480);
    let calls = 0;
    const read = async (path: string) => {
      calls++;
      if (path === "meta.json") throw new Error("fetch failed: meta.json");
      return files.get(path)!;
    };
    await assert.rejects(loadDiagnostic(bytes, digest, read, policy), /fetch failed/);
    assert.ok(calls >= 1);
  });
});
