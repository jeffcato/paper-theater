# Bundled plaque font

Noto Serif Regular (latin subset, woff2) — the profile plaque face. No
provider calls were used; both files are static registry/CDN fetches.

- `noto-serif-latin-400-normal.woff2` — from
  `@fontsource/noto-serif@5.3.0`
  (`https://registry.npmjs.org/@fontsource/noto-serif/-/noto-serif-5.3.0.tgz`,
  `package/files/noto-serif-latin-400-normal.woff2`),
  sha256 `4c0cbe3eec50d260754d681c17ee2af49a43d7fd93ce42877f665fcb1a889b87`.
- `OFL.txt` — SIL Open Font License 1.1 from
  `https://raw.githubusercontent.com/google/fonts/main/ofl/notoserif/OFL.txt`,
  sha256 `cee9892f9f0cc8fe882c9e9537ee6a89621d86ee7ceaf70b02e2b2b1c25c061a`.

Coverage: the latin subset spans U+0000–00FF plus U+2000–206F punctuation,
which covers the full 42-character readiness-v2 read-along set (ASCII plus
U+2019/U+201C/U+201D). The browser harness asserts per-chunk
`document.fonts.check` coverage rather than trusting the subset label.

`noto-serif.css` declares the `@font-face`; hosts import it (bundlers
rewrite the relative URL) or serve `fonts/` statically in harnesses.
