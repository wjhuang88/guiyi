# Iteration 003 plan

## Goal

Converge GUIYI's product and architecture direction on the published Talos runtime SDK before implementing further AI/agent infrastructure.

## Authorization

- ENG-029 is the only active Story in this direction-convergence slice.
- ENG-014 remains implementation-blocked until ENG-029 is accepted and merged.
- ENG-012 and ENG-013 remain sequenced after an ENG-014 integration baseline.
- Proposed Stories remain non-executable.

## Active Story

### ENG-029 Converge GUIYI AI boundary on Talos and add product-direction governance

GitHub Issue: #50

Branch: `docs/ENG-029-talos-boundary-governance`

Architecture decision: `docs/decisions/ADR-0016-TALOS-AGENT-RUNTIME.md`

Validation record: `docs/validation/ENG-029.md`

## Confirmed external baseline

The current Talos workspace line inspected for this iteration is `0.10.1`.

The Talos repository documents `talos-runtime` as the supported embeddable SDK facade. Its public contract exposes `RuntimeBuilder`, `RuntimeHandle`, `AgentTool`, provider/session-event types, permission/sandbox composition, and a single-direct-Talos-dependency embedding path.

This iteration therefore treats crates.io `talos-runtime` as the default future integration dependency. It does not authorize a Git dependency on Talos `main` or direct use of Talos implementation crates.

## Work packages

### A. Architecture ownership

Deliverables:

- ADR-0016 names Talos as the canonical first-party Agent runtime;
- ADR-0009 is superseded without deleting history;
- GUIYI remains authoritative for game-world semantics and domain execution safety;
- current `AgentLoopDriver`/AgentSession overlap is documented as transitional debt owned by ENG-014.

### B. SDK integration contract

Deliverables:

- registry-first `talos-runtime` dependency policy;
- single-direct-Talos-dependency default;
- `RuntimeBuilder` + `AgentTool` as the intended public SDK extension surface;
- live GUIYI Tool descriptors remain capability/schema authority;
- smallest Talos feature-set rule;
- explicit pre-1.0 version/lockfile upgrade policy.

### C. Product convergence path

Deliverables:

- current sequence is ENG-029 → ENG-014 → ENG-012/013 → ENG-020..023 → ENG-030 → ENG-024;
- ENG-030 becomes the product-level Talos-driven playable-slice proof;
- human workbench remains after the real Agent workflow is credible.

### D. Governance gate

Deliverables:

- mandatory Story ownership test;
- north-star/blocker justification for new platform work;
- scope-correction procedure when work belongs in another layer;
- explicit Talos boundary and dependency rules;
- semantic Tool preference over low-level mutation exposure.

### E. Documentation and closure

Deliverables:

- root README, documentation index, reference docs, ADR register, roadmap, backlog and SOPs agree;
- validation report records exact local/CI evidence;
- Backlog/Issue status changes to Done only after clean-head CI is green.

## Non-goals

- No Rust integration with Talos in ENG-029.
- No dependency addition to Cargo manifests.
- No removal/rename of `engine_agent_host`, `AgentLoopDriver`, or `AgentSession`.
- No Project Graph, Stage command, tactical-domain, or GUI implementation.
- No provider/model configuration inside GUIYI.

## Validation

Documentation/architecture gates:

```bash
python3 scripts/check_docs.py
python3 scripts/validate_repository.py
python3 scripts/check_architecture.py
```

Also verify repository diff hygiene and, through PR CI, the configured repository gates.

## Exit gate

ENG-029 exits only when:

- Issue #50 acceptance criteria are satisfied by current docs;
- ownership and dependency rules are internally consistent;
- `talos-runtime` is the named supported default SDK integration boundary;
- ENG-014 has an implementation-ready objective and explicit non-goals;
- direction governance prevents future generic Agent-runtime drift;
- validation evidence is recorded and clean-head CI is green.
