use super::*;

impl Document {
    fn group_stack_order(&self, selected: &[usize], order: StackOrder) -> Vec<usize> {
        let mut slots: Vec<_> = (0..self.marks.len())
            .filter(|i| self.marks[*i].tool != Tool::Redact)
            .collect();
        let chosen: Vec<_> = slots
            .iter()
            .copied()
            .filter(|i| selected.contains(i))
            .collect();
        if chosen.is_empty() {
            return (0..self.marks.len()).collect();
        }
        let others: Vec<_> = slots
            .iter()
            .copied()
            .filter(|i| !selected.contains(i))
            .collect();
        let overlaps = |i: usize| {
            chosen.iter().any(|s| {
                self.marks[*s]
                    .world_bounds()
                    .intersects(self.marks[i].world_bounds())
            })
        };
        let target = match order {
            StackOrder::Front => Some(others.len()),
            StackOrder::Back => Some(0),
            StackOrder::Forward => others
                .iter()
                .position(|i| *i > *chosen.last().unwrap() && overlaps(*i))
                .map(|i| i + 1),
            StackOrder::Backward => others.iter().rposition(|i| *i < chosen[0] && overlaps(*i)),
        };
        if let Some(target) = target {
            let mut moved = others;
            moved.splice(target..target, chosen);
            slots = moved;
        }
        let mut slots = slots.into_iter();
        (0..self.marks.len())
            .map(|i| {
                if self.marks[i].tool == Tool::Redact {
                    i
                } else {
                    slots.next().unwrap()
                }
            })
            .collect()
    }
    pub fn can_reorder_many(&self, selected: &[usize], order: StackOrder) -> bool {
        if selected.len() == 1 {
            return self.can_reorder(selected[0], order);
        }
        self.group_stack_order(selected, order)
            .into_iter()
            .enumerate()
            .any(|(i, original)| i != original)
    }
    pub fn reorder_many(&mut self, selected: &[usize], order: StackOrder) -> Vec<usize> {
        if selected.len() == 1 {
            return vec![self.reorder(selected[0], order)];
        }
        let permutation = self.group_stack_order(selected, order);
        if permutation
            .iter()
            .enumerate()
            .any(|(i, original)| i != *original)
        {
            self.checkpoint();
            let old = self.marks.clone();
            self.marks = permutation.iter().map(|i| old[*i].clone()).collect();
        }
        permutation
            .into_iter()
            .enumerate()
            .filter(|(_, i)| selected.contains(i))
            .map(|(i, _)| i)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn group_stacking_preserves_relative_order_pixelation_and_one_step_undo() {
        let marks: Vec<_> = ["a", "b", "pixelation", "c", "d"]
            .into_iter()
            .map(|name| {
                let mut mark = Mark::new(
                    if name == "pixelation" {
                        Tool::Redact
                    } else {
                        Tool::Rectangle
                    },
                    Pos::new(10., 10.),
                    Style::default(),
                );
                mark.points[1] = Pos::new(80., 80.);
                mark.text = name.into();
                mark
            })
            .collect();
        for (order, selected, expected) in [
            (
                StackOrder::Front,
                vec![0, 3],
                vec!["b", "d", "pixelation", "a", "c"],
            ),
            (
                StackOrder::Back,
                vec![0, 3],
                vec!["a", "c", "pixelation", "b", "d"],
            ),
            (
                StackOrder::Forward,
                vec![0, 3],
                vec!["b", "d", "pixelation", "a", "c"],
            ),
            (
                StackOrder::Backward,
                vec![1, 3],
                vec!["b", "c", "pixelation", "a", "d"],
            ),
        ] {
            let mut doc = Document {
                annotation_scale: OnceLock::new(),
                base: Arc::new(image::RgbaImage::new(100, 100)),
                marks: marks.clone(),
                undo: Vec::new(),
                redo: Vec::new(),
                mosaic: Arc::default(),
            };
            assert!(doc.can_reorder_many(&selected, order));
            let moved = doc.reorder_many(&selected, order);
            assert_eq!(
                doc.marks
                    .iter()
                    .map(|m| m.text.as_str())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(moved.len(), 2);
            assert_eq!(doc.marks[2], marks[2]);
            let after = doc.marks.clone();
            doc.undo();
            assert_eq!(doc.marks, marks);
            assert!(!doc.can_undo());
            doc.redo();
            assert_eq!(doc.marks, after);
        }
    }
}
