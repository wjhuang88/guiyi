# AI agent operation SOP

For the first-party AI path, Talos owns the generic Agent/session/turn loop and GUIYI owns the game-domain execution boundary. See ADR-0016 and `DIRECTION-GOVERNANCE.md`.

A Talos-driven GUIYI workflow should:

1. Construct the embedded runtime through the published `talos-runtime` SDK rather than Talos implementation crates.
2. Register GUIYI's live capabilities as Talos `AgentTool` adapters, preserving Tool IDs and schemas.
3. Establish the GUIYI domain execution context with caller identity, working set/access scope, permissions, and applicable operation limits.
4. Discover relevant GUIYI Tools from the live catalog rather than a static mega-prompt.
5. Query semantic project state instead of scanning arbitrary files.
6. Dry-run every destructive or broad GUIYI change.
7. Apply one coherent GUIYI transaction.
8. Run GUIYI validation immediately.
9. Let Talos evaluate structured results and request repair or rollback when needed.
10. Run build and preview gates before completion.
11. Produce a final reviewable summary referencing GUIYI transaction IDs and remaining diagnostics.

GUIYI domain permissions remain authoritative even when Talos has separately admitted a Tool call. Content-authoring calls should not receive code-edit, external-process, commit, or publish authority by default.

Direct CLI/Workbench/test clients may perform the same GUIYI Tool operations without Talos, but they must pass through the same permission, working-set, validation, transaction, persistence, and diagnostic boundaries.

Do not add generic prompt, planner, provider, context-compression, memory, retry, conversation, or stopping logic to GUIYI as part of an agent workflow. Those concerns belong to Talos unless an accepted ADR supersedes ADR-0016.
