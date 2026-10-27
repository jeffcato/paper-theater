#!/usr/bin/env python3
"""Apply a SAM binary mask to a PNG (stdlib only: zlib PNG codec).

Usage:
  sam_mask_apply.py <src.png> <mask.json> <out.png>
  sam_mask_apply.py --self-test

mask.json: {"height": H, "width": W, "raster": [0/1, ...]} row-major.
The mask is nearest-neighbour upscaled to the source size and applied as
the alpha channel; RGB is passed through untouched.

Prints a JSON stats object: size, mask coverage, near-#808080 grey kept
fraction, near-white removed fraction, and agreement of the mask with a
local not-white map (independent cross-check).
"""
import json
import struct
import sys
import zlib


def _chunks(data):
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not PNG"
    pos, out = 8, []
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        typ = data[pos + 4:pos + 8]
        out.append((typ, data[pos + 8:pos + 8 + length]))
        pos += 12 + length
    return out


def _paeth(a, b, c):
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    return b if pb <= pc else c


def read_png(path):
    raw = open(path, "rb").read()
    w = h = None
    ctype = bdepth = inter = None
    idat = b""
    for typ, data in _chunks(raw):
        if typ == b"IHDR":
            w, h, bdepth, ctype, _, _, inter = struct.unpack(">IIBBBBB", data)
        elif typ == b"IDAT":
            idat += data
    assert bdepth == 8 and inter == 0, f"unsupported PNG (depth={bdepth}, interlace={inter})"
    assert ctype in (2, 6), f"unsupported color type {ctype}"
    ch = 3 if ctype == 2 else 4
    stream = zlib.decompress(idat)
    stride = w * ch
    rows, prev, pos = [], bytearray(stride), 0
    for _ in range(h):
        f = stream[pos]
        pos += 1
        cur = bytearray(stream[pos:pos + stride])
        pos += stride
        if f == 1:
            for i in range(ch, stride):
                cur[i] = (cur[i] + cur[i - ch]) & 0xFF
        elif f == 2:
            for i in range(stride):
                cur[i] = (cur[i] + prev[i]) & 0xFF
        elif f == 3:
            for i in range(stride):
                a = cur[i - ch] if i >= ch else 0
                cur[i] = (cur[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif f == 4:
            for i in range(stride):
                a = cur[i - ch] if i >= ch else 0
                b = prev[i]
                c = prev[i - ch] if i >= ch else 0
                cur[i] = (cur[i] + _paeth(a, b, c)) & 0xFF
        elif f != 0:
            raise AssertionError(f"bad filter {f}")
        rows.append(cur)
        prev = cur
    px = []
    for cur in rows:
        row = []
        for x in range(w):
            r, g, b = cur[3 * x:3 * x + 3] if ch == 3 else cur[4 * x:4 * x + 3]
            a = 255 if ch == 3 else cur[4 * x + 3]
            row.append((r, g, b, a))
        px.append(row)
    return w, h, px


def write_png_rgba(path, w, h, px):
    raw = bytearray()
    for row in px:
        raw.append(0)
        for (r, g, b, a) in row:
            raw += bytes((r, g, b, a))

    def chunk(typ, data):
        c = struct.pack(">I", len(data)) + typ + data
        return c + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)

    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(bytes(raw)))
           + chunk(b"IEND", b""))
    open(path, "wb").write(png)


def is_grey(r, g, b):
    return all(abs(c - 128) <= 14 for c in (r, g, b)) and max(r, g, b) - min(r, g, b) <= 10


def is_white(r, g, b):
    return min(r, g, b) >= 240


def apply(src, mask, out):
    w, h, px = read_png(src)
    m = json.load(open(mask))
    mw, mh, rast = m["width"], m["height"], m["raster"]
    assert len(rast) == mw * mh, "raster size mismatch"
    kept = [[0] * w for _ in range(h)]
    outpx = []
    for y in range(h):
        row = []
        for x in range(w):
            bit = rast[(y * mh // h) * mw + (x * mw // w)]
            kept[y][x] = 1 if bit else 0
            r, g, b, _ = px[y][x]
            row.append((r, g, b, 255 if bit else 0))
        outpx.append(row)
    write_png_rgba(out, w, h, outpx)
    grey = grey_kept = white = white_gone = agree = 0
    for y in range(h):
        for x in range(w):
            r, g, b, _ = px[y][x]
            g8, wh = is_grey(r, g, b), is_white(r, g, b)
            grey += g8
            grey_kept += g8 and kept[y][x]
            white += wh
            white_gone += wh and not kept[y][x]
            agree += kept[y][x] == (0 if wh else 1)
    n = w * h
    stats = {
        "size": [w, h], "mask_hw": [mh, mw],
        "coverage_frac": round(sum(map(sum, kept)) / n, 4),
        "grey_total": grey, "grey_kept_frac": round(grey_kept / grey, 4) if grey else 1.0,
        "white_total": white, "white_removed_frac": round(white_gone / white, 4) if white else 1.0,
        "notwhite_agreement_frac": round(agree / n, 4),
    }
    print(json.dumps(stats, indent=2))
    return stats


def flood_cut(src, out, threshold=250):
    """Cut border-connected near-white background (stdlib only).

    Flood-fills from every border pixel through pixels with min channel >=
    threshold (8-connected). Only canvas-connected white goes transparent;
    enclosed fills (apertures, card, holes) and all RGB are untouched.
    Prints a stats JSON; returns (stats, alpha_map).
    """
    from collections import deque
    w, h, px = read_png(src)
    isbg = bytearray(w * h)
    for y in range(h):
        base = y * w
        for x in range(w):
            r, g, b, _ = px[y][x]
            if min(r, g, b) >= threshold:
                isbg[base + x] = 1
    seen = bytearray(w * h)
    queue = deque()
    for x in range(w):
        for y in (0, h - 1):
            if isbg[y * w + x]:
                seen[y * w + x] = 1
                queue.append(y * w + x)
    for y in range(h):
        for x in (0, w - 1):
            if isbg[y * w + x] and not seen[y * w + x]:
                seen[y * w + x] = 1
                queue.append(y * w + x)
    while queue:
        i = queue.popleft()
        x, y = i % w, i // w
        for dy in (-1, 0, 1):
            ny = y + dy
            if ny < 0 or ny >= h:
                continue
            for dx in (-1, 0, 1):
                if dx == 0 and dy == 0:
                    continue
                nx = x + dx
                if nx < 0 or nx >= w:
                    continue
                j = ny * w + nx
                if isbg[j] and not seen[j]:
                    seen[j] = 1
                    queue.append(j)
    outpx = []
    grey = grey_kept = white = white_gone = 0
    for y in range(h):
        row = []
        for x in range(w):
            r, g, b, _ = px[y][x]
            cut = seen[y * w + x]
            row.append((r, g, b, 0 if cut else 255))
            if is_grey(r, g, b):
                grey += 1
                grey_kept += not cut
            if isbg[y * w + x]:
                white += 1
                white_gone += cut
        outpx.append(row)
    write_png_rgba(out, w, h, outpx)
    n = w * h
    stats = {
        "method": "border-flood", "threshold": threshold,
        "size": [w, h],
        "coverage_frac": round(sum(seen) / n, 4),
        "grey_total": grey, "grey_kept_frac": round(grey_kept / grey, 4) if grey else 1.0,
        "white_total": white, "white_removed_frac": round(white_gone / white, 4) if white else 1.0,
    }
    print(json.dumps(stats, indent=2))
    return stats, seen


def flood_self_test():
    import tempfile
    from pathlib import Path
    W, H = 24, 16
    DARK, GREY, CARD, ISO = (30, 20, 15), (128, 128, 128), (245, 224, 187), (252, 252, 252)
    WHITE = (255, 255, 255)
    grid = [[DARK] * W for _ in range(H)]
    for y in range(H):
        for x in range(W):
            if x < 2 or y < 2 or x >= W - 2 or y >= H - 2:
                grid[y][x] = WHITE  # connected border
    for y in range(6, 9):
        for x in range(10, 13):
            grid[y][x] = WHITE  # enclosed hole: must survive
    for y in range(4, 6):
        for x in range(4, 8):
            grid[y][x] = GREY
    for y in range(10, 13):
        for x in range(16, 21):
            grid[y][x] = CARD
    grid[12][6] = ISO  # isolated near-white, unconnected: survives
    with tempfile.TemporaryDirectory() as td:
        src = str(Path(td) / "s.png")
        write_png_rgba(src, W, H, [[c + (255,) for c in row] for row in grid])
        out = str(Path(td) / "o.png")
        stats, seen = flood_cut(src, out)
        _, _, ox = read_png(out)
        assert ox[0][0][3] == 0 and ox[1][1][3] == 0, "border cut"
        assert ox[7][11][3] == 255, "enclosed hole kept"
        assert ox[4][5][3] == 255 and ox[11][17][3] == 255, "grey/card kept"
        assert ox[12][6][3] == 255, "isolated near-white kept"
        assert stats["grey_kept_frac"] == 1.0, stats
        # border white cells: total white minus 9 hole minus 1 isolated
        assert stats["white_removed_frac"] == round(
            (stats["white_total"] - 10) / stats["white_total"], 4), stats
    print("FLOOD-SELF-TEST OK")


def self_test():
    import tempfile
    from pathlib import Path
    with tempfile.TemporaryDirectory() as td:
        src = str(Path(td) / "s.png")
        write_png_rgba(src, 8, 6, [[(10 * x + y, 20 * x, 30, 255) for x in range(8)] for y in range(6)])
        w, h, px = read_png(src)
        assert (w, h) == (8, 6) and px[2][3][:3] == (32, 60, 30), "codec round-trip"
        m = str(Path(td) / "m.json")
        json.dump({"height": 6, "width": 8, "raster": [1] * 48}, open(m, "w"))
        s = apply(src, m, str(Path(td) / "o.png"))
        assert s["coverage_frac"] == 1.0 and s["grey_kept_frac"] == 1.0, s
        json.dump({"height": 3, "width": 4, "raster": [1, 0, 0, 1] * 3}, open(m, "w"))
        _, _, qx = read_png(src)
        s = apply(src, m, str(Path(td) / "o2.png"))
        _, _, ox = read_png(str(Path(td) / "o2.png"))
        assert ox[0][0][3] == 255 and ox[0][2][3] == 0 and ox[0][6][3] == 255, "alpha upscale/apply"
        assert all(ox[y][x][:3] == qx[y][x][:3] for y in range(6) for x in range(8)), "RGB passthrough"
    print("SELF-TEST OK")


if __name__ == "__main__":
    if sys.argv[1:2] == ["--self-test"]:
        self_test()
    elif sys.argv[1:2] == ["--flood-self-test"]:
        flood_self_test()
    elif sys.argv[1:2] == ["--flood-cut"]:
        flood_cut(sys.argv[2], sys.argv[3],
                  int(sys.argv[4]) if len(sys.argv) > 4 else 250)
    else:
        apply(sys.argv[1], sys.argv[2], sys.argv[3])
