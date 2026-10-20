# Iteration SOP

Use one-week iterations with one primary outcome and one to three primary Stories.

Before Day 0, every candidate Story must pass `DIRECTION-GOVERNANCE.md`: ownership layer is explicit, dependency direction is valid, and the Story advances the current north star or closes a mandatory safety/reliability/compatibility blocker.

- Day 0: scope, ownership, dependencies, baseline, demo definition, explicit non-goals.
- Day 1: ADR/interface/test fixture and failing test or reproducible validation.
- Days 2–3: implementation and local gates.
- Day 4: integration, failure paths, lifecycle repetition, docs.
- Day 5: full gates, demo, validation report, retrospective.

Do not fill an iteration with unrelated foundation work merely to keep implementation busy. If the primary product sequence is blocked, the iteration should either remove that blocker or explicitly change the Backlog/roadmap with evidence.

For AI integration iterations, ADR-0016 is the default boundary: generic Agent runtime behavior belongs in Talos and the published `talos-runtime` SDK is the normal embedding surface.

A Story is Done only when acceptance criteria, tests, documentation, migration/rollback notes, and required validation evidence are complete.
