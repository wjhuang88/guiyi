# Development SOP

Before implementation, read and apply [Product direction governance](DIRECTION-GOVERNANCE.md).

1. Select a Ready Story from the product backlog.
2. Confirm the Story passed the ownership test and names its primary layer: Talos runtime, GUIYI engine platform, GUIYI toolkit, game repository, or human client.
3. Confirm the Story either advances the current Talos-driven production north star or closes a mandatory safety/reliability/compatibility gap.
4. Read related reference and ADR files.
5. Run baseline gates.
6. Add a failing test or reproducible validation.
7. Implement the smallest vertical change.
8. Run local gates after every acceptance point.
9. Update machine-readable diagnostics and Tool descriptors.
10. Update reference documentation.
11. Produce a validation report and rollback notes.
12. Merge only after all gates pass.

No implementation task may begin without a Story ID. Scope discoveries become new backlog items rather than silent expansion.

For AI integration work, the default dependency and ownership rule is ADR-0016: use the published `talos-runtime` SDK at an outer adapter boundary; do not grow generic model/provider/planning/memory/conversation/retry logic inside GUIYI.

When implementation reveals that scope belongs to another layer, stop expanding the patch and apply the scope-correction procedure in `DIRECTION-GOVERNANCE.md`.
