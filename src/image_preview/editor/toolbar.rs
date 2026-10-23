use super::*;
use crate::annotation::{
    ARROW_TEXT_SIZES, Align, COLORS, Cap, Dash, Fill, LABEL_SIZES, SHAPES, STROKE_WIDTHS,
    TEXT_SIZES, fill_icon, pixelate_icon, scribble_icon, shape_icon,
};
use gpui_kit::assets::IconName as TextIcon;
use gpui_kit::component::{Selectable as _, v_flex};
use gpui_kit::{AnyElement, rgb};

const PANEL_WIDTH: f32 = 160.;
const SLIDER_RADIUS: f32 = 9.;
pub(super) const MIN_PIXELATE_DIAMETER: f32 = 30.;
pub(super) const MAX_PIXELATE_DIAMETER: f32 = 138.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SliderControl {
    Opacity,
    PixelateSize,
}
const COLOR_NAMES: [&str; 12] = [
    "White",
    "Grey",
    "Light violet",
    "Violet",
    "Blue",
    "Light blue",
    "Yellow",
    "Orange",
    "Green",
    "Light green",
    "Light red",
    "Red",
];

impl ImagePreview {
    pub(in crate::image_preview) fn annotation_toolbar(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tools = [
            (Tool::Select, Some(IconName::MousePointer2), "Select · V"),
            (Tool::Pan, Some(IconName::Hand), "Hand · H / hold Space"),
            (Tool::Pen, None, "Pen · P"),
            (Tool::Eraser, Some(IconName::Eraser), "Eraser · E"),
            (Tool::Arrow, Some(IconName::ArrowUpRight), "Arrow · A"),
            (Tool::Text, Some(IconName::Type), "Text · T"),
            (
                Tool::Marker,
                Some(IconName::ListOrdered),
                "Numbered marker · N",
            ),
        ];
        div()
            .absolute()
            .bottom(px(8. + self.marker_notes_height()))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                v_flex()
                    .relative()
                    .on_mouse_move(cx.listener(Self::editor_controls_hover))
                    .gap_0()
                    .items_end()
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .child(
                                h_flex()
                                    .id("annotation-history-bar")
                                    .test_support()
                                    .ml(px(10.))
                                    .gap_0p5()
                                    .p_1()
                                    .rounded_t_lg()
                                    .bg(rgb(0x19191c))
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .child(
                                        Button::new("annotation-undo")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Undo2)
                                            .disabled(!self.doc().is_some_and(Document::can_undo))
                                            .tooltip("Undo · ⌘Z")
                                            .accessibility_label("Undo")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.history(false, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("annotation-redo")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Redo2)
                                            .disabled(!self.doc().is_some_and(Document::can_redo))
                                            .tooltip("Redo · ⇧⌘Z")
                                            .accessibility_label("Redo")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.history(true, cx)
                                                }),
                                            ),
                                    )
                                    .child(
                                        Button::new("annotation-delete")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Trash)
                                            .disabled(self.editor.selected.is_none())
                                            .tooltip("Delete selection")
                                            .accessibility_label("Delete selection")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.delete_selected(cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("annotation-duplicate")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Copy)
                                            .disabled(self.editor.selected.is_none())
                                            .tooltip("Clone")
                                            .accessibility_label("Clone")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.clone_selected(cx)
                                            })),
                                    ),
                            )
                            .child(
                                Button::new("annotation-lock")
                                    .ghost()
                                    .small()
                                    .icon(if self.editor.locked {
                                        IconName::Lock
                                    } else {
                                        IconName::LockOpen
                                    })
                                    .selected(self.editor.locked)
                                    .tooltip("Keep tool active · L")
                                    .accessibility_label("Keep tool active")
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.editor.locked = !this.editor.locked;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("annotation-tool-bar")
                            .test_support()
                            .gap_0p5()
                            .p_1()
                            .rounded_lg()
                            .bg(rgb(0x202024))
                            .border_1()
                            .border_color(rgb(0x383840))
                            .shadow_lg()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                            .children(tools.into_iter().map(|(tool, icon, label)| {
                                Button::new(("annotation-tool", tool_id(tool)))
                                    .ghost()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded_md()
                                    .when_some(icon, |button, icon| button.icon(icon))
                                    .when(tool == Tool::Pen, |button| {
                                        button.child(scribble_icon(if self.editor.tool == tool {
                                            rgb(0xffffff).into()
                                        } else {
                                            cx.theme().foreground
                                        }))
                                    })
                                    .selected(self.editor.tool == tool)
                                    .when(self.editor.tool == tool, |button| {
                                        button.bg(rgb(0x4387f5)).text_color(rgb(0xffffff))
                                    })
                                    .tooltip(label)
                                    .accessibility_label(label)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.select_tool(tool, window, cx)
                                    }))
                            }))
                            .children([Tool::Rectangle, Tool::Ellipse].into_iter().map(|tool| {
                                Button::new(("annotation-tool", tool_id(tool)))
                                    .ghost()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded_md()
                                    .child(shape_icon(
                                        tool,
                                        if self.editor.tool == tool {
                                            rgb(0xffffff).into()
                                        } else {
                                            cx.theme().foreground
                                        },
                                    ))
                                    .selected(self.editor.tool == tool)
                                    .when(self.editor.tool == tool, |button| {
                                        button.bg(rgb(0x4387f5))
                                    })
                                    .tooltip(if tool == Tool::Rectangle {
                                        "Rectangle · R"
                                    } else {
                                        "Ellipse · O"
                                    })
                                    .accessibility_label(shape_name(tool))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.select_tool(tool, window, cx)
                                    }))
                            }))
                            .child(
                                Button::new("annotation-shapes-overflow")
                                    .ghost()
                                    .w(px(32.))
                                    .h(px(36.))
                                    .icon(IconName::ChevronUp)
                                    .selected(self.editor.shapes_open == Some(ShapeMenu::Toolbar))
                                    .tooltip("More shapes")
                                    .accessibility_label("More shapes")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.editor.shapes_open = if this.editor.shapes_open
                                            == Some(ShapeMenu::Toolbar)
                                        {
                                            None
                                        } else {
                                            Some(ShapeMenu::Toolbar)
                                        };
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new(("annotation-tool", 8usize))
                                    .ghost()
                                    .w(px(36.))
                                    .h(px(36.))
                                    .rounded_md()
                                    .child(pixelate_icon())
                                    .selected(self.editor.tool == Tool::Redact)
                                    .when(self.editor.tool == Tool::Redact, |button| {
                                        button.bg(rgb(0x4387f5))
                                    })
                                    .tooltip("Pixelate image · X")
                                    .accessibility_label("Pixelate image")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.select_tool(Tool::Redact, window, cx)
                                    })),
                            ),
                    )
                    .when(self.editor.shapes_open == Some(ShapeMenu::Toolbar), |bar| {
                        bar.child(self.shape_picker(ShapeMenu::Toolbar, cx))
                    }),
            )
    }
    fn color_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_0p5().p_1().children((0..3).map(|row| {
            h_flex()
                .gap_0p5()
                .children((0..4).map(move |column| row * 4 + column).map(|i| {
                    let c = COLORS[i];
                    Button::new(("annotation-color", i))
                        .ghost()
                        .w(px(36.))
                        .h(px(32.))
                        .rounded_md()
                        .selected(self.editor.style.color == i)
                        .when(self.editor.style.color == i, |button| {
                            button.bg(rgb(0x333337))
                        })
                        .child(
                            div()
                                .size(px(16.))
                                .rounded_full()
                                .bg(rgb((u32::from(c[0]) << 16)
                                    | (u32::from(c[1]) << 8)
                                    | u32::from(c[2]))),
                        )
                        .tooltip(COLOR_NAMES[i])
                        .accessibility_label(COLOR_NAMES[i])
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.change_style(|s| s.color = i, cx);
                            this.refocus_text(window, cx);
                        }))
                }))
        }))
    }
    pub(in crate::image_preview) fn annotation_properties(
        &self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut tool = self
            .editor
            .selected
            .and_then(|i| self.doc()?.marks.get(i).map(|mark| mark.tool))
            .unwrap_or(self.editor.tool);
        let indices = self.selected_indices();
        if indices.len() > 1 {
            let doc = self.doc().unwrap();
            if indices.iter().all(|i| doc.marks[*i].tool == Tool::Text) {
                tool = Tool::Text;
            } else if indices.iter().all(|i| doc.marks[*i].tool.is_shape()) {
                tool = Tool::Rectangle;
            } else if !indices.iter().all(|i| doc.marks[*i].tool == tool) {
                tool = Tool::Pen;
            }
        }
        if tool == Tool::Marker {
            return panel()
                .absolute()
                .right_2()
                .top_2()
                .w(px(PANEL_WIDTH))
                .on_mouse_move(cx.listener(Self::editor_controls_hover))
                .child(self.color_palette(cx))
                .into_any_element();
        }
        let styled = tool.is_geometry() || matches!(tool, Tool::Text | Tool::Arrow | Tool::Pen);
        if tool == Tool::Redact {
            return panel()
                .absolute()
                .right_2()
                .top_2()
                .w(px(PANEL_WIDTH))
                .on_mouse_move(cx.listener(Self::editor_controls_hover))
                .child(
                    h_flex()
                        .items_center()
                        .px_2()
                        .gap_2()
                        .child(
                            div()
                                .size(px(3.))
                                .flex_none()
                                .rounded_full()
                                .bg(rgb(0xd0d0d5)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(self.style_slider(SliderControl::PixelateSize, cx)),
                        )
                        .child(
                            div()
                                .size(px(12.))
                                .flex_none()
                                .rounded_full()
                                .border_1()
                                .border_color(rgb(0xd0d0d5)),
                        ),
                )
                .into_any_element();
        }
        if !styled {
            return div().into_any_element();
        }
        let s = &self.editor.style;
        let shape = tool.is_shape();
        let text = tool == Tool::Text;
        let fonts = if text {
            TEXT_SIZES
        } else if matches!(tool, Tool::Arrow | Tool::Line) {
            ARROW_TEXT_SIZES
        } else {
            LABEL_SIZES
        };
        let sizes: [(&str, f32, f32); 4] =
            std::array::from_fn(|i| (["S", "M", "L", "XL"][i], fonts[i], STROKE_WIDTHS[i]));
        // Continuous text resizing (or a custom stroke) can fall between presets.
        // Highlight the closest choice without changing the object's actual size.
        let selected_size = sizes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let distance = |size: &(_, f32, f32)| {
                    if text {
                        (s.font_size - size.1).abs()
                    } else {
                        (s.width - size.2).abs()
                    }
                };
                distance(a).total_cmp(&distance(b))
            })
            .unwrap()
            .0;
        let panel = panel()
            .on_mouse_move(cx.listener(Self::editor_controls_hover))
            .absolute()
            .right_2()
            .top_2()
            .w(px(PANEL_WIDTH))
            .child(self.color_palette(cx))
            .child(
                div()
                    .mx_2()
                    .child(self.style_slider(SliderControl::Opacity, cx)),
            )
            .when(shape, |panel| {
                panel.child(rule()).child(
                    h_flex().p_1().gap_0p5().children(
                        [
                            (Fill::None, "No fill"),
                            (Fill::Soft, "Soft fill"),
                            (Fill::Solid, "Solid fill"),
                            (Fill::Hatch, "Pattern fill"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (fill, label))| {
                            Button::new(("annotation-fill", i))
                                .ghost()
                                .w(px(36.))
                                .h(px(32.))
                                .p_0()
                                .rounded_md()
                                .selected(s.fill == fill)
                                .when(s.fill == fill, |button| button.bg(rgb(0x333337)))
                                .tooltip(label)
                                .accessibility_label(label)
                                .child(fill_icon(fill, cx.theme().foreground))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.change_style(|s| s.fill = fill, cx);
                                    this.refocus_text(window, cx);
                                }))
                        }),
                    ),
                )
            })
            .when(!text, |panel| {
                panel.child(
                    h_flex().p_1().gap_0p5().children(
                        [
                            (Dash::Solid, IconName::Circle, "Solid"),
                            (Dash::Dashed, IconName::CircleDashed, "Dashed"),
                            (Dash::Dotted, IconName::CircleDotDashed, "Dotted"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (dash, icon, label))| {
                            option(("annotation-dash", i), icon, label, s.dash == dash).on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.change_style(|s| s.dash = dash, cx);
                                    this.refocus_text(window, cx);
                                }),
                            )
                        }),
                    ),
                )
            })
            .child(
                h_flex()
                    .p_1()
                    .gap_0p5()
                    .children(
                        sizes
                            .into_iter()
                            .enumerate()
                            .map(|(i, (label, font, width))| {
                                Button::new(("annotation-size", i))
                                    .ghost()
                                    .p_0()
                                    .w(px(36.))
                                    .h(px(32.))
                                    .rounded_md()
                                    .label(label)
                                    .selected(i == selected_size)
                                    .toggled(i == selected_size)
                                    .when(i == selected_size, |button| button.bg(rgb(0x333337)))
                                    .tooltip(if text {
                                        format!("Text size {label} · drag corners for any size")
                                    } else {
                                        format!("Stroke size {label}")
                                    })
                                    .accessibility_label(format!("Size {label}"))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.change_style(
                                            |s| {
                                                s.font_size = font;
                                                s.width = width;
                                            },
                                            cx,
                                        );
                                        this.refocus_text(window, cx);
                                    }))
                            }),
                    ),
            )
            .when(
                text || shape || matches!(tool, Tool::Arrow | Tool::Line),
                |panel| {
                    let align =
                        s.text_align
                            .unwrap_or(if text { Align::Start } else { Align::Middle });
                    panel.child(rule()).child(
                        h_flex().p_1().justify_between().children(
                            [
                                (Align::Start, TextIcon::TextAlignStart, "Align text left"),
                                (
                                    Align::Middle,
                                    TextIcon::TextAlignCenter,
                                    "Align text center",
                                ),
                                (Align::End, TextIcon::TextAlignEnd, "Align text right"),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(i, (value, icon, label))| {
                                Button::new(("annotation-text-align", i))
                                    .ghost()
                                    .w(px(36.))
                                    .h(px(32.))
                                    .icon(icon)
                                    .selected(align == value)
                                    .when(align == value, |b| b.bg(rgb(0x333337)))
                                    .tooltip(label)
                                    .accessibility_label(label)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.change_style(|s| s.text_align = Some(value), cx);
                                        this.refocus_text(window, cx);
                                    }))
                            }),
                        ),
                    )
                },
            )
            .when(shape, |panel| {
                panel.child(
                    h_flex().p_1().justify_between().children(
                        [
                            (
                                Align::Start,
                                TextIcon::AlignVerticalJustifyStart,
                                "Align text top",
                            ),
                            (
                                Align::Middle,
                                TextIcon::AlignVerticalJustifyCenter,
                                "Align text middle",
                            ),
                            (
                                Align::End,
                                TextIcon::AlignVerticalJustifyEnd,
                                "Align text bottom",
                            ),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (value, icon, label))| {
                            Button::new(("annotation-text-vertical", i))
                                .ghost()
                                .w(px(36.))
                                .h(px(32.))
                                .icon(icon)
                                .selected(s.vertical_align == value)
                                .when(s.vertical_align == value, |b| b.bg(rgb(0x333337)))
                                .tooltip(label)
                                .accessibility_label(label)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.change_style(|s| s.vertical_align = value, cx);
                                    this.refocus_text(window, cx);
                                }))
                        }),
                    ),
                )
            })
            .when(tool == Tool::Arrow, |panel| {
                panel.child(rule()).child(
                    h_flex()
                        .p_1()
                        .justify_between()
                        .child(
                            Button::new("annotation-start-cap")
                                .ghost()
                                .small()
                                .icon(cap_icon(s.start))
                                .tooltip(format!("Start: {}", cap_name(s.start)))
                                .accessibility_label("Start cap")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.change_style(|s| s.start = s.start.next(), cx);
                                    this.refocus_text(window, cx);
                                })),
                        )
                        .child(
                            Button::new("annotation-end-cap")
                                .ghost()
                                .small()
                                .icon(cap_icon(s.end))
                                .tooltip(format!("End: {}", cap_name(s.end)))
                                .accessibility_label("End cap")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.change_style(|s| s.end = s.end.next(), cx);
                                    this.refocus_text(window, cx);
                                })),
                        ),
                )
            })
            .when(tool.is_geometry(), |panel| {
                panel.child(rule()).child(
                    div()
                        .relative()
                        .child(
                            Button::new("annotation-shape-picker")
                                .ghost()
                                .w_full()
                                .h(px(36.))
                                .child(
                                    h_flex()
                                        .w_full()
                                        .justify_between()
                                        .child(div().text_sm().child("Shape"))
                                        .child(shape_icon(tool, cx.theme().foreground)),
                                )
                                .accessibility_label("Choose shape")
                                .tooltip("Choose shape")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.editor.shapes_open =
                                        if this.editor.shapes_open == Some(ShapeMenu::Properties) {
                                            None
                                        } else {
                                            Some(ShapeMenu::Properties)
                                        };
                                    cx.notify();
                                })),
                        )
                        .when(
                            self.editor.shapes_open == Some(ShapeMenu::Properties),
                            |anchor| {
                                let rows = SHAPES.len().div_ceil(4) as f32;
                                let menu_height = rows * 32. + (rows - 1.) * 2. + 10.;
                                anchor.child(
                                    div()
                                        .absolute()
                                        .left(px(-PANEL_WIDTH - 8.))
                                        .top(px((36. - menu_height) / 2.))
                                        .child(
                                            gpui_kit::anchored()
                                                .snap_to_window_with_margin(gpui_kit::Edges {
                                                    top: px(TOOLBAR_HEIGHT + 8.),
                                                    right: px(8.),
                                                    bottom: px(8.),
                                                    left: px(8.),
                                                })
                                                .child(
                                                    self.shape_picker(ShapeMenu::Properties, cx),
                                                ),
                                        ),
                                )
                            },
                        ),
                )
            });
        div().absolute().inset_0().child(panel).into_any_element()
    }
    fn shape_picker(&self, origin: ShapeMenu, cx: &mut Context<Self>) -> impl IntoElement {
        panel()
            .id("annotation-shape-menu")
            .test_support()
            .when(origin == ShapeMenu::Toolbar, |panel| {
                panel.absolute().right(px(40.)).bottom(px(52.))
            })
            .w(px(PANEL_WIDTH))
            .p_1()
            .gap_0p5()
            .children(SHAPES.chunks(4).enumerate().map(|(row, tools)| {
                h_flex()
                    .gap_0p5()
                    .children(tools.iter().copied().enumerate().map(|(col, tool)| {
                        Button::new(("annotation-shape", row * 4 + col))
                            .ghost()
                            .w(px(36.))
                            .h(px(32.))
                            .child(shape_icon(tool, cx.theme().foreground))
                            .selected(
                                self.editor
                                    .selected
                                    .and_then(|i| self.doc()?.marks.get(i).map(|mark| mark.tool))
                                    .unwrap_or(self.editor.tool)
                                    == tool,
                            )
                            .tooltip(shape_name(tool))
                            .accessibility_label(shape_name(tool))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if origin == ShapeMenu::Properties && this.editor.selected.is_some()
                                {
                                    this.finish_text(cx);
                                    let indices = this.selected_indices();
                                    let doc = this.doc_mut().unwrap();
                                    doc.checkpoint();
                                    for index in indices {
                                        let mark = &mut doc.marks[index];
                                        if !mark.tool.is_geometry() {
                                            continue;
                                        }
                                        let frame = mark.frame();
                                        let bounds = mark.bounds();
                                        mark.points = vec![
                                            Pos::new(bounds.x, bounds.y),
                                            Pos::new(bounds.x + bounds.w, bounds.y + bounds.h),
                                        ];
                                        mark.tool = tool;
                                        if tool == Tool::Line {
                                            mark.points = mark
                                                .points
                                                .iter()
                                                .map(|p| frame.to_world(*p))
                                                .collect();
                                            mark.rotation = 0.;
                                            mark.style.start = Cap::None;
                                            mark.style.end = Cap::None;
                                        }
                                    }
                                    this.editor.shapes_open = None;
                                    this.editor.group_frame = None;
                                    this.refresh_annotations(cx);
                                } else {
                                    this.select_tool(tool, window, cx);
                                }
                            }))
                    }))
            }))
    }
    fn style_slider(&self, control: SliderControl, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = Rc::new(std::cell::Cell::new(None::<Bounds<Pixels>>));
        let paint_bounds = bounds.clone();
        let down_bounds = bounds.clone();
        let move_bounds = bounds.clone();
        let value = match control {
            SliderControl::Opacity => self.editor.style.opacity,
            // More travel for small brushes, while still offering a broad upper range.
            SliderControl::PixelateSize => ((self.pixelate_width() - MIN_PIXELATE_DIAMETER)
                / (MAX_PIXELATE_DIAMETER - MIN_PIXELATE_DIAMETER))
                .sqrt(),
        };
        div()
            .id(match control {
                SliderControl::Opacity => "annotation-opacity",
                SliderControl::PixelateSize => "pixelate-size",
            })
            .test_support()
            .h(px(36.))
            .w_full()
            .relative()
            .cursor(CursorStyle::PointingHand)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.editor.slider_dragging = Some(control);
                    if control == SliderControl::Opacity
                        && !this.selected_indices().is_empty()
                        && let Some(doc) = this.doc_mut()
                    {
                        doc.checkpoint();
                    }
                    this.set_slider(control, event.position, down_bounds.get(), cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                if this.editor.slider_dragging == Some(control) && event.dragging() {
                    this.set_slider(control, event.position, move_bounds.get(), cx);
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.editor.slider_dragging = None),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.editor.slider_dragging = None),
            )
            .child(
                canvas(
                    move |bounds, _, _| {
                        paint_bounds.set(Some(bounds));
                    },
                    move |bounds, _, window, _| {
                        let x = bounds.origin.x + px(SLIDER_RADIUS);
                        let width = bounds.size.width - px(SLIDER_RADIUS * 2.);
                        let y = bounds.origin.y + bounds.size.height / 2.;
                        window.paint_quad(gpui_kit::fill(
                            Bounds::new(point(x, y - px(1.5)), size(width, px(3.))),
                            rgb(0x43434c),
                        ));
                        window.paint_quad(gpui_kit::fill(
                            Bounds::new(point(x, y - px(1.5)), size(width * value, px(3.))),
                            rgb(0x5798ff),
                        ));
                        let thumb = Bounds::new(
                            point(x + width * value - px(SLIDER_RADIUS), y - px(SLIDER_RADIUS)),
                            size(px(SLIDER_RADIUS * 2.), px(SLIDER_RADIUS * 2.)),
                        );
                        window.paint_quad(gpui_kit::quad(
                            thumb,
                            px(SLIDER_RADIUS),
                            rgb(0x202024),
                            px(2.),
                            rgb(0xd0d0d5),
                            gpui_kit::BorderStyle::Solid,
                        ));
                    },
                )
                .size_full(),
            )
    }
    fn set_slider(
        &mut self,
        control: SliderControl,
        p: Point<Pixels>,
        bounds: Option<Bounds<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        let Some(bounds) = bounds else {
            return;
        };
        let value = ((p.x - bounds.origin.x - px(SLIDER_RADIUS)).as_f32()
            / (bounds.size.width.as_f32() - SLIDER_RADIUS * 2.))
            .clamp(0., 1.);
        if control == SliderControl::PixelateSize {
            self.editor.pixelate_diameter = Some(
                MIN_PIXELATE_DIAMETER
                    + (MAX_PIXELATE_DIAMETER - MIN_PIXELATE_DIAMETER) * value * value,
            );
            cx.notify();
            return;
        }
        self.editor.style.opacity = value;
        let indices = self.selected_indices();
        if let Some(doc) = self.doc_mut() {
            for index in indices {
                doc.marks[index].style.opacity = value;
            }
        }
        self.refresh_annotations(cx);
    }
    pub(in crate::image_preview) fn annotation_actions(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .child(
                Button::new("annotation-revert")
                    .app_secondary()
                    .w(px(88.))
                    .label("Discard")
                    .disabled(self.saving.is_some() || !self.can_discard())
                    .on_click(cx.listener(|this, _, window, cx| this.confirm_revert(window, cx))),
            )
            .child(
                Button::new("annotation-apply")
                    .app_primary()
                    .w(px(120.))
                    .label(if self.saving.is_some() {
                        "Saving…"
                    } else {
                        "Save"
                    })
                    .when(self.saving.is_some(), |button| {
                        button.icon(IconName::Loader)
                    })
                    .loading(self.saving.is_some())
                    .disabled(!self.has_unsaved_changes())
                    .on_click(cx.listener(|this, _, window, cx| this.apply_annotation(window, cx))),
            )
    }
}
fn panel() -> gpui_kit::Div {
    v_flex()
        .rounded_lg()
        .bg(rgb(0x202024))
        .border_1()
        .border_color(rgb(0x383840))
        .shadow_lg()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
}
fn rule() -> gpui_kit::Div {
    div().h(px(1.)).w_full().bg(rgb(0x35353e))
}
fn option(
    id: (&'static str, usize),
    icon: IconName,
    label: &'static str,
    selected: bool,
) -> Button {
    Button::new(id)
        .ghost()
        .w(px(36.))
        .h(px(32.))
        .icon(icon)
        .selected(selected)
        .when(selected, |button| button.bg(rgb(0x333337)))
        .tooltip(label)
        .accessibility_label(label)
}
fn tool_id(tool: Tool) -> usize {
    match tool {
        Tool::Select => 0,
        Tool::Pan => 1,
        Tool::Text => 2,
        Tool::Marker => 10,
        Tool::Rectangle => 3,
        Tool::Ellipse => 4,
        Tool::Arrow => 6,
        Tool::Pen => 7,
        Tool::Redact => 8,
        Tool::Eraser => 9,
        _ => 3,
    }
}
fn cap_icon(cap: Cap) -> IconName {
    match cap {
        Cap::None => IconName::Minus,
        Cap::Arrow => IconName::ArrowRight,
        Cap::Triangle => IconName::Triangle,
        Cap::Circle => IconName::Circle,
    }
}
fn cap_name(cap: Cap) -> &'static str {
    match cap {
        Cap::None => "none",
        Cap::Arrow => "open arrow",
        Cap::Triangle => "triangle",
        Cap::Circle => "circle",
    }
}
fn shape_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Rectangle => "Rectangle",
        Tool::Ellipse => "Ellipse",
        Tool::Triangle => "Triangle",
        Tool::Diamond => "Diamond",
        Tool::Star => "Star",
        Tool::Hexagon => "Hexagon",
        Tool::Rhombus => "Rhombus",
        Tool::Oval => "Oval",
        Tool::ArrowLeft => "Left arrow",
        Tool::ArrowUp => "Up arrow",
        Tool::ArrowDown => "Down arrow",
        Tool::ArrowRight => "Right arrow",
        Tool::Cloud => "Cloud",
        Tool::XBox => "Crossed box",
        Tool::CheckBox => "Checked box",
        Tool::Heart => "Heart",
        Tool::Line => "Line · K",
        _ => "Shape",
    }
}
