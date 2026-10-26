// loader group: hash-bound loading, diagnostic vs production acceptance.
// A diagnostic fixture must report missing acceptance IDs and be rejected
// by the production path.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  loadDiagnostic,
  loadRelease,
  sha256Hex,
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
