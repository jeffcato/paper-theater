#!/usr/bin/env node
// Banner control art (player lane). Turns the five editor-side raw
// lithograph sheets into the delivered house-art PNGs and writes their
// provenance records next to the other diagnostic fixtures. It makes no
// provider call. Needs macOS `sips` for the JPEG decode and the resize.
//
//   node scripts/banner-art.mjs <raw-dir>
//
// <raw-dir> holds banner-<id>.raw.jpg. For each sheet the script:
//   1. decodes to BMP and measures the painted piece against its grey ground
//   2. crops a fixed window around the piece (square for the round buttons)
//   3. floods the ground that touches the border to exact #808080, so the
//      player's border keying meets a clean, even edge
//   4. resizes to the brief's deliver size and records size and digest
import { execFile } from "node:child_process"
import { createHash } from "node:crypto"
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { crc32, deflateSync } from "node:zlib"

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, "..", "..", "..")
const outDir = join(root, "fixtures", "generation", "immersive-house-test-v2")
const GROUND = 128

// id -> deliver size and crop mode. Sizes mirror the brief (src/house-brief.ts).
const SHEETS = [
  { id: "banner.play", file: "banner-play", deliver: [512, 512], mode: "square" },
  { id: "banner.restart", file: "banner-restart", deliver: [512, 512], mode: "square" },
  { id: "banner.readalong", file: "banner-readalong", deliver: [512, 512], mode: "square" },
  { id: "banner.return", file: "banner-return", deliver: [512, 512], mode: "square" },
  { id: "banner.seek", file: "banner-seek", deliver: [1280, 240], mode: "wide" },
]

const sh = (cmd, args) =>
  new Promise((ok, fail) => execFile(cmd, args, (err, out) => (err ? fail(err) : ok(out))))

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex")

// -- BMP in, PNG out ---------------------------------------------------------

const readBmp = (bytes) => {
  const offset = bytes.readUInt32LE(10)
  const width = bytes.readInt32LE(18)
  const rawHeight = bytes.readInt32LE(22)
  const bpp = bytes.readUInt16LE(28)
  if (bpp !== 24) throw new Error(`bmp depth ${bpp}`)
  const height = Math.abs(rawHeight)
  const stride = Math.ceil((width * bpp) / 32) * 4
  const rgb = new Uint8Array(width * height * 3)
  Array.from({ length: height }, (_, y) => {
    const src = offset + (rawHeight < 0 ? y : height - 1 - y) * stride
    Array.from({ length: width }, (_, x) => {
      const i = src + x * 3
      const o = (y * width + x) * 3
      rgb[o] = bytes[i + 2]
      rgb[o + 1] = bytes[i + 1]
      rgb[o + 2] = bytes[i]
    })
  })
  return { width, height, rgb }
}

const chunk = (type, data) => {
  const head = Buffer.alloc(8)
  head.writeUInt32BE(data.length, 0)
  head.write(type, 4, "ascii")
  const tail = Buffer.alloc(4)
  tail.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])) >>> 0, 0)
  return Buffer.concat([head, data, tail])
}

const encodePng = ({ width, height, rgb }) => {
  const rows = Array.from({ length: height }, (_, y) =>
    Buffer.concat([Buffer.from([0]), Buffer.from(rgb.subarray(y * width * 3, (y + 1) * width * 3))]),
  )
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(width, 0)
  ihdr.writeUInt32BE(height, 4)
  ihdr.set([8, 2, 0, 0, 0], 8)
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.concat(rows), { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ])
}

// -- Measurement and cleaning ------------------------------------------------

const isGround = (rgb, p, ground, tol) =>
  Math.abs(rgb[p * 3] - ground[0]) <= tol &&
  Math.abs(rgb[p * 3 + 1] - ground[1]) <= tol &&
  Math.abs(rgb[p * 3 + 2] - ground[2]) <= tol

/** Mean colour of the outermost pixel ring: the ground the model painted. */
const sampleGround = ({ width, height, rgb }) => {
  const ring = [
    ...Array.from({ length: width }, (_, x) => [x, 2]),
    ...Array.from({ length: width }, (_, x) => [x, height - 3]),
    ...Array.from({ length: height }, (_, y) => [2, y]),
    ...Array.from({ length: height }, (_, y) => [width - 3, y]),
  ]
  return [0, 1, 2].map((c) =>
    Math.round(ring.reduce((sum, [x, y]) => sum + rgb[(y * width + x) * 3 + c], 0) / ring.length),
  )
}

/** Border-connected ground, flooded to exact mid-grey. Returns the painted mask. */
const cleanGround = (img, tol = 30) => {
  const { width, height, rgb } = img
  const ground = sampleGround(img)
  const seen = new Uint8Array(width * height)
  const stack = new Int32Array(width * height)
  const sp = { n: 0 }
  const push = (x, y) => {
    const p = y * width + x
    if (x < 0 || y < 0 || x >= width || y >= height || seen[p] || !isGround(rgb, p, ground, tol)) return
    seen[p] = 1
    stack[sp.n++] = p
  }
  Array.from({ length: width }, (_, x) => (push(x, 0), push(x, height - 1)))
  Array.from({ length: height }, (_, y) => (push(0, y), push(width - 1, y)))
  while (sp.n > 0) {
    const p = stack[--sp.n]
    rgb[p * 3] = rgb[p * 3 + 1] = rgb[p * 3 + 2] = GROUND
    const x = p % width
    const y = (p - x) / width
    push(x - 1, y)
    push(x + 1, y)
    push(x, y - 1)
    push(x, y + 1)
  }
  return { ground, seen }
}

const bounds = ({ width, height }, seen) =>
  Array.from({ length: height }, (_, y) => y).reduce(
    (box, y) =>
      Array.from({ length: width }, (_, x) => x).reduce(
        (b, x) =>
          seen[y * width + x]
            ? b
            : { x0: Math.min(b.x0, x), y0: Math.min(b.y0, y), x1: Math.max(b.x1, x), y1: Math.max(b.y1, y) },
        box,
      ),
    { x0: width, y0: height, x1: -1, y1: -1 },
  )

const cropWindow = (box, mode, { width, height }) => {
  const cx = (box.x0 + box.x1 + 1) / 2
  const cy = (box.y0 + box.y1 + 1) / 2
  const w = box.x1 - box.x0 + 1
  const h = box.y1 - box.y0 + 1
  const side = Math.round(Math.max(w, h) * 1.04)
  const [cw, ch] = mode === "square" ? [side, side] : [Math.round(w * 1.03), Math.round(h * 1.06)]
  const x = Math.max(0, Math.min(width - cw, Math.round(cx - cw / 2)))
  const y = Math.max(0, Math.min(height - ch, Math.round(cy - ch / 2)))
  return { x, y, w: cw, h: ch }
}

const crop = ({ width, rgb }, { x, y, w, h }) => {
  const out = new Uint8Array(w * h * 3)
  Array.from({ length: h }, (_, row) =>
    out.set(rgb.subarray(((y + row) * width + x) * 3, ((y + row) * width + x + w) * 3), row * w * 3),
  )
  return { width: w, height: h, rgb: out }
}

// -- One sheet ---------------------------------------------------------------

const deliverSheet = (sheet, rawDir, tmp) => {
  const rawPath = join(rawDir, `${sheet.file}.raw.jpg`)
  const bmpPath = join(tmp, `${sheet.file}.bmp`)
  const cleanPath = join(tmp, `${sheet.file}.clean.png`)
  const finalPath = join(outDir, `${sheet.file}.png`)
  return sh("sips", ["-s", "format", "bmp", rawPath, "--out", bmpPath])
    .then(() => readFile(bmpPath))
    .then((bytes) => {
      const img = readBmp(bytes)
      const { ground, seen } = cleanGround(img)
      const box = bounds(img, seen)
      const window = cropWindow(box, sheet.mode, img)
      const cropped = crop(img, window)
      const [dw, dh0] = sheet.deliver
      const dh = dh0 || Math.round((dw * cropped.height) / cropped.width)
      return writeFile(cleanPath, encodePng(cropped)).then(() => ({ ground, box, window, deliver: [dw, dh], source: [img.width, img.height] }))
    })
    .then((info) =>
      sh("sips", ["-z", String(info.deliver[1]), String(info.deliver[0]), cleanPath, "--out", finalPath]).then(() => info),
    )
    .then((info) =>
      Promise.all([readFile(rawPath), readFile(finalPath), copyFile(rawPath, join(outDir, `${sheet.file}.raw.jpg`))]).then(
        ([raw, png]) => ({
          asset: sheet.id,
          file: `${sheet.file}.png`,
          raw_file: `${sheet.file}.raw.jpg`,
          raw_sha256: sha256(raw),
          raw_px: info.source,
          ground_sampled_rgb: info.ground,
          painted_box_px: info.box,
          crop_px: info.window,
          deliver_px: info.deliver,
          file_sha256: sha256(png),
        }),
      ),
    )
}

const [rawArg] = process.argv.slice(2)

if (!rawArg) {
  console.error("usage: node scripts/banner-art.mjs <raw-dir>")
  process.exit(2)
}

mkdir(outDir, { recursive: true })
  .then(() => mkdir(join(tmpdir(), "pt-banner-art"), { recursive: true }))
  .then(() =>
    SHEETS.reduce(
      (chain, sheet) =>
        chain.then((done) =>
          deliverSheet(sheet, resolve(rawArg), join(tmpdir(), "pt-banner-art")).then((record) => [...done, record]),
        ),
      Promise.resolve([]),
    ),
  )
  .then((records) => writeFile(join(outDir, "banner-art.records.json"), `${JSON.stringify(records, null, 2)}\n`).then(() => records))
  .then((records) => records.forEach((r) => console.log(`${r.asset} ${r.deliver_px.join("x")} ${r.file_sha256}`)))
  .catch((err) => {
    console.error(err)
    process.exit(1)
  })
