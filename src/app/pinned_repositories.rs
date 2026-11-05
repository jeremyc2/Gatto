use super::*;
use gpui_kit::component::{
    IndexPath,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    popover::Popover,
};

const PINNED_PREVIEW: usize = 5;
const PICKER_WIDTH: f32 = 304.;

pub(super) type PickerState = ListState<PinnedRepositories>;

#[derive(Clone)]
enum Row {
    Repository(SharedString),
    ShowMore(usize),
    ShowFewer,
}

pub(super) struct PinnedRepositories {
    pinned: Vec<SharedString>,
    available: Vec<SharedString>,
    rows: [Vec<Row>; 2],
    query: String,
    expanded: bool,
    loading: LoadState,
}

impl PinnedRepositories {
    fn rebuild(&mut self) {
        let matches = |name: &&SharedString| name.to_ascii_lowercase().contains(&self.query);
        let mut pinned = self
            .pinned
            .iter()
            .filter(matches)
            .cloned()
            .map(Row::Repository)
            .collect::<Vec<_>>();
        if self.query.is_empty() && pinned.len() > PINNED_PREVIEW {
            if self.expanded {
                pinned.push(Row::ShowFewer);
            } else {
                let hidden = pinned.len() - PINNED_PREVIEW;
                pinned.truncate(PINNED_PREVIEW);
                pinned.push(Row::ShowMore(hidden));
            }
        }
        self.rows = [
            pinned,
            self.available
                .iter()
                .filter(matches)
                .cloned()
                .map(Row::Repository)
                .collect(),
        ];
    }

    fn row(&self, index: IndexPath) -> Option<&Row> {
        self.rows.get(index.section)?.get(index.row)
    }
}

impl ListDelegate for PinnedRepositories {
    type Item = ListItem;

    fn sections_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.rows[section].len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<PickerState>,
    ) -> gpui_kit::Task<()> {
        self.query = query.to_ascii_lowercase();
        self.rebuild();
        cx.notify();
        gpui_kit::Task::ready(())
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<PickerState>,
    ) -> Option<impl IntoElement> {
        (!self.rows[0].is_empty()).then(|| {
            let label = if section == 0 { "Pinned" } else { "All" };
            h_flex()
                .h(px(27.))
                .px_2()
                .text_xs()
                .font_semibold()
                .text_color(cx.theme().muted_foreground)
                .debug_selector(move || label.into())
                .child(label)
        })
    }

    fn render_item(
        &mut self,
        index: IndexPath,
        _: &mut Window,
        cx: &mut Context<PickerState>,
    ) -> Option<ListItem> {
        let row = self.row(index)?;
        let item = ListItem::new(index.row)
            .h(px(30.))
            .px_2()
            .rounded_md()
            .text_sm()
            .cursor_pointer();
        Some(match row {
            Row::Repository(name) => {
                let pinned = index.section == 0;
                let selector = format!("pin-repository-{name}");
                item.debug_selector(move || selector.clone())
                    .accessibility_label(format!(
                        "{name}, {}",
                        if pinned { "pinned" } else { "unpinned" }
                    ))
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_between()
                            .child(div().min_w_0().truncate().child(name.clone()))
                            .when(pinned, |row| {
                                row.child(
                                    Icon::new(IconName::Check)
                                        .xsmall()
                                        .flex_none()
                                        .text_color(cx.theme().link),
                                )
                            }),
                    )
            }
            Row::ShowMore(_) | Row::ShowFewer => {
                let (label, icon) = match row {
                    Row::ShowMore(hidden) => (format!("Show {hidden} more"), IconName::ChevronDown),
                    _ => ("Show fewer".into(), IconName::ChevronUp),
                };
                item.debug_selector(|| "pinned-repositories-disclosure".into())
                    .accessibility_label(label.clone())
                    .text_color(cx.theme().link)
                    .child(
                        h_flex()
                            .justify_between()
                            .child(label)
                            .child(Icon::new(icon).xsmall()),
                    )
            }
        })
    }

    fn render_empty(&mut self, _: &mut Window, cx: &mut Context<PickerState>) -> impl IntoElement {
        let message = match self.loading {
            LoadState::Loading => return div(),
            LoadState::Failed => "Could not load repositories.",
            _ if self.query.is_empty() => "No repositories available.",
            _ => "No matching repositories.",
        };
        div()
            .px_3()
            .py_4()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(message)
    }

    fn set_selected_index(
        &mut self,
        _: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<PickerState>,
    ) {
    }
}

impl UploaderApp {
    pub(super) fn new_pinned_repository_picker(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PickerState> {
        let picker = cx.new(|cx| {
            ListState::new(
                PinnedRepositories {
                    pinned: Vec::new(),
                    available: Vec::new(),
                    rows: Default::default(),
                    query: String::new(),
                    expanded: false,
                    loading: LoadState::Idle,
                },
                window,
                cx,
            )
            .searchable(true)
        });
        cx.subscribe_in(
            &picker,
            window,
            |this, picker, event: &ListEvent, window, cx| {
                let ListEvent::Confirm(index) = event else {
                    return;
                };
                picker.focus_handle(cx).focus(window, cx);
                match picker.read(cx).delegate().row(*index).cloned() {
                    Some(Row::Repository(name)) => this.toggle_repository_pin(&name, window, cx),
                    Some(row @ (Row::ShowMore(_) | Row::ShowFewer)) => {
                        picker.update(cx, |picker, cx| {
                            let delegate = picker.delegate_mut();
                            delegate.expanded = matches!(row, Row::ShowMore(_));
                            delegate.rebuild();
                            let row = delegate.rows[0].len() - 1;
                            picker.set_selected_index(Some(IndexPath::new(row)), window, cx);
                            cx.notify();
                        });
                    }
                    None => {}
                }
            },
        )
        .detach();
        picker
    }

    pub(super) fn refresh_pinned_repository_picker(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.pinned_repository_picker_open {
            return;
        }
        let pins = &self.settings.pinned_repositories;
        self.pinned_repository_picker.update(cx, |picker, cx| {
            let delegate = picker.delegate_mut();
            delegate.loading = self.load_state;
            delegate.pinned = pins.iter().cloned().map(Into::into).collect();
            delegate.available = self
                .repositories
                .keys()
                .filter(|name| !pins.contains(*name))
                .cloned()
                .map(Into::into)
                .collect();
            for names in [&mut delegate.pinned, &mut delegate.available] {
                names.sort_by_cached_key(|name| name.to_ascii_lowercase());
            }
            delegate.rebuild();
            if let Some(index) = picker.selected_index()
                && picker.delegate().row(index).is_none()
            {
                picker.set_selected_index(None, window, cx);
            }
            cx.notify();
        });
    }

    fn toggle_repository_pin(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = self.settings.clone();
        if !settings.pinned_repositories.remove(name) {
            settings.pinned_repositories.insert(name.to_owned());
        }
        if self.persist_preferences
            && let Err(error) = settings.save()
        {
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
            return;
        }
        self.settings = settings;
        self.refresh_repository_picker(window, cx);
        self.refresh_pinned_repository_picker(window, cx);
        cx.notify();
    }

    pub(super) fn render_pinned_repositories(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.settings.pinned_repositories.len();
        let label = match count {
            0 => format!(
                "Search {} repositories…",
                self.settings.owner.as_deref().unwrap_or_default()
            ),
            1 => "1 repository pinned".into(),
            _ => format!("{count} repositories pinned"),
        };
        let picker = self.pinned_repository_picker.clone();
        let focus = picker.focus_handle(cx);
        let loading = self.load_state == LoadState::Loading;
        h_flex()
            .gap_4()
            .justify_between()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .p_4()
            .child(
                v_flex()
                    .min_w_0()
                    .gap_1()
                    .child(div().font_semibold().text_sm().child("Pinned repositories"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Choose which repositories appear first when uploading."),
                    )
                    .when(self.load_state == LoadState::Failed, |heading| {
                        heading.child(
                            Button::new("retry-pin-repositories")
                                .app_secondary()
                                .small()
                                .self_start()
                                .label("Retry loading repositories")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.load_repositories(window, cx)
                                })),
                        )
                    }),
            )
            .child(
                Popover::new("pinned-repository-popover")
                    .w(px(PICKER_WIDTH))
                    .p_0()
                    .bg(cx.theme().sidebar)
                    .open(self.pinned_repository_picker_open)
                    .track_focus(&focus)
                    .on_open_change(cx.listener(|this, open: &bool, window, cx| {
                        this.pinned_repository_picker_open = *open;
                        if *open {
                            this.pinned_repository_picker.update(cx, |picker, cx| {
                                picker.delegate_mut().expanded = false;
                                if !picker.delegate().query.is_empty() {
                                    picker.set_query("", window, cx);
                                }
                                picker.set_selected_index(None, window, cx);
                                picker
                                    .scroll_handle()
                                    .scroll_to_item(0, gpui_kit::ScrollStrategy::Top);
                            });
                            if this.load_state == LoadState::Idle {
                                this.load_repositories_when_opened(window, cx);
                            } else {
                                this.refresh_pinned_repository_picker(window, cx);
                            }
                        }
                        cx.notify();
                    }))
                    .trigger(
                        Button::new("pinned-repository-trigger")
                            .debug_selector(|| "pinned-repository-trigger".into())
                            .dropdown_caret(true)
                            .accessibility_label(format!("Pinned repositories. {label}"))
                            .app_secondary()
                            .w(px(PICKER_WIDTH))
                            .justify_between()
                            .child(div().min_w_0().truncate().child(label)),
                    )
                    .content(move |_, _, cx| {
                        v_flex()
                            .child(
                                List::new(&picker)
                                    .search_placeholder("Filter repositories")
                                    .small()
                                    .max_h(px(250.))
                                    .p_1p5(),
                            )
                            .when(loading, |panel| {
                                panel.child(
                                    h_flex()
                                        .id("pinned-repository-loading")
                                        .role(gpui_kit::Role::Status)
                                        .debug_selector(|| "pinned-repository-loading".into())
                                        .gap_2()
                                        .px_3()
                                        .py_3()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(Spinner::new().small())
                                        .child("Loading repositories…"),
                                )
                            })
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Modifiers, TestAppContext, VisualTestContext};

    fn click(cx: &mut VisualTestContext, selector: &'static str) {
        let position = cx.debug_bounds(selector).unwrap().center();
        cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui_kit::test]
    fn catalog_loading_stays_visible_with_and_without_saved_pins(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Loading;
                app.settings.pinned_repositories.insert("alpha".into());
                app.show_settings(cx);
            });
            window.draw(cx).clear(cx);
        });

        click(cx, "pinned-repository-trigger");
        assert!(cx.debug_bounds("pinned-repository-loading").is_some());
        click(cx, "pin-repository-alpha");
        assert!(cx.debug_bounds("pin-repository-alpha").is_none());
        assert!(cx.debug_bounds("pinned-repository-loading").is_some());

        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.accept_repository_session(
                    github::GithubSession {
                        token: "test-token".into(),
                        repositories: vec![Repository {
                            id: 1,
                            name: "alpha".into(),
                        }],
                    },
                    window,
                    cx,
                );
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("pinned-repository-loading").is_none());
        assert!(cx.debug_bounds("pin-repository-alpha").is_some());
    }

    #[gpui_kit::test]
    fn settings_can_pin_multiple_repositories_without_changing_upload_destination(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                for (id, name) in [(1, "destination"), (2, "alpha"), (3, "beta")] {
                    app.repositories.insert(
                        name.into(),
                        Repository {
                            id,
                            name: name.into(),
                        },
                    );
                }
                app.select_repository("destination", window, cx);
                app.show_settings(cx);
            });
            window.draw(cx).clear(cx);
        });

        click(cx, "pinned-repository-trigger");
        assert!(cx.debug_bounds("All").is_none());
        click(cx, "pin-repository-alpha");
        assert!(cx.debug_bounds("All").is_some());
        click(cx, "pin-repository-beta");
        assert!(
            cx.debug_bounds("pin-repository-alpha").unwrap().top()
                < cx.debug_bounds("pin-repository-beta").unwrap().top()
        );
        cx.update(|_, cx| {
            let app = view.read(cx);
            assert!(app.pinned_repository_picker_open);
            assert_eq!(
                app.settings.pinned_repositories,
                ["alpha".into(), "beta".into()].into()
            );
            assert_eq!(
                app.selected_repository.as_ref().unwrap().name,
                "destination"
            );
        });

        cx.simulate_input("alpha");
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(
                view.read(cx)
                    .pinned_repository_picker
                    .read(cx)
                    .delegate()
                    .query,
                "alpha"
            );
            window.draw(cx).clear(cx);
        });
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        cx.update(|_, cx| {
            assert_eq!(
                view.read(cx).settings.pinned_repositories,
                ["beta".into()].into()
            );
        });
        cx.simulate_keystrokes("escape");
        cx.update(|_, cx| {
            assert!(!view.read(cx).pinned_repository_picker_open);
            assert!(view.read(cx).settings_open);
        });
    }

    #[gpui_kit::test]
    fn the_pinned_preview_can_expand_collapse_and_search_hidden_pins(cx: &mut TestAppContext) {
        let (view, cx) = super::super::tests::upload_window(cx);
        cx.update(|window, cx| {
            view.update(cx, |app, cx| {
                app.load_state = LoadState::Ready;
                app.settings.pinned_repositories = ["a", "b", "c", "d", "e", "hidden"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                app.show_settings(cx);
            });
            window.draw(cx).clear(cx);
        });

        click(cx, "pinned-repository-trigger");
        assert!(cx.debug_bounds("pin-repository-hidden").is_none());
        click(cx, "pinned-repositories-disclosure");
        assert!(cx.debug_bounds("pin-repository-hidden").is_some());
        click(cx, "pinned-repositories-disclosure");
        assert!(cx.debug_bounds("pin-repository-hidden").is_none());
        cx.simulate_input("hidden");
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        click(cx, "pin-repository-hidden");
        cx.update(|_, cx| {
            assert_eq!(view.read(cx).settings.pinned_repositories.len(), 5);
        });
    }
}
