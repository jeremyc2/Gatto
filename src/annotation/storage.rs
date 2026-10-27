use super::*;
use image::ImageEncoder as _;
use serde::{Deserialize, Serialize};

pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jeremyc2/Gatto/main/schemas/gatto-annotations.schema.json";

#[derive(Clone, Serialize, Deserialize)]
pub struct AnnotationState {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: u32,
    pub next_marker: u32,
    pub annotation_scale: Option<f32>,
    pub marks: Vec<Mark>,
    pub comparison: Option<Comparison>,
}

#[derive(Clone)]
pub struct AnnotationDocument {
    base: Arc<image::RgbaImage>,
    state: AnnotationState,
    source_png: Arc<OnceLock<Arc<Vec<u8>>>>,
}

impl AnnotationDocument {
    pub fn new(document: &Document) -> Self {
        Self {
            base: document.base.clone(),
            source_png: document.source_png.clone(),
            state: AnnotationState {
                schema: SCHEMA_URL.into(),
                version: 1,
                next_marker: document.next_marker,
                annotation_scale: document.annotation_scale.get().copied(),
                marks: document.marks.clone(),
                comparison: document.comparison.clone(),
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.state.marks.is_empty() && self.state.comparison.is_none()
    }

    pub fn source_png(&self) -> Result<Arc<Vec<u8>>> {
        if let Some(bytes) = self.source_png.get() {
            return Ok(bytes.clone());
        }
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes).write_image(
            self.base.as_raw(),
            self.base.width(),
            self.base.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        let _ = self.source_png.set(Arc::new(bytes));
        Ok(self
            .source_png
            .get()
            .expect("encoded annotation source")
            .clone())
    }

    pub fn json(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(&self.state)?)
    }

    pub fn from_files(source: Vec<u8>, json: &[u8]) -> Result<Self> {
        let state: AnnotationState = serde_json::from_slice(json)?;
        anyhow::ensure!(
            state.version == 1 && state.next_marker > 0,
            "Unsupported annotation data"
        );
        anyhow::ensure!(
            state
                .annotation_scale
                .is_none_or(|scale| scale.is_finite() && scale > 0.),
            "Invalid annotation scale"
        );
        for mark in &state.marks {
            anyhow::ensure!(
                mark.points.len() >= 2
                    && mark
                        .points
                        .iter()
                        .all(|p| p.x.is_finite() && p.y.is_finite())
                    && mark.style.color < COLORS.len()
                    && mark.style.scale.is_finite()
                    && mark.style.scale > 0.
                    && [
                        mark.style.width,
                        mark.style.opacity,
                        mark.style.font_size,
                        mark.rotation,
                        mark.label_position
                    ]
                    .iter()
                    .all(|n| n.is_finite())
                    && mark.wrap.is_none_or(f32::is_finite)
                    && mark.label_base_height.is_none_or(f32::is_finite),
                "Invalid annotation geometry"
            );
        }
        if let Some(comparison) = &state.comparison {
            comparison.validate()?;
        }
        anyhow::ensure!(
            image::guess_format(&source)? == image::ImageFormat::Png,
            "Annotation source must be PNG"
        );
        let base = Arc::new(image::load_from_memory(&source)?.to_rgba8());
        let source_png = Arc::new(OnceLock::from(Arc::new(source)));
        Ok(Self {
            base,
            state,
            source_png,
        })
    }

    pub fn document(&self) -> Document {
        let annotation_scale = OnceLock::new();
        if let Some(scale) = self.state.annotation_scale {
            let _ = annotation_scale.set(scale);
        }
        Document {
            base: self.base.clone(),
            marks: self.state.marks.clone(),
            comparison: self.state.comparison.clone(),
            next_marker: self.state.next_marker,
            annotation_scale,
            undo: Vec::new(),
            redo: Vec::new(),
            mosaic: Arc::default(),
            source_png: self.source_png.clone(),
        }
    }
}

impl Session {
    pub fn saved_annotations(&self) -> Option<Arc<AnnotationDocument>> {
        let retained = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let retained = retained.as_ref()?;
        let mut document = retained.document.snapshot();
        document.marks.clone_from(&retained.saved_marks);
        document.comparison.clone_from(&retained.saved_comparison);
        document.next_marker = retained.saved_next_marker;
        let annotations = AnnotationDocument::new(&document);
        (!annotations.is_empty()).then(|| Arc::new(annotations))
    }

    pub fn restore_annotations(&self, annotations: &AnnotationDocument) -> Result<()> {
        let document = annotations.document();
        let saved = document.edit_state();
        let image = Arc::new(gpui_kit::Image::from_bytes(
            gpui_kit::ImageFormat::Png,
            annotations.source_png()?.as_ref().clone(),
        ));
        self.retain(document, image, saved);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_annotations_round_trip_with_original_pixels_and_all_settings() {
        let bytes = include_bytes!("../../packaging/menu-bar-icon.png");
        let mut document = Document::from_image(image::load_from_memory(bytes).unwrap().to_rgba8());
        document.checkpoint();
        let mut mark = Mark::new(
            Tool::Arrow,
            Pos::new(-12., 9.),
            Style::default().with_scale(2.),
        );
        mark.points[1] = Pos::new(48., 80.);
        mark.text = "Label".into();
        mark.description = "Marker details".into();
        mark.rotation = 0.4;
        mark.flip_x = true;
        mark.style.color = 5;
        mark.style.fill = Fill::Hatch;
        mark.style.dash = Dash::Dashed;
        mark.style.opacity = 0.7;
        mark.style.text_align = Some(Align::End);
        document.marks.push(mark);
        document.next_marker = 8;
        document.annotation_scale.set(2.).unwrap();
        document.comparison = Some(Comparison::new([
            Rect {
                x: 0.,
                y: 0.,
                w: 20.,
                h: 30.,
            },
            Rect {
                x: 30.,
                y: 0.,
                w: 20.,
                h: 30.,
            },
        ]));
        let saved = AnnotationDocument::new(&document);
        let json = saved.json().unwrap();
        let restored =
            AnnotationDocument::from_files(saved.source_png().unwrap().as_ref().clone(), &json)
                .unwrap()
                .document();
        assert_eq!(restored.base, document.base);
        assert_eq!(restored.marks, document.marks);
        assert_eq!(restored.comparison, document.comparison);
        assert_eq!(restored.next_marker, 8);
        assert_eq!(restored.annotation_scale.get(), Some(&2.));
        assert!(!restored.can_undo());
        assert!(!restored.can_redo());
        let mut invalid: serde_json::Value = serde_json::from_slice(&json).unwrap();
        invalid["marks"][0]["style"]["color"] = serde_json::json!(COLORS.len());
        assert!(
            AnnotationDocument::from_files(
                saved.source_png().unwrap().as_ref().clone(),
                &serde_json::to_vec(&invalid).unwrap()
            )
            .is_err()
        );
    }
}
