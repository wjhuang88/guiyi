use guiyi_engine_command::{CommandDescriptor, CommandError, CommandHandler, EngineState};
use guiyi_engine_content::DocumentStore;
use guiyi_engine_core::{
    DocumentAccessPlan, DocumentId, ObjectId, Permission, PermissionSet, ToolId,
};
use guiyi_engine_query::{QueryDescriptor, QueryError, QueryHandler};
use guiyi_engine_schema::{FieldSchema, SchemaNode};
use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use serde::Deserialize;
use serde_json::{json, Value};
use tactical_rpg_content::{
    shortest_path, stage_position_in_bounds, CoordinateSpace, HexCoord, NavigationError,
    StageDocument,
};
use tactical_rpg_validation::validate_stage;

pub(crate) const STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE: &str =
    "STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE";
pub(crate) const STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS: &str =
    "STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS";
pub(crate) const STAGE_NAV_ENDPOINT_OUT_OF_BOUNDS: &str = "STAGE_NAV_ENDPOINT_OUT_OF_BOUNDS";
pub(crate) const STAGE_OBJECT_NOT_FOUND: &str = "STAGE_OBJECT_NOT_FOUND";

#[derive(Debug, Deserialize)]
struct SetNavigationBlockedInput {
    stage_id: DocumentId,
    q: i32,
    r: i32,
    blocked: bool,
}

pub(crate) struct SetNavigationBlockedCommand;

impl CommandHandler for SetNavigationBlockedCommand {
    fn descriptor(&self) -> CommandDescriptor {
        CommandDescriptor {
            id: ToolId::from_static("stage.set_navigation_blocked"),
            title: "Set Stage navigation blocked cell".into(),
            description: "Block or unblock one grid cell in Stage navigation authoring data.".into(),
            input_schema: Value::Null,
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::EditContent]),
            side_effects: vec!["modifies_document".into()],
            related_tools: vec![
                ToolId::from_static("stage.reachability"),
                ToolId::from_static("stage.validate"),
            ],
        }
    }

    fn input_schema(&self) -> SchemaNode {
        SchemaNode::object(vec![
            FieldSchema::required("stage_id", SchemaNode::string()),
            FieldSchema::required("q", SchemaNode::integer()),
            FieldSchema::required("r", SchemaNode::integer()),
            FieldSchema::required("blocked", SchemaNode::boolean()),
        ])
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, CommandError> {
        let input: SetNavigationBlockedInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.stage_id))
    }

    fn validate(&self, input: &Value, state: &EngineState) -> DiagnosticBag {
        let mut bag = DiagnosticBag::default();
        let Ok(input) = serde_json::from_value::<SetNavigationBlockedInput>(input.clone()) else {
            return bag;
        };
        let Some(mut stage) = stage_for_validation(state, &input.stage_id) else {
            return bag;
        };
        if !grid_navigation_supported(&stage.coordinate_space) {
            bag.push(unsupported_coordinate_space(&input.stage_id));
            return bag;
        }

        let cell = HexCoord::new(input.q, input.r);
        if !stage_position_in_bounds(&stage.coordinate_space, cell) {
            bag.push(blocked_cell_out_of_bounds(&input.stage_id, cell));
            return bag;
        }

        set_blocked(&mut stage, cell, input.blocked);
        bag.extend(validate_stage(&stage));
        bag
    }

    fn apply(&self, input: &Value, state: &mut EngineState) -> Result<Value, CommandError> {
        let input: SetNavigationBlockedInput = serde_json::from_value(input.clone())?;
        let envelope = state.documents.get(&input.stage_id)?.clone();
        let mut stage = StageDocument::from_envelope(&envelope)?;
        let cell = HexCoord::new(input.q, input.r);

        if !grid_navigation_supported(&stage.coordinate_space) {
            return Err(CommandError::InvalidInput(
                STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE.into(),
            ));
        }
        if !stage_position_in_bounds(&stage.coordinate_space, cell) {
            return Err(CommandError::InvalidInput(
                STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS.into(),
            ));
        }

        let changed = set_blocked(&mut stage, cell, input.blocked);
        if changed {
            state.documents.upsert(stage.to_envelope()?);
        }
        Ok(json!({
            "stage_id": input.stage_id,
            "cell": cell,
            "blocked": input.blocked,
            "changed": changed
        }))
    }
}

#[derive(Debug, Deserialize)]
struct ReachabilityInput {
    stage_id: DocumentId,
    from_object_id: ObjectId,
    to_object_id: ObjectId,
}

pub(crate) struct ReachabilityQuery;

impl QueryHandler for ReachabilityQuery {
    fn descriptor(&self) -> QueryDescriptor {
        QueryDescriptor {
            id: ToolId::from_static("stage.reachability"),
            title: "Stage object reachability".into(),
            description: "Return deterministic shortest-path reachability between two Stage objects."
                .into(),
            input_schema: json!({
                "type": "object",
                "required": ["stage_id", "from_object_id", "to_object_id"],
                "properties": {
                    "stage_id": {"type": "string"},
                    "from_object_id": {"type": "string"},
                    "to_object_id": {"type": "string"}
                }
            }),
            output_schema: json!({"type": "object"}),
            required_permissions: PermissionSet::new([Permission::Read, Permission::RunValidation]),
            related_tools: vec![
                ToolId::from_static("stage.set_navigation_blocked"),
                ToolId::from_static("stage.move_object"),
            ],
        }
    }

    fn document_access(&self, input: &Value) -> Result<DocumentAccessPlan, QueryError> {
        let input: ReachabilityInput = serde_json::from_value(input.clone())?;
        Ok(DocumentAccessPlan::document(input.stage_id))
    }

    fn execute(&self, input: &Value, store: &DocumentStore) -> Result<Value, QueryError> {
        let input: ReachabilityInput = serde_json::from_value(input.clone())?;
        let document = store
            .get(&input.stage_id)
            .map_err(|error| QueryError::Failed(error.to_string()))?;
        let stage = StageDocument::from_envelope(document)
            .map_err(|error| QueryError::Failed(error.to_string()))?;

        let Some(from) = stage
            .objects
            .iter()
            .find(|object| object.id == input.from_object_id)
        else {
            return Ok(query_diagnostic_result(
                "invalid",
                &input,
                object_not_found(&input.stage_id, &input.from_object_id),
            ));
        };
        let Some(to) = stage
            .objects
            .iter()
            .find(|object| object.id == input.to_object_id)
        else {
            return Ok(query_diagnostic_result(
                "invalid",
                &input,
                object_not_found(&input.stage_id, &input.to_object_id),
            ));
        };

        match shortest_path(&stage, from.position, to.position) {
            Ok(Some(path)) => Ok(json!({
                "status": "ok",
                "stage_id": input.stage_id,
                "from_object_id": input.from_object_id,
                "to_object_id": input.to_object_id,
                "reachable": true,
                "distance": path.len().saturating_sub(1),
                "path": path,
                "diagnostics": []
            })),
            Ok(None) => Ok(json!({
                "status": "ok",
                "stage_id": input.stage_id,
                "from_object_id": input.from_object_id,
                "to_object_id": input.to_object_id,
                "reachable": false,
                "distance": Value::Null,
                "path": [],
                "diagnostics": []
            })),
            Err(NavigationError::UnsupportedCoordinateSpace) => Ok(query_diagnostic_result(
                "unsupported",
                &input,
                unsupported_coordinate_space(&input.stage_id),
            )),
            Err(NavigationError::EndpointOutOfBounds { position }) => Ok(query_diagnostic_result(
                "invalid",
                &input,
                endpoint_out_of_bounds(&input.stage_id, position),
            )),
        }
    }
}

fn stage_for_validation(state: &EngineState, stage_id: &DocumentId) -> Option<StageDocument> {
    state
        .documents
        .get(stage_id)
        .ok()
        .and_then(|envelope| StageDocument::from_envelope(envelope).ok())
}

fn grid_navigation_supported(space: &CoordinateSpace) -> bool {
    matches!(space, CoordinateSpace::HexAxial { .. } | CoordinateSpace::Square { .. })
}

fn set_blocked(stage: &mut StageDocument, cell: HexCoord, blocked: bool) -> bool {
    if blocked {
        if stage.navigation.blocked.contains(&cell) {
            false
        } else {
            stage.navigation.blocked.push(cell);
            stage
                .navigation
                .blocked
                .sort_by_key(|position| (position.q, position.r));
            true
        }
    } else {
        let before = stage.navigation.blocked.len();
        stage.navigation.blocked.retain(|position| *position != cell);
        if stage.navigation.blocked.len() == before {
            false
        } else {
            stage
                .navigation
                .blocked
                .sort_by_key(|position| (position.q, position.r));
            true
        }
    }
}

fn query_diagnostic_result(status: &str, input: &ReachabilityInput, diagnostic: Diagnostic) -> Value {
    json!({
        "status": status,
        "stage_id": input.stage_id,
        "from_object_id": input.from_object_id,
        "to_object_id": input.to_object_id,
        "reachable": Value::Null,
        "distance": Value::Null,
        "path": [],
        "diagnostics": [diagnostic]
    })
}

fn object_not_found(stage_id: &DocumentId, object_id: &ObjectId) -> Diagnostic {
    Diagnostic {
        code: STAGE_OBJECT_NOT_FOUND.into(),
        severity: Severity::Error,
        message: format!("Stage object not found: {object_id}"),
        location: Some(DiagnosticLocation {
            document: Some(stage_id.clone()),
            object: Some(object_id.clone()),
            field_path: Some("/objects".into()),
        }),
        suggested_tools: vec!["project.document.get".into()],
        auto_fixable: false,
    }
}

fn unsupported_coordinate_space(stage_id: &DocumentId) -> Diagnostic {
    Diagnostic {
        code: STAGE_NAV_UNSUPPORTED_COORDINATE_SPACE.into(),
        severity: Severity::Error,
        message: "grid navigation is only supported for HexAxial and Square Stages".into(),
        location: Some(DiagnosticLocation {
            document: Some(stage_id.clone()),
            object: None,
            field_path: Some("/coordinate_space".into()),
        }),
        suggested_tools: Vec::new(),
        auto_fixable: false,
    }
}

fn blocked_cell_out_of_bounds(stage_id: &DocumentId, cell: HexCoord) -> Diagnostic {
    Diagnostic {
        code: STAGE_NAV_BLOCKED_CELL_OUT_OF_BOUNDS.into(),
        severity: Severity::Error,
        message: format!("navigation blocked cell ({}, {}) is outside Stage bounds", cell.q, cell.r),
        location: Some(DiagnosticLocation {
            document: Some(stage_id.clone()),
            object: None,
            field_path: Some("/navigation/blocked".into()),
        }),
        suggested_tools: vec!["stage.set_navigation_blocked".into()],
        auto_fixable: false,
    }
}

fn endpoint_out_of_bounds(stage_id: &DocumentId, position: HexCoord) -> Diagnostic {
    Diagnostic {
        code: STAGE_NAV_ENDPOINT_OUT_OF_BOUNDS.into(),
        severity: Severity::Error,
        message: format!(
            "reachability endpoint ({}, {}) is outside Stage bounds",
            position.q, position.r
        ),
        location: Some(DiagnosticLocation {
            document: Some(stage_id.clone()),
            object: None,
            field_path: Some("/objects".into()),
        }),
        suggested_tools: vec!["stage.move_object".into()],
        auto_fixable: false,
    }
}
