R17 source handoff for integration I20, frozen before R18 edits. [Manifest](handoff.json).

The read-only content-addressed tar contains 51 source/document/evidence files and its internal SHA-256 inventory. It preserves the completed provider-service handoff, 165 scoped passing checks and both aggregate Pilot failure logs. It makes no aggregate-green claim. The recorded server binary still matches its original hash.

The integration-owned R17 task document changed after that handoff; its historical/current hashes are recorded and its replacement is excluded. All available recorded implementation/dependency pins match. R18 changes are outside this immutable checkpoint. No Git operations or staging permission are implied.
