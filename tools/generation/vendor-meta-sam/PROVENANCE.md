# Provenance: vendored SAM output parser

- Component: official Meta SAM Responses-API output parser, verbatim
  `dist/*.js` (runtime files only, no type definitions).
- Source: npm package `@meta-sam/parser` version 0.1.0, retrieved from the
  public npm registry on 2026-10-27 (registry metadata: homepage
  `https://dev.meta.ai/`, repository
  `github.com/meta-models/meta-sam`, directory
  `typescript/packages/parser`).
- License: SAM License (November 19, 2025), shipped verbatim as LICENSE
  in this directory per its redistribution clause (a copy of the Agreement
  accompanies the materials).
- Transform: none. Files copied byte-identical; see hashes below.
- Use in product: `tools/generation/house-art-mask.mjs` imports
  `./vendor-meta-sam/index.js` to build box/point prompts, parse the
  streamed Responses output, and decode mask rasters. No other product
  code depends on this directory.

## SHA-256 (verify with `sha256sum *` in this directory)

- coco-rle.js `8448c2003f03a5b54314d14f2d10b707052f668a57f951358db7377360a2ccc6`
- errors.js `907a37c8301d5646de6897b0660e980058d96860c4df2489396ae4e08206b00e`
- index.js `0fd7c5aa4f253ef9e8b8c5be1d61816512d3bece514c7bab3050875bf3cee609`
- mask-codec.js `a1ee71bf3186904696ff790658214ba4d501e14deac3a5187975021d9c7475126d2936deeaf403`
- mask-conversion.js `a1a687e6d712043664d91b0bd929938b5037b3d3eb83c61779c918a2489febee`
- object-prompt.js `4449ee5544eb373cdd60ac402cbc63a5187975021d9c7475126d2936deeaf403`
- segmentation.js `c56aada3daf313f0fe05569daf67f7f030d76fad01842a713aa3aa73bc1e4b31`
- stream.js `538b26127b5b5c6e2edb6dcbc4e7be0120cd76aded6419acf3f633609fb51c6c`
- svg-path.js `3a8b1bde1c32ea82352060b50338d282ea00203d4e65f05811cb9a9ec109e27d`
- types.js `dddb798fb4ea8a0bb25da07356ee7481f6872d4e8e6ff342c2c98c034963bfdb`
- LICENSE `4dea99bfaa016e21bc860d73f344236bd1e5c4977d1a9a8fd32f822b500ae1be`
