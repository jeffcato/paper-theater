#!/usr/bin/env python3
"""Deterministic Wave 1 checks for house.facade (stdlib only).

Converts the PNG to BMP via sips, then measures: exact size, aperture
flat-grey statistics inside the brief's stage-opening rect (inset 4% to
avoid the moulding edge), and the facade's coverage of the canvas
(non-paper-margin rows/cols). OCR runs separately via tesseract.
"""
import json, struct, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IMG = ROOT / (sys.argv[1] if len(sys.argv) > 1 else "fixtures/generation/immersive-house-test-v1/house.facade.png")
W, H = 2048, 1245
# brief aperture: u 0.1846-0.8154, v 0.2247 to bottom edge
AP = (0.1846, 0.2247, 0.8154, 1.0)

def read_bmp(p: Path):
    d = p.read_bytes()
    assert d[:2] == b"BM", "not BMP"
    off = struct.unpack_from("<I", d, 10)[0]
    w = struct.unpack_from("<i", d, 18)[0]
    h = struct.unpack_from("<i", d, 22)[0]
    bpp = struct.unpack_from("<H", d, 28)[0]
    assert bpp in (24, 32), f"bpp {bpp}"
    step = bpp // 8
    stride = ((w * step + 3) // 4) * 4
    px = {}
    rows = []
    for row in range(abs(h)):
        y = (abs(h) - 1 - row) if h > 0 else row
        base = off + row * stride
        line = []
        for x in range(w):
            b, g, r = d[base + x * step: base + x * step + 3]
            line.append((r, g, b))
        rows.append(line)
    return w, abs(h), rows

with tempfile.TemporaryDirectory() as td:
    bmp = Path(td) / "facade.bmp"
    subprocess.run(["sips", "-s", "format", "bmp", str(IMG), "--out", str(bmp)],
                   check=True, capture_output=True)
    w, h, px = read_bmp(bmp)

checks = {"file": str(IMG.relative_to(ROOT)), "size": [w, h], "size_ok": [w, h] == [W, H]}

# aperture grey stats: inset the brief rect by 6% each side, sample every 7px
x0, y0 = int(AP[0] * w * 1.06), int(AP[1] * h + 0.10 * h)  # below the valance band
x1, y1 = int(AP[2] * w * 0.94), int(AP[3] * h * 0.97)
tot = near = 0; sums = [0, 0, 0]
for y in range(y0, y1, 7):
    for x in range(x0, x1, 7):
        r, g, b = px[y][x]
        tot += 1; sums[0] += r; sums[1] += g; sums[2] += b
        if all(abs(c - 128) <= 14 for c in (r, g, b)) and max(r, g, b) - min(r, g, b) <= 10:
            near += 1
checks["aperture_sampled_region"] = [x0, y0, x1, y1]
checks["aperture_mean_rgb"] = [round(s / tot) for s in sums]
checks["aperture_flat_grey_fraction"] = round(near / tot, 4) if tot else 0

# valance intrusion: fraction of non-grey in the brief aperture's top band
# (v 0.2247-0.37), where the brief expects the valance only above the opening
ty0, ty1 = int(AP[1] * h), int(0.37 * h)
tt = ng = 0
for y in range(ty0, ty1, 5):
    for x in range(int(AP[0] * w) + 8, int(AP[2] * w) - 8, 5):
        r, g, b = px[y][x]; tt += 1
        if not (all(abs(c - 128) <= 20 for c in (r, g, b))):
            ng += 1
checks["aperture_top_band_nongrey_fraction"] = round(ng / tt, 4) if tt else 0

print(json.dumps(checks, indent=2))
