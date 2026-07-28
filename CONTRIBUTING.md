# Contributing

1. Every implementation change must reference a backlog Story ID.
2. Read the related ADR and reference documents before coding.
3. Add a failing test or executable validation before the implementation when practical.
4. Keep changes inside the approved Story scope.
5. Run all quality gates before opening a pull request.
6. Include validation evidence, migration impact, and rollback instructions in the PR.

Branch naming:

- `feature/ENG-xxx-short-name`
- `fix/ENG-xxx-short-name`
- `docs/ENG-xxx-short-name`

Commit examples:

- `feat(command): ENG-021 add transactional dry-run`
- `test(runtime): ENG-034 cover repeated stage unload`
- `docs(adr): ENG-009 approve agent permission model`
