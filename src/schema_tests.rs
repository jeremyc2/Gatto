//! Validate actual persisted records, including their annotation sidecars.
use std::sync::Arc;

use serde_json::{Value, json};

use crate::{
    annotation::{AnnotationDocument, Comparison, Document, Mark, Pos, Rect, Style, Tool},
    model::{AttachmentUploadState, Media, Repository, StagedAttachment},
    projects::Project,
    settings::{AppSettings, UploadAction},
};

const SETTINGS: &str = include_str!("../schemas/gatto-settings.schema.json");
const PROJECT: &str = include_str!("../schemas/gatto-project.schema.json");
const ANNOTATIONS: &str = include_str!("../schemas/gatto-annotations.schema.json");

fn validator(source: &str) -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(source).unwrap();
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap()
}

fn assert_valid(validator: &jsonschema::Validator, value: &Value) {
    let errors = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn settings_schema_accepts_saved_defaults_preferences_and_legacy_defaults() {
    let validator = validator(SETTINGS);
    let mut settings = AppSettings::default();
    let defaults = serde_json::to_value(&settings).unwrap();
    assert_valid(&validator, &defaults);
    assert!(defaults.get("last_repository").is_none());
    assert!(defaults.get("last_repository_id").is_none());
    settings.owner = Some("acme".into());
    settings.add_from_clipboard_project = Some("Review".into());
    settings.pinned_projects.insert("Notes".into());
    settings.upload_action = UploadAction::UploadAndCopy;
    assert_valid(&validator, &serde_json::to_value(settings).unwrap());
    // Settings accept omitted preferences and default them, unlike portable manifests.
    assert_valid(&validator, &json!({}));
    for invalid in [
        ".",
        "..",
        "../Review",
        "/Review",
        "Review/Notes",
        "bad\nname",
    ] {
        let mut value = defaults.clone();
        value["add_from_clipboard_project"] = json!(invalid);
        assert!(!validator.is_valid(&value), "{invalid:?}");
    }
    for key in [
        "last_repository",
        "last_repository_id",
        "unknown_preference",
    ] {
        let mut value = defaults.clone();
        value[key] = json!("old");
        assert!(!validator.is_valid(&value));
    }
}

#[test]
fn saved_project_media_and_annotation_sidecars_match_their_schemas() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = include_bytes!("../packaging/menu-bar-icon.png");
    let mut item = StagedAttachment::from_bytes("cat.png".into(), bytes.to_vec(), None).unwrap();
    let mut document = Document::decode(bytes).unwrap();
    let mut mark = Mark::new(Tool::Marker, Pos::new(5., 5.), Style::default());
    mark.number = Some(1);
    mark.description = "A note".into();
    document.marks.push(mark);
    document.next_marker = 2;
    document.comparison = Some(Comparison::new([
        Rect {
            x: 0.,
            y: 0.,
            w: 10.,
            h: 20.,
        },
        Rect {
            x: 20.,
            y: 0.,
            w: 10.,
            h: 20.,
        },
    ]));
    if let Media::Image(image) = &mut item.media {
        image.annotation_document = Some(Arc::new(AnnotationDocument::new(&document)));
    }
    item.upload_state = AttachmentUploadState::Uploaded {
        url: "https://github.com/user-attachments/assets/test".into(),
    };
    let mut project = Project::create_in_parent(directory.path(), "Review", &[]).unwrap();
    project.metadata.repository = Some(Repository {
        id: 42,
        name: "images".into(),
    });
    project.metadata.repository_owner = Some("acme".into());
    project.save(&[item]).unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(project.folder.join("project.json")).unwrap())
            .unwrap();
    let project_validator = validator(PROJECT);
    assert_valid(&project_validator, &manifest);
    let sidecar = manifest["attachments"][0]["annotations"]["json"]
        .as_str()
        .unwrap();
    let annotations: Value =
        serde_json::from_slice(&std::fs::read(project.folder.join("media").join(sidecar)).unwrap())
            .unwrap();
    assert_valid(&validator(ANNOTATIONS), &annotations);
    let mut malformed = manifest.clone();
    malformed["attachments"][0]["file"] =
        json!(format!("{}-{}.png", "-".repeat(36), "a".repeat(64)));
    assert!(!project_validator.is_valid(&malformed));
    let mut future = annotations.clone();
    future["version"] = json!(2);
    assert!(!validator(ANNOTATIONS).is_valid(&future));
    let mut bad_points = annotations;
    bad_points["marks"][0]["points"] = json!([]);
    assert!(!validator(ANNOTATIONS).is_valid(&bad_points));
}

#[test]
fn all_schemas_reject_unknown_versions_and_properties() {
    for source in [SETTINGS, PROJECT, ANNOTATIONS] {
        let schema: Value = serde_json::from_str(source).unwrap();
        assert!(jsonschema::meta::is_valid(&schema));
        assert_eq!(schema["properties"]["version"]["const"], 1);
        assert_eq!(schema["additionalProperties"], false);
        let validator = validator(source);
        assert!(!validator.is_valid(&json!({"version": 2})));
    }
}
