use guiyi_engine_content::DocumentStore;
use guiyi_engine_validation::{Diagnostic, DiagnosticBag, DiagnosticLocation, Severity};
use std::collections::BTreeSet;
use tactical_rpg_content::{
    EncounterDocument, StageDocument, StageObjectKind, STAGE_DOCUMENT_TYPE,
};

pub const ENCOUNTER_NAME_EMPTY: &str = "ENCOUNTER_NAME_EMPTY";
pub const ENCOUNTER_NO_PARTICIPANTS: &str = "ENCOUNTER_NO_PARTICIPANTS";
pub const ENCOUNTER_DUPLICATE_PARTICIPANT_ID: &str = "ENCOUNTER_DUPLICATE_PARTICIPANT_ID";
pub const ENCOUNTER_PARTICIPANT_SIDE_EMPTY: &str = "ENCOUNTER_PARTICIPANT_SIDE_EMPTY";
pub const ENCOUNTER_STAGE_NOT_FOUND: &str = "ENCOUNTER_STAGE_NOT_FOUND";
pub const ENCOUNTER_STAGE_INVALID_TYPE: &str = "ENCOUNTER_STAGE_INVALID_TYPE";
pub const ENCOUNTER_SPAWN_NOT_FOUND: &str = "ENCOUNTER_SPAWN_NOT_FOUND";
pub const ENCOUNTER_SPAWN_INVALID_KIND: &str = "ENCOUNTER_SPAWN_INVALID_KIND";

pub fn validate_encounter(encounter: &EncounterDocument) -> DiagnosticBag {
    let mut bag = DiagnosticBag::default();
    if encounter.name.trim().is_empty() {
        bag.push(
            Diagnostic::error(ENCOUNTER_NAME_EMPTY, "encounter name cannot be empty")
                .at_document(encounter.id.clone()),
        );
    }
    if encounter.participants.is_empty() {
        bag.push(
            Diagnostic::warning(
                ENCOUNTER_NO_PARTICIPANTS,
                "encounter has no participants and is not yet playable",
            )
            .at_document(encounter.id.clone()),
        );
    }

    let mut ids = BTreeSet::new();
    for participant in &encounter.participants {
        if !ids.insert(participant.id.clone()) {
            bag.push(Diagnostic {
                code: ENCOUNTER_DUPLICATE_PARTICIPANT_ID.into(),
                severity: Severity::Error,
                message: format!("duplicate Encounter participant id: {}", participant.id),
                location: Some(DiagnosticLocation {
                    document: Some(encounter.id.clone()),
                    object: Some(participant.id.clone()),
                    field_path: Some("/participants".into()),
                }),
                suggested_tools: vec!["encounter.remove_participant".into()],
                auto_fixable: false,
            });
        }
        if participant.side.trim().is_empty() {
            bag.push(Diagnostic {
                code: ENCOUNTER_PARTICIPANT_SIDE_EMPTY.into(),
                severity: Severity::Error,
                message: format!("Encounter participant {} has an empty side", participant.id),
                location: Some(DiagnosticLocation {
                    document: Some(encounter.id.clone()),
                    object: Some(participant.id.clone()),
                    field_path: Some("side".into()),
                }),
                suggested_tools: vec!["encounter.add_participant".into()],
                auto_fixable: false,
            });
        }
    }
    bag
}

pub fn validate_encounter_project(
    encounter: &EncounterDocument,
    store: &DocumentStore,
) -> DiagnosticBag {
    let mut bag = validate_encounter(encounter);
    let stage_envelope = match store.get(&encounter.stage) {
        Ok(stage) => stage,
        Err(_) => {
            bag.push(
                Diagnostic::error(
                    ENCOUNTER_STAGE_NOT_FOUND,
                    format!("Encounter Stage {} does not exist", encounter.stage),
                )
                .at_document(encounter.id.clone()),
            );
            return bag;
        }
    };
    if stage_envelope.header.type_id.as_str() != STAGE_DOCUMENT_TYPE {
        bag.push(
            Diagnostic::error(
                ENCOUNTER_STAGE_INVALID_TYPE,
                format!(
                    "Encounter Stage {} has type {}, expected {}",
                    encounter.stage, stage_envelope.header.type_id, STAGE_DOCUMENT_TYPE
                ),
            )
            .at_document(encounter.id.clone()),
        );
        return bag;
    }
    let stage = match StageDocument::from_envelope(stage_envelope) {
        Ok(stage) => stage,
        Err(error) => {
            bag.push(
                Diagnostic::error(ENCOUNTER_STAGE_INVALID_TYPE, error.to_string())
                    .at_document(encounter.id.clone()),
            );
            return bag;
        }
    };

    for participant in &encounter.participants {
        let Some(object) = stage
            .objects
            .iter()
            .find(|object| object.id == participant.spawn_point)
        else {
            bag.push(Diagnostic {
                code: ENCOUNTER_SPAWN_NOT_FOUND.into(),
                severity: Severity::Error,
                message: format!(
                    "Encounter participant {} references missing spawn point {}",
                    participant.id, participant.spawn_point
                ),
                location: Some(DiagnosticLocation {
                    document: Some(encounter.id.clone()),
                    object: Some(participant.id.clone()),
                    field_path: Some("spawn_point".into()),
                }),
                suggested_tools: vec!["stage.create_spawn".into()],
                auto_fixable: false,
            });
            continue;
        };
        if !matches!(&object.object, StageObjectKind::SpawnPoint { .. }) {
            bag.push(Diagnostic {
                code: ENCOUNTER_SPAWN_INVALID_KIND.into(),
                severity: Severity::Error,
                message: format!(
                    "Encounter participant {} references Stage object {} that is not a spawn point",
                    participant.id, participant.spawn_point
                ),
                location: Some(DiagnosticLocation {
                    document: Some(encounter.id.clone()),
                    object: Some(participant.id.clone()),
                    field_path: Some("spawn_point".into()),
                }),
                suggested_tools: vec!["stage.create_spawn".into()],
                auto_fixable: false,
            });
        }
    }
    bag
}
