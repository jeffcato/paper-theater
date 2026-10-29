# Trusted player consumer pinning

INT-06/INT-07 precursor; no service revision change. Runtime adoption, genuine media, publication migration and exact-head CI remain pending until separately recorded. This pinning carries only identity bindings; deserialized JSON and successful validation grant no acceptance, dispatch, publication or media authority.

`TrustedPlayerLoadV1` pins the player-selected profile, the executed checker identity, the exact verified acceptance report bytes, the public catalog entry and the release identity, plus the explicit diagnostic flag the production path keeps false. Checker, report and catalog digests must be real lowercase SHA-256 and must never be zero-hash placeholders. Profile and release identities must be present; no defaults are supplied. No acceptance outcome is expressed here: absent evidence stays blocking by construction, and genuine accepted chapter media plus the INT-06 publication migration remain required before any release loads.

Rust: `TrustedPlayerLoadV1::validate()`. TypeScript: `validateTrustedPlayerLoadV1` with the same field. Both reject placeholder or missing trusted identities with identical codes. These functions verify pinning shape, never that media exists or a checker actually ran.

Five exact-byte Rust/TypeScript cases agree; placeholders stay rejected. The player adopts this interface before genuine chapter media exists; the audience production path keeps `requireAccepted` and never uses diagnostic provenance as acceptance authority.
