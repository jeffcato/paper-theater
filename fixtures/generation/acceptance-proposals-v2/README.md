# I17 matrix/checker adoption examples

Separately versioned G20-shaped diagnostic metadata adopts `pt-acceptance-required-v2` and the current pinned checker identity. The three/37/37 required IDs and five scoped/custom/adverse cases are unchanged. Tests compare the frozen v1 rows through the archived accessor and compare current StageExecutor behavior to this v2 fixture. Original `acceptance-proposals-v1` bytes stay unchanged.

```sh
PT_ACCEPTANCE_EXPORT=fixtures/generation/acceptance-proposals-v2 cargo test -p pt-production generation::tests::acceptance_proposals:: --offline
```

Private proposal/context metadata remains schema 1. Actual synthetic recipe/player bytes and input reconstruction are defined by the existing test fixture; no autonomous production evidence or runtime policy is generated. Source coverage's new independent implementation does not close other semantic verifiers or NeedsRepair. Integration acknowledged its behavioral compatibility test replacement before generation removed the inert literal.
