use super::*;
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, Pixels, Point, WeakEntity, canvas, point,
};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct AttachmentDrag {
    pub id: u64,
}

pub(super) struct ReorderSession {
    id: u64,
    original: Vec<u64>,
    pointer: Point<Pixels>,
    grab_y: Pixels,
    shifts: HashMap<u64, f32>,
    last_frame: Instant,
}

pub(super) struct AttachmentDragPreview {
    pub owner: WeakEntity<UploaderApp>,
    pub attachment: StagedAttachment,
    pub width: Pixels,
    pub offset: Point<Pixels>,
}

impl Render for AttachmentDragPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pointer = window.mouse_position();
        let compact = self
            .owner
            .update(cx, |app, _| {
                !app.settings.sidebar_collapsed && pointer.x < app.main_scroll.bounds().origin.x
            })
            .unwrap_or(false);
        if compact {
            return div()
                .relative()
                .size_0()
                .child(
                    h_flex()
                        .id("compact-attachment-drag")
                        .debug_selector(|| "compact-attachment-drag".into())
                        .absolute()
                        .left(self.offset.x + px(12.))
                        .top(self.offset.y + px(12.))
                        .w(px(160.))
                        .h(px(34.))
                        .gap_2()
                        .px_2()
                        .rounded_md()
                        .shadow_lg()
                        .bg(cx.theme().sidebar_accent)
                        .border_1()
                        .border_color(cx.theme().sidebar_border)
                        .child(Icon::new(AssetIconName::Image).size(px(16.)))
                        .child(
                            div()
                                .text_xs()
                                .truncate()
                                .child(self.attachment.title.clone()),
                        ),
                )
                .into_any_element();
        }
        let card = self
            .owner
            .update(cx, |app, cx| {
                app.render_attachment_card(&self.attachment, true, cx)
            })
            .ok();
        div()
            .w(self.width)
            .when_some(card, |preview, card| preview.child(card))
            .into_any_element()
    }
}

impl UploaderApp {
    pub(super) fn register_attachment_reorder_controls(&self, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        cx.intercept_keystrokes(move |event, window, cx| {
            if event.keystroke.key != "escape" {
                return;
            }
            let canceled = owner
                .update(cx, |this, cx| {
                    if this.attachment_drag.is_none() {
                        return false;
                    }
                    this.cancel_attachment_reorder(window, cx);
                    true
                })
                .unwrap_or(false);
            if canceled {
                cx.stop_propagation();
            }
        })
        .detach();
    }

    pub(super) fn begin_attachment_reorder(
        &mut self,
        id: u64,
        grab_y: Pixels,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || self.import_busy() {
            return;
        }
        self.attachment_drag = Some(ReorderSession {
            id,
            original: self.staged_attachments.iter().map(|item| item.id).collect(),
            pointer: window.mouse_position(),
            grab_y,
            shifts: HashMap::new(),
            last_frame: Instant::now(),
        });
        self.schedule_reorder_frame(window, cx);
        cx.notify();
    }

    fn schedule_reorder_frame(&self, window: &mut Window, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        window.on_next_frame(move |window, cx| {
            let _ = owner.update(cx, |this, cx| {
                if this.scroll_attachment_reorder(window, cx) {
                    this.schedule_reorder_frame(window, cx);
                }
            });
        });
    }

    pub(super) fn move_attachment_reorder(
        &mut self,
        pointer: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = &mut self.attachment_drag else {
            return;
        };
        drag.pointer = pointer;
        self.preview_attachment_order(cx);
        cx.notify();
    }

    fn preview_attachment_order(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = &self.attachment_drag else {
            return;
        };
        let id = drag.id;
        let viewport = self.main_scroll.bounds();
        if drag.pointer.x < viewport.origin.x
            || drag.pointer.x > viewport.origin.x + viewport.size.width
        {
            return;
        }
        let bounds = self.attachment_bounds.borrow();
        let Some(source) = bounds.get(&id) else {
            return;
        };
        let center = drag.pointer.y - drag.grab_y + source.size.height / 2.;
        let Some(from) = self
            .staged_attachments
            .iter()
            .position(|item| item.id == id)
        else {
            return;
        };
        let to = self
            .staged_attachments
            .iter()
            .filter(|item| item.id != id)
            .filter(|item| {
                bounds
                    .get(&item.id)
                    .is_some_and(|rect| center > rect.center().y)
            })
            .count();
        drop(bounds);
        if from == to {
            return;
        }
        let before: HashMap<_, _> = self
            .staged_attachments
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id, index))
            .collect();
        let target = self.staged_attachments[to].id;
        crate::projects::reorder(&mut self.staged_attachments, id, target);
        let drag = self.attachment_drag.as_mut().unwrap();
        drag.shifts = self
            .staged_attachments
            .iter()
            .enumerate()
            .filter(|(_, item)| item.id != id)
            .map(|(index, item)| (item.id, (before[&item.id] as f32 - index as f32) * 72.))
            .collect();
        self.reorder_revision = self.reorder_revision.wrapping_add(1);
        self.sync_upload_result();
        cx.notify();
    }

    fn scroll_attachment_reorder(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(drag) = &mut self.attachment_drag else {
            return false;
        };
        if !cx.has_active_drag() {
            self.finish_attachment_reorder(cx);
            return false;
        }
        let step = drag.last_frame.elapsed().as_secs_f32().clamp(0.004, 0.04) * 720.;
        drag.last_frame = Instant::now();
        let bounds = self.main_scroll.bounds();
        if drag.pointer.x < bounds.origin.x || drag.pointer.x > bounds.origin.x + bounds.size.width
        {
            return true;
        }
        let y = drag.pointer.y;
        let top = bounds.origin.y;
        let bottom = top + bounds.size.height;
        let edge = px(44.);
        let speed = if y < top + edge {
            ((top + edge - y) / edge).clamp(0., 1.) * step
        } else if y > bottom - edge {
            -((y - bottom + edge) / edge).clamp(0., 1.) * step
        } else {
            0.
        };
        if speed != 0. {
            let offset = self.main_scroll.offset();
            let next =
                (offset.y + px(speed)).clamp(-self.main_scroll.max_offset().y.max(px(0.)), px(0.));
            if next != offset.y {
                self.main_scroll.set_offset(point(offset.x, next));
                self.preview_attachment_order(cx);
                window.refresh();
            }
        }
        true
    }

    pub(super) fn drop_attachment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = &self.attachment_drag else {
            return;
        };
        let id = drag.id;
        let target = if self.settings.sidebar_collapsed {
            None
        } else {
            self.project_bounds
                .borrow()
                .iter()
                .find(|(_, bounds)| bounds.contains(&window.mouse_position()))
                .map(|(folder, _)| folder.clone())
        };
        if let Some(folder) = target {
            self.cancel_attachment_reorder(window, cx);
            if self.project_state.folder.as_ref() != Some(&folder) {
                self.move_attachment_to_project(id, folder, window, cx);
            }
        } else if !self.main_scroll.bounds().contains(&window.mouse_position()) {
            self.cancel_attachment_reorder(window, cx);
        } else {
            self.finish_attachment_reorder(cx);
            cx.stop_active_drag(window);
        }
    }

    pub(super) fn finish_attachment_reorder(&mut self, cx: &mut Context<Self>) {
        let Some(drag) = self.attachment_drag.take() else {
            return;
        };
        if self
            .staged_attachments
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>()
            != drag.original
        {
            self.sync_upload_result();
            self.save_project(cx);
        }
        cx.notify();
    }

    pub(super) fn cancel_attachment_reorder(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.attachment_drag.take() else {
            return;
        };
        let order: HashMap<_, _> = drag
            .original
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect();
        self.staged_attachments
            .sort_by_key(|item| order.get(&item.id).copied().unwrap_or(usize::MAX));
        self.sync_upload_result();
        cx.stop_active_drag(window);
        cx.notify();
    }

    pub(super) fn render_reorder_row(
        &self,
        image: &StagedAttachment,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = image.id;
        let dragged = self
            .attachment_drag
            .as_ref()
            .is_some_and(|drag| drag.id == id);
        let shift = self
            .attachment_drag
            .as_ref()
            .and_then(|drag| drag.shifts.get(&id))
            .copied()
            .unwrap_or(0.);
        let card = if dragged {
            div().h(px(64.)).into_any_element()
        } else {
            self.render_attachment_card(image, false, cx)
        };
        let layout = self.attachment_bounds.clone();
        let animated = div().w_full().child(card).map(|card| {
            if shift != 0. && !cx.reduce_motion() {
                card.with_animation(
                    format!("attachment-shift-{id}-{}", self.reorder_revision),
                    Animation::new(Duration::from_millis(140))
                        .with_easing(|t| 1. - (1. - t).powi(3)),
                    move |card, t| card.relative().top(px(shift * (1. - t))),
                )
                .into_any_element()
            } else {
                card.into_any_element()
            }
        });
        div()
            .id(format!("attachment-row-{id}"))
            .w_full()
            .h(px(64.))
            .relative()
            .child(animated)
            .child(
                canvas(
                    move |bounds, _, _| {
                        layout.borrow_mut().insert(id, bounds);
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext};

    fn populate(view: &Entity<UploaderApp>, cx: &mut gpui_kit::VisualTestContext, count: usize) {
        cx.update(|window, cx| {
            crate::application_menu::init(cx);
            crate::app::init_keybindings(cx);
            view.update(cx, |app, cx| {
                app.staged_attachments = (1..=count)
                    .map(|index| {
                        let mut item = StagedAttachment::from_bytes(
                            format!("Image {index}.png"),
                            include_bytes!("../../packaging/menu-bar-icon.png").to_vec(),
                            None,
                        )
                        .unwrap();
                        item.id = index as u64;
                        item
                    })
                    .collect();
                app.focus_handle.focus(window, cx);
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
    }

    fn begin(cx: &mut gpui_kit::VisualTestContext, from: Point<Pixels>) {
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_move(
            from + point(px(0.), px(6.)),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.update(|window, cx| {
            assert!(cx.has_active_drag());
            window.draw(cx).clear(cx);
        });
    }

    #[gpui_kit::test]
    fn whole_card_drag_uses_a_full_card_and_escape_restores_the_starting_order(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        populate(&view, cx, 2);
        let card = cx.debug_bounds("attachment-card-1").unwrap();
        let from = card.origin + point(px(150.), px(32.));
        let to = cx.debug_bounds("attachment-card-2").unwrap().center() + point(px(0.), px(14.));
        begin(cx, from);
        let floating = cx.debug_bounds("attachment-card-1-floating").unwrap();
        assert_eq!(
            floating.size, card.size,
            "The floating representation is the complete card"
        );
        cx.simulate_mouse_move(
            point(px(20.), from.y),
            Some(MouseButton::Left),
            Modifiers::default(),
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("compact-attachment-drag").is_some());
        assert!(cx.debug_bounds("attachment-card-1-floating").is_none());
        cx.simulate_mouse_move(from, Some(MouseButton::Left), Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            cx.debug_bounds("attachment-card-1-floating").unwrap().size,
            card.size
        );
        cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::default());
        cx.update(|window, cx| {
            view.update(cx, |app, _| assert_eq!(app.staged_attachments[0].id, 2));
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, cx| {
                assert_eq!(
                    app.staged_attachments
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>(),
                    vec![1, 2]
                );
                assert!(app.attachment_drag.is_none());
                assert!(!cx.has_active_drag());
                assert!(app.preview_window.is_none());
            })
        });
    }

    #[gpui_kit::test]
    fn edge_scrolling_continues_with_a_stationary_pointer(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        populate(&view, cx, 20);
        let from = cx.debug_bounds("attachment-card-1").unwrap().center();
        begin(cx, from);
        let (target, before) = cx.update(|_, cx| {
            view.update(cx, |app, _| {
                let bounds = app.main_scroll.bounds();
                (
                    point(
                        bounds.center().x,
                        bounds.origin.y + bounds.size.height - px(4.),
                    ),
                    app.main_scroll.offset().y,
                )
            })
        });
        cx.simulate_mouse_move(target, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();
        for _ in 0..6 {
            cx.update(|window, cx| {
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
        }
        cx.update(|_, cx| {
            view.update(cx, |app, cx| {
                assert!(
                    app.main_scroll.offset().y < before,
                    "before {:?} after {:?}, max {:?}, bounds {:?}, pointer {:?}, active {}",
                    before,
                    app.main_scroll.offset(),
                    app.main_scroll.max_offset(),
                    app.main_scroll.bounds(),
                    app.attachment_drag.as_ref().map(|drag| drag.pointer),
                    cx.has_active_drag()
                )
            })
        });
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();
    }
    #[gpui_kit::test]
    fn dropping_on_a_sidebar_project_moves_an_scratchpad_attachment_and_reuses_its_url(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        populate(&view, cx, 2);
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("Target");
        let target = crate::projects::Project::create_named(target, "Target")
            .unwrap()
            .folder;
        let persistent_id = cx.update(|window, cx| {
            let id = view.update(cx, |app, cx| {
                app.settings.project_directory_override = Some(directory.path().to_owned());
                app.project_state.folders = vec![target.clone()];
                app.staged_attachments[0].upload_state = AttachmentUploadState::Uploaded {
                    url: "https://example.com/reuse".into(),
                };
                cx.notify();
                app.staged_attachments[0].persistent_id
            });
            window.draw(cx).clear(cx);
            id
        });
        let from = cx.debug_bounds("attachment-card-1").unwrap().center();
        let to = cx.update(|_, cx| {
            view.update(cx, |app, _| app.project_bounds.borrow()[&target].center())
        });
        begin(cx, from);
        cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|_, cx| {
            view.update(cx, |app, _| {
                assert_eq!(app.staged_attachments.len(), 1);
                assert_eq!(app.staged_attachments[0].id, 2);
                assert!(app.attachment_drag.is_none());
            })
        });
        let (_, moved) = crate::projects::Project::open(target).unwrap();
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].persistent_id, persistent_id);
        assert_eq!(
            moved[0].upload_state.uploaded_url(),
            Some("https://example.com/reuse")
        );
    }
}
