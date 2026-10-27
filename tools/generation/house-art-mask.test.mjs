// Red-first tests for tools/generation/house-art-mask.mjs (SAM 3.1 masking).
// All offline: no key, no network. Wire format follows
// https://dev.meta.ai/docs/sam/segmenting and
// https://dev.meta.ai/docs/sam/reading-segmentation.
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const run = promisify(execFile);
const TOOL = new URL("./house-art-mask.mjs", import.meta.url).pathname;
const { buildRequestInput, accumulateStreamText } = await import(TOOL);
const textOf = (input) => input[0].content.find((q) => q.type === "input_text").text;
const imgOf = (input) => input[0].content.find((q) => q.type === "input_image").image_url;

const BOX = { x1: 12, y1: 8, x2: 2000, y2: 1200 };
const NEG = [[4, 4], [2040, 1236]];
const SIZE = { w: 2048, h: 1245 };

test("box prompt embeds exact media pixels and measured box", () => {
  const input = buildRequestInput({
    mode: "box", size: SIZE, objectId: 1, box: BOX, negPoints: NEG,
    imageDataUrl: "data:image/png;base64,AAA",
  });
  const text = textOf(input);
  assert.match(text, /x1=12;y1=8;x2=2000;y2=1200;w=2048;h=1245/);
  assert.match(text, /w=2048;h=1245/);
  const img = imgOf(input);
  assert.ok(img.startsWith("data:image/png;base64,"));
});

test("negative points are marked negative in the prompt", () => {
  const input = buildRequestInput({
    mode: "box", size: SIZE, objectId: 1, box: BOX, negPoints: NEG,
    imageDataUrl: "data:image/png;base64,AAA",
  });
  const text = textOf(input);
  assert.match(text, /-\<\|point;x=4;y=4;w=2048;h=1245\|\>/);
  assert.match(text, /-\<\|point;x=2040;y=1236;w=2048;h=1245\|\>/);
});

test("positive points serialize without a minus prefix", () => {
  const input = buildRequestInput({
    mode: "box", size: SIZE, objectId: 1, box: BOX,
    posPoints: [[1024, 200]], negPoints: NEG,
    imageDataUrl: "data:image/png;base64,AAA",
  });
  const t = textOf(input);
  assert.match(t, /<\|point;x=1024;y=200;w=2048;h=1245\|>/);
  assert.doesNotMatch(t, /-<\|point;x=1024;y=200/);
});

test("concept mode sends one short noun phrase, no boxes", () => {
  const input = buildRequestInput({
    mode: "concept", concept: "brass ticket window",
    imageDataUrl: "data:image/png;base64,AAA",
  });
  assert.equal(textOf(input), "brass ticket window");
});

test("SSE accumulation returns completed output text", async () => {
  const sse = [
    `data: {"type":"response.output_text.delta","delta":"<0f>1<|box;"}`,
    `data: {"type":"response.output_text.delta","delta":"x1=1;y1=2|>}"}`,
    `data: {"type":"response.completed","response":{"status":"completed"}}`,
  ].join("\n");
  const text = accumulateStreamText(sse);
  assert.equal(text, "<0f>1<|box;x1=1;y1=2|>}");
});

test("mask applier self-test passes (stdlib PNG codec round-trip)", async () => {
  const { stdout } = await run("python3",
    [new URL("./sam_mask_apply.py", import.meta.url).pathname, "--self-test"],
    { cwd: new URL("../..", import.meta.url).pathname });
  assert.match(stdout, /SELF-TEST OK/);
});

test("flood-cut self-test passes (border white cut, enclosed fills kept)", async () => {
  const { stdout } = await run("python3",
    [new URL("./sam_mask_apply.py", import.meta.url).pathname, "--flood-self-test"],
    { cwd: new URL("../..", import.meta.url).pathname });
  assert.match(stdout, /FLOOD-SELF-TEST OK/);
});
