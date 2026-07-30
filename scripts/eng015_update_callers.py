from pathlib import Path


def replace(path: str, old: str, new: str, expected: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != expected:
        raise SystemExit(f"{path}: expected {expected} occurrences, found {count}")
    target.write_text(text.replace(old, new))


replace(
    "crates/engine_cli/src/main.rs",
    "    let catalog = ToolCatalog::from_registries(&commands, &queries);",
    "    let catalog = ToolCatalog::from_registries(&commands, &queries)\n        .map_err(|error| error.to_string())?;",
)

editor = Path("crates/engine_editor/src/main.rs")
text = editor.read_text()
old = "    let catalog = ToolCatalog::from_registries(&commands, &queries);"
if text.count(old) != 2:
    raise SystemExit(f"engine_editor: expected 2 catalog call sites, found {text.count(old)}")
text = text.replace(
    old,
    "    let catalog = ToolCatalog::from_registries(&commands, &queries)\n        .map_err(|error| error.to_string())?;",
    1,
)
text = text.replace(
    old,
    "        let catalog = ToolCatalog::from_registries(&commands, &queries)\n            .expect(\"built-in tool IDs are unique\");",
    1,
)
editor.write_text(text)

replace(
    "crates/engine_agent_host/src/lib.rs",
    "        let catalog = ToolCatalog::from_registries(&command_registry, &query_registry);",
    "        let catalog = ToolCatalog::from_registries(&command_registry, &query_registry)\n            .expect(\"built-in tool IDs are unique\");",
)
replace(
    "crates/engine_agent_host/src/lib.rs",
    "        let catalog = ToolCatalog::from_registries(&commands, &queries);",
    "        let catalog = ToolCatalog::from_registries(&commands, &queries)\n            .expect(\"test tool IDs are unique\");",
)
replace(
    "examples/mock_agent_workflow/src/main.rs",
    "    let catalog = ToolCatalog::from_registries(&commands, &queries);",
    "    let catalog = ToolCatalog::from_registries(&commands, &queries)\n        .expect(\"example tool IDs are unique\");",
)
