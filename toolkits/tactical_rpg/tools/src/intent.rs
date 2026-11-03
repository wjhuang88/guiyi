use crate::navigation::SetNavigationBlockedCommand;
use crate::CreateSpawnCommand;
use guiyi_engine_command::{CommandDescriptor, CommandError, CommandHandler, EngineState};
use guiyi_engine_core::{DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tactical_rpg_content::{CoordinateSpace, HexCoord, StageDocument, StageObjectKind};
use tactical_rpg_validation::validate_stage;

const TOOL_ID: &str = "tactical.make_stage_playable";
const DEFAULT_SPAWN_OBJECT_ID: &str = "spawn.player";
const DEFAULT_SPAWN_PROFILE: &str = "player";
const STAGE_NO_SPAWN_POINT: &str = "STAGE_NO_SPAWN_POINT";
const STAGE_NAV_SPAWN_BLOCKED: &str = "STAGE_NAV_SPAWN_BLOCKED";
const PLAYABILITY_PLAN_FAILED: &str = "TACTICAL_STAGE_PLAYABILITY_PLAN_FAILED";

#[derive(Debug, Clone, Deserialize)]
struct MakeStagePlayableInput {
    stage_id: DocumentId,
    #[serde(default)]
    spawn_object_id: Option<ObjectId>,
    #[serde(default)]
    spawn_profile: Option<String>,
}

impl MakeStagePlayableInput {
    fn spawn_object_id(&self) -> ObjectId {
        self.spawn_object_id
            .clone()
            .unwrap_or_else(|| ObjectId::from_static(DEFAULT_SPAWN_OBJECT_ID))
    }

    fn spawn_profile(&self) -> String {
        self.spawn_profile
            .clone()
            .unwrap_or_else(|| DEFAULT_SPAWN_PROFILE.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlannedAction {
    SetNavigationBlocked {
        stage_id: DocumentId,
        q: i32,
        r: i32,
    },
    CreateSpawn {
        stage_id: DocumentId,
        object_id: ObjectId,
        profile: String,
        q: i32,
        r: i32,
    },
}

impl PlannedAction {
    fn input(&self) -> Value {
        match self {
            Self::SetNavigationBlocked { stage_id, q, r } => json!({
                "stage_id": stage_id,
                "q": q,
                "r": r,
                "blocked": false
            }),
            Self::CreateSpawn {
                stage_id,
                object_id,
                profile,
                q,
                r,
            } => json!({
                "stage_id": stage_id,
                "object_id": object_id,
                "profile": profile,
                "q": q,
                "r": r
            }),
        }
    }

    fn command_id(&self) -> &'static str {
        match self {
            Self::SetNavigationBlocked { .. } => "stage.set_navigation_blocked",
            Self::CreateSpawn { .. } => "stage.create_spawn",
        }
    }

    fn reason_codes(&self) -> &'static [&'static str] {
        match self {
            Self::SetNavigationBlocked { .. } => &[STAGE_NAV_SPAWN_BLOCKED],
            Self::CreateSpawn { .. } => &[STAGE_NO_SPAWN_POINT],
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "command": self.command_id(),
            "input": self.input(),
            "reason_codes": self.reason_codes()
        })
    }

    fn apply(&self, state: &mut EngineState) -> Result<(), CommandError> {
        match self {
            Self::SetNavigationBlocked { .. } => {
                SetNavigationBlockedCommand.apply(&self.input(), state)?;
            }
            Self::CreateSpawn { .. } => {
                CreateSpawnCommand.apply(&self.input(), state)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct PlayabilityPlan {
    stage_id: DocumentId,
    playable_before: bool,
    playable_after: bool,
    diagnostics_before: DiagnosticBag,
    diagnostics_after: DiagnosticBag,
    actions: Vec<PlannedAction>,
}

pub(crate) struct MakeStagePlayableCommand;

impl CommandHandler for MakeStagePlayableCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static(TOOL_ID),
            title: "Make tactical Stage playable".into(),
            description: "Plan and atomically apply bounded semantic Stage repairs so the existing Stage validator reports no errors and a standard preview spawn exists.".into(),
            input_schema: Value::Null,
            output_schema: json!({
                "type": "object",
                "required": [
                    "stage_id",
                    "playable_before",
                    "playable_after",
                    "plan",
                    "diagnostics_before",
                    "diagnostics_after"
                ],
                "properties": {
                    "stage_id": {"type": "string"},
                    "playable_before": {"type": "boolean"},
                    "playable_after": {"type": "boolean"},
                    "plan": {"type": "array"},
                    "diagnostics_before": {"type": "array"},
                    "diagnostics_after": {"type": "array"}
                }
            }),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![
                ToolId::from_static("stage.validate"),
                ToolId::from_static("stage.set_navigation_blocked"),
                ToolId::from_static("stage.create_spawn"),
            ],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("stage_id", SchemaNode::string()),
            FieldSchema::optional(
                "spawn_object_id",
                SchemaNode::string().with_default(json!(DEFAULT_SPAWN_OBJECT_ID)),
            ),
            FieldSchema::optional(
                "spawn_profile",
                SchemaNode::string()
                    .with_min_length(1)
                    .with_default(json!(DEFAULT_SPAWN_PROFILE)),
            ),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: MakeStagePlayableInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.stage_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let Ok(input) = serde_json::from_value::<MakeStagePlayableInput>(input.clone()) else {
            return DiagnosticBag::default();
        };
        match derive_plan(&input, state) {
            Ok(plan) => plan.diagnostics_after,
            Err(error) => plan_failure(&input.stage_id, error.to_string()),
        }
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: MakeStagePlayableInput = serde_json::from_value(input.clone())?;
        let plan = derive_plan(&input, state)?;
        if !plan.playable_after {
            return Err(CommandError::ValidationFailed(plan.diagnostics_after));
        }

        for action in &plan.actions {
            action.apply(state)?;
        }

        let final_stage = read_stage(state, &plan.stage_id)?;
        let final_diagnostics = validate_stage(&final_stage);
        if !is_locally_playable(&final_diagnostics) {
            return Err(CommandError::ValidationFailed(final_diagnostics));
        }

        Ok(json!({
            "stage_id": plan.stage_id,
            "playable_before": plan.playable_before,
            "playable_after": true,
            "plan": plan.actions.iter().map(PlannedAction::as_json).collect::<Vec<_>>(),
            "diagnostics_before": plan.diagnostics_before.diagnostics,
            "diagnostics_after": final_diagnostics.diagnostics
        }))
    }
}

fn derive_plan(
    input: &MakeStagePlayableInput,
    state: &EngineState,
) -> Result<PlayabilityPlan, CommandError> {
    let stage = read_stage(state, &input.stage_id)?;
    let diagnostics_before = validate_stage(&stage);
    let playable_before = is_locally_playable(&diagnostics_before);
    if playable_before {
        return Ok(PlayabilityPlan {
            stage_id: input.stage_id.clone(),
            playable_before: true,
            playable_after: true,
            diagnostics_after: diagnostics_before.clone(),
            diagnostics_before,
            actions: Vec::new(),
        });
    }

    let mut working = state.clone();
    let mut actions = Vec::new();

    let blocked = stage
        .navigation
        .blocked
        .iter()
        .map(|cell| (cell.q, cell.r))
        .collect::<BTreeSet<_>>();
    let blocked_spawn_cells = stage
        .objects
        .iter()
        .filter(|object| matches!(&object.object, StageObjectKind::SpawnPoint { .. }))
        .filter(|object| blocked.contains(&(object.position.q, object.position.r)))
        .map(|object| (object.position.q, object.position.r))
        .collect::<BTreeSet<_>>();

    for (q, r) in blocked_spawn_cells {
        let action = PlannedAction::SetNavigationBlocked {
            stage_id: input.stage_id.clone(),
            q,
            r,
        };
        action.apply(&mut working)?;
        actions.push(action);
    }

    let stage_after_unblocking = read_stage(&working, &input.stage_id)?;
    let has_spawn = stage_after_unblocking
        .objects
        .iter()
        .any(|object| matches!(&object.object, StageObjectKind::SpawnPoint { .. }));

    if !has_spawn {
        let spawn_object_id = input.spawn_object_id();
        let spawn_profile = input.spawn_profile();
        let Some((width, height)) = grid_dimensions(&stage_after_unblocking.coordinate_space) else {
            let mut diagnostics_after = validate_stage(&stage_after_unblocking);
            diagnostics_after.extend(plan_failure(
                &input.stage_id,
                "automatic missing-spawn repair requires a HexAxial or Square Stage",
            ));
            return Ok(PlayabilityPlan {
                stage_id: input.stage_id.clone(),
                playable_before,
                playable_after: false,
                diagnostics_before,
                diagnostics_after,
                actions,
            });
        };

        let blocked = stage_after_unblocking
            .navigation
            .blocked
            .iter()
            .map(|cell| (cell.q, cell.r))
            .collect::<BTreeSet<_>>();
        let mut first_candidate_diagnostics = None;
        let mut selected = None;

        'coordinates: for q in 0..width as i32 {
            for r in 0..height as i32 {
                if blocked.contains(&(q, r)) {
                    continue;
                }
                let action = PlannedAction::CreateSpawn {
                    stage_id: input.stage_id.clone(),
                    object_id: spawn_object_id.clone(),
                    profile: spawn_profile.clone(),
                    q,
                    r,
                };
                let mut candidate = working.clone();
                match action.apply(&mut candidate) {
                    Ok(()) => {
                        let candidate_stage = read_stage(&candidate, &input.stage_id)?;
                        let diagnostics = validate_stage(&candidate_stage);
                        if is_locally_playable(&diagnostics) {
                            selected = Some((action, candidate));
                            break 'coordinates;
                        }
                        if first_candidate_diagnostics.is_none() {
                            first_candidate_diagnostics = Some(diagnostics);
                        }
                    }
                    Err(CommandError::ValidationFailed(diagnostics)) => {
                        if first_candidate_diagnostics.is_none() {
                            first_candidate_diagnostics = Some(diagnostics);
                        }
                    }
                    Err(error) => {
                        let mut diagnostics_after = validate_stage(&stage_after_unblocking);
                        diagnostics_after.extend(plan_failure(&input.stage_id, error.to_string()));
                        return Ok(PlayabilityPlan {
                            stage_id: input.stage_id.clone(),
                            playable_before,
                            playable_after: false,
                            diagnostics_before,
                            diagnostics_after,
                            actions,
                        });
                    }
                }
            }
        }

        if let Some((action, candidate)) = selected {
            working = candidate;
            actions.push(action);
        } else {
            let mut diagnostics_after = first_candidate_diagnostics
                .unwrap_or_else(|| validate_stage(&stage_after_unblocking));
            if !diagnostics_after.has_errors() {
                diagnostics_after.extend(plan_failure(
                    &input.stage_id,
                    "no deterministic unblocked fallback spawn coordinate produces a locally playable Stage",
                ));
            }
            return Ok(PlayabilityPlan {
                stage_id: input.stage_id.clone(),
                playable_before,
                playable_after: false,
                diagnostics_before,
                diagnostics_after,
                actions,
            });
        }
    }

    let final_stage = read_stage(&working, &input.stage_id)?;
    let mut diagnostics_after = validate_stage(&final_stage);
    let playable_after = is_locally_playable(&diagnostics_after);
    if !playable_after && !diagnostics_after.has_errors() {
        diagnostics_after.extend(plan_failure(
            &input.stage_id,
            "bounded Stage repairs did not satisfy the local-playability contract",
        ));
    }

    Ok(PlayabilityPlan {
        stage_id: input.stage_id.clone(),
        playable_before,
        playable_after,
        diagnostics_before,
        diagnostics_after,
        actions,
    })
}

fn read_stage(state: &EngineState, stage_id: &DocumentId) -> Result<StageDocument, CommandError> {
    let envelope = state.documents.get(stage_id)?.clone();
    Ok(StageDocument::from_envelope(&envelope)?)
}

fn grid_dimensions(space: &CoordinateSpace) -> Option<(u32, u32)> {
    match space {
        CoordinateSpace::HexAxial { width, height } | CoordinateSpace::Square { width, height } => {
            Some((*width, *height))
        }
        CoordinateSpace::Free2d | CoordinateSpace::Free3d => None,
    }
}

fn is_locally_playable(diagnostics: &DiagnosticBag) -> bool {
    !diagnostics.has_errors()
        && !diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == STAGE_NO_SPAWN_POINT)
}

fn plan_failure(stage_id: &DocumentId, message: impl Into<String>) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    bag.push(
        Diagnostic::error(PLAYABILITY_PLAN_FAILED, message)
            .at_document(stage_id.clone()),
    );
    bag
}

#[cfg(test)]
mod tests {
    use super::*;
    use guiyi_engine_command::{CommandContext, CommandExecutor, CommandRegistry, CommandRequest};
    use guiyi_engine_core::PermissionSet;
    use tactical_rpg_content::{StageNavigation, StageObject};

    fn state_with(stage: StageDocument) -> EngineState {
        let mut state = EngineState::default();
        state.documents.insert(stage.to_envelope().unwrap()).unwrap();
        state
    }

    fn execute(stage: StageDocument, dry_run: bool) -> (EngineState, guiyi_engine_command::TransactionReport) {
        let mut registry = CommandRegistry::default();
        crate::register_tactical_commands(&mut registry).unwrap();
        let mut executor = CommandExecutor::new(registry);
        let mut state = state_with(stage);
        let report = executor
            .execute(
                CommandRequest {
                    command: ToolId::from_static(TOOL_ID),
                    input: json!({"stage_id": "stage.intent"}),
                    dry_run,
                },
                &CommandContext {
                    actor: "eng023-unit".into(),
                    permissions: PermissionSet::content_author(),
                },
                &mut state,
            )
            .unwrap();
        (state, report)
    }

    #[test]
    fn deterministic_missing_spawn_uses_first_unblocked_coordinate() {
        let mut stage = StageDocument::new_hex(
            DocumentId::from_static("stage.intent"),
            "Intent",
            2,
            2,
        );
        stage.navigation = StageNavigation {
            blocked: vec![HexCoord::new(0, 0)],
        };

        let (_, report) = execute(stage, true);
        assert_eq!(report.output["playable_before"], false);
        assert_eq!(report.output["playable_after"], true);
        assert_eq!(report.output["plan"][0]["command"], "stage.create_spawn");
        assert_eq!(report.output["plan"][0]["input"]["q"], 0);
        assert_eq!(report.output["plan"][0]["input"]["r"], 1);
        assert_eq!(report.output["plan"][0]["input"]["object_id"], DEFAULT_SPAWN_OBJECT_ID);
        assert_eq!(report.output["plan"][0]["input"]["profile"], DEFAULT_SPAWN_PROFILE);
    }

    #[test]
    fn multiple_blocked_spawn_repairs_are_sorted_and_use_primitive_command_inputs() {
        let mut stage = StageDocument::new_hex(
            DocumentId::from_static("stage.intent"),
            "Intent",
            3,
            3,
        );
        stage.navigation = StageNavigation {
            blocked: vec![HexCoord::new(2, 1), HexCoord::new(0, 2)],
        };
        for (id, q, r) in [("spawn.b", 2, 1), ("spawn.a", 0, 2)] {
            stage.objects.push(StageObject {
                id: ObjectId::new(id).unwrap(),
                position: HexCoord::new(q, r),
                object: StageObjectKind::SpawnPoint {
                    profile: "player".into(),
                },
                properties: json!({}),
            });
        }

        let (_, report) = execute(stage, true);
        assert_eq!(report.output["plan"].as_array().unwrap().len(), 2);
        assert_eq!(report.output["plan"][0]["input"]["q"], 0);
        assert_eq!(report.output["plan"][0]["input"]["r"], 2);
        assert_eq!(report.output["plan"][1]["input"]["q"], 2);
        assert_eq!(report.output["plan"][1]["input"]["r"], 1);
        assert_eq!(
            report.output["plan"][0]["reason_codes"],
            json!([STAGE_NAV_SPAWN_BLOCKED])
        );
    }
}
