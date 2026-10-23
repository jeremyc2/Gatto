//! Numbered badges keep their explanatory text outside the rendered image.
use super::*;

impl Mark {
    pub(super) fn marker_radius(&self) -> f32 {
        18. * self.style.scale
    }

    /// Shared filled vector paths for the editor and PNG exporter.
    pub(super) fn marker_layers(&self) -> Vec<(Path, [u8; 3])> {
        let center = self.points[0];
        let radius = self.marker_radius();
        let unit = self.style.scale;
        let color = COLORS[self.style.color];
        let luminance = color
            .iter()
            .map(|c| {
                let c = f32::from(*c) / 255.;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            })
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(c, weight)| c * weight)
            .sum::<f32>();
        let edge = if luminance > 0.179 {
            [16, 16, 22]
        } else {
            [255, 255, 255]
        };
        let mut layers = Vec::new();
        if let Some(body) = PathBuilder::from_circle(center.x, center.y, radius) {
            layers.push((body, color));
        }
        // Two separated concentric rings give markers their own silhouette in every color.
        for inset in [1.5, 4.5] {
            if let Some(ring) = PathBuilder::from_circle(center.x, center.y, radius - inset * unit)
                .and_then(|path| {
                    path.stroke(
                        &Stroke {
                            width: unit,
                            ..Stroke::default()
                        },
                        1.,
                    )
                })
            {
                layers.push((ring, edge));
            }
        }
        let mut number = Mark::new(
            Tool::Text,
            center,
            Style {
                scale: unit,
                font_size: 21. * unit,
                ..Style::default()
            },
        );
        number.text = self.number.unwrap_or(1).to_string();
        if let Some(path) = number.standalone_path() {
            let bounds = path.bounds();
            let fit = ((radius - 6. * unit) * 2. / bounds.width().hypot(bounds.height())).min(1.);
            let x = bounds.left() + bounds.width() / 2.;
            let y = bounds.top() + bounds.height() / 2.;
            if let Some(path) = path.transform(Transform::from_row(
                fit,
                0.,
                0.,
                fit,
                center.x - x * fit,
                center.y - y * fit,
            )) {
                layers.push((path, edge));
            }
        }
        layers
    }

    pub(super) fn paint_marker(&self, layer: &mut Pixmap, transform: Transform) {
        for (path, color) in self.marker_layers() {
            let mut ink = Paint::default();
            ink.set_color_rgba8(color[0], color[1], color[2], 255);
            layer.fill_path(&path, &ink, FillRule::Winding, transform, None);
        }
    }
}

impl Document {
    pub fn marker_count(&self) -> usize {
        self.marks
            .iter()
            .filter(|mark| mark.tool == Tool::Marker)
            .count()
    }

    pub fn number_marker(&mut self, mark: &mut Mark) -> bool {
        if mark.tool != Tool::Marker {
            return true;
        }
        if self.marker_count() >= 99 {
            return false;
        }
        if let Some(existing) = self.marks.iter().find(|mark| mark.tool == Tool::Marker) {
            mark.style = mark.style.clone().with_scale(existing.style.scale);
        }
        mark.rotation = 0.;
        mark.number = Some(self.next_marker);
        self.next_marker += 1;
        true
    }

    /// Number by sequence identity, independent of visual stacking order.
    pub fn renumber_markers(&mut self) {
        let mut indices: Vec<_> = self
            .marks
            .iter()
            .enumerate()
            .filter(|(_, mark)| mark.tool == Tool::Marker)
            .map(|(i, mark)| (mark.number.unwrap_or(u32::MAX), i))
            .collect();
        indices.sort_unstable();
        for (number, (_, index)) in indices.iter().enumerate() {
            self.marks[*index].number = Some(number as u32 + 1);
        }
        self.next_marker = indices.len() as u32 + 1;
    }

    pub fn resize_markers(&mut self, scale: f32) {
        for mark in self
            .marks
            .iter_mut()
            .filter(|mark| mark.tool == Tool::Marker)
        {
            mark.style = mark.style.clone().with_scale(scale);
            mark.rotation = 0.;
        }
    }

    pub fn marker_descriptions(&self) -> Vec<(u32, String)> {
        let mut descriptions: Vec<_> = self
            .marks
            .iter()
            .filter(|mark| mark.tool == Tool::Marker)
            .filter_map(|mark| Some((mark.number?, mark.description.trim().to_owned())))
            .collect();
        descriptions.sort_by_key(|(number, _)| *number);
        descriptions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbering_survives_deletion_and_history_and_stays_per_image() {
        let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
        let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
        doc.checkpoint();
        doc.number_marker(&mut mark);
        doc.marks.push(mark.clone());
        doc.checkpoint();
        doc.marks.clear();
        doc.renumber_markers();
        doc.checkpoint();
        doc.number_marker(&mut mark);
        doc.marks.push(mark);
        assert_eq!(doc.marks[0].number, Some(1));
        assert_eq!(doc.next_marker, 2);
        doc.undo();
        assert!(doc.marks.is_empty());
        assert_eq!(doc.next_marker, 1);
        doc.undo();
        assert_eq!(doc.marks[0].number, Some(1));
        doc.redo();
        doc.redo();
        assert_eq!(doc.snapshot().next_marker, 2);
        assert_eq!(
            Document::from_image(image::RgbaImage::new(1, 1)).next_marker,
            1
        );
    }

    #[test]
    fn deleting_first_or_middle_renumbers_in_sequence_order_and_keeps_notes() {
        for remove in [1, 2] {
            let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
            for description in ["First", "Second", "Third"] {
                let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
                doc.number_marker(&mut mark);
                mark.description = description.into();
                doc.marks.push(mark);
            }
            doc.marks.reverse();
            doc.checkpoint();
            doc.marks.retain(|mark| mark.number != Some(remove));
            doc.renumber_markers();
            assert_eq!(
                doc.marker_descriptions(),
                if remove == 1 {
                    vec![(1, "Second".into()), (2, "Third".into())]
                } else {
                    vec![(1, "First".into()), (2, "Third".into())]
                }
            );
            assert_eq!(doc.next_marker, 3);
            doc.undo();
            assert_eq!(
                doc.marker_descriptions(),
                vec![
                    (1, "First".into()),
                    (2, "Second".into()),
                    (3, "Third".into())
                ]
            );
        }
    }

    #[test]
    fn one_and_two_digit_glyphs_fit_inside_both_rings_at_every_size() {
        for scale in [0.5, 1., 3.] {
            for number in [1, 9, 10, 11, 88, 99] {
                let mut mark = Mark::new(
                    Tool::Marker,
                    Pos::new(100., 100.),
                    Style::default().with_scale(scale),
                );
                mark.number = Some(number);
                let layers = mark.marker_layers();
                assert_eq!(layers.len(), 4);
                let b = layers.last().unwrap().0.bounds();
                let center = mark.points[0];
                for p in [Pos::new(b.left(), b.top()), Pos::new(b.right(), b.bottom())] {
                    assert!(p.distance(center) <= mark.marker_radius() - 5.9 * scale);
                }
            }
        }
    }

    #[test]
    fn descriptions_follow_numbers_after_stacking_changes_and_keep_blank_notes() {
        let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
        for description in [" First ", " \n ", "Third"] {
            let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
            doc.number_marker(&mut mark);
            mark.description = description.into();
            doc.marks.push(mark);
        }
        doc.marks.reverse();
        assert_eq!(
            doc.marker_descriptions(),
            vec![(1, "First".into()), (2, String::new()), (3, "Third".into())]
        );
    }

    #[test]
    fn description_never_changes_image_pixels_or_bounds() {
        let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
        let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
        doc.number_marker(&mut mark);
        doc.marks.push(mark);
        let before = doc.png(None).unwrap();
        doc.marks[0].description = "Private explanation 👋\nSecond line".into();
        let after = doc.png(None).unwrap();
        assert_eq!(before, after);
        assert_eq!(
            doc.marker_descriptions(),
            vec![(1, "Private explanation 👋\nSecond line".into())]
        );
        let output = image::load_from_memory(&after.0).unwrap().to_rgba8();
        assert!(output.pixels().any(|pixel| pixel[3] != 0));
        assert!(doc.marks[0].hit(Pos::new(50., 50.), 0.));
        assert!(!doc.marks[0].hit(Pos::new(67., 67.), 0.));
    }
}
