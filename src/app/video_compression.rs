use super::*;
use crate::compression::CompressionMode;
use gpui_kit::component::popover::Popover;

impl UploaderApp {
    pub(super) fn set_video_compression(
        &mut self,
        mode: CompressionMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.uploading || (self.import_busy() && self.compression.is_none()) {
            return;
        }
        let previous = self.settings.video_compression;
        self.settings.video_compression = mode;
        if self.persist_preferences
            && let Err(error) = self.settings.save()
        {
            self.settings.video_compression = previous;
            window.push_notification(
                Notification::error(format!("Could not save preferences: {error}")),
                cx,
            );
        }
        cx.notify();
    }

    pub(super) fn render_video_compression(&self) -> impl IntoElement {
        let mode = self.settings.video_compression;
        h_flex()
            .gap_3()
            .justify_between()
            .items_center()
            .child(div().text_sm().child("Compression"))
            .child(
                Button::new("video-compression")
                    .app_secondary()
                    .label(mode.label())
                    .icon(IconName::ChevronDown)
                    .accessibility_label("Video compression mode")
                    .disabled(self.uploading || (self.import_busy() && self.compression.is_none()))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            CompressionMode::Quality.label(),
                            mode == CompressionMode::Quality,
                            Box::new(SelectQualityCompression),
                        )
                        .menu_with_check(
                            CompressionMode::SmallerFiles.label(),
                            mode == CompressionMode::SmallerFiles,
                            Box::new(SelectSmallerCompression),
                        )
                        .menu_with_check(
                            CompressionMode::Fast.label(),
                            mode == CompressionMode::Fast,
                            Box::new(SelectFastCompression),
                        )
                    }),
            )
    }

    pub(super) fn video_upload_info(&self) -> Popover {
        use gpui_kit::component::button::ButtonVariants as _;
        Popover::new("compression-comparison")
            .w(px(520.))
            .trigger(
                Button::new("video-upload-info")
                    .ghost()
                    .xsmall()
                    .size(px(20.))
                    .icon(AssetIconName::Info)
                    .accessibility_label("About video uploads and compression"),
            )
            .content(|_, _, cx| {
                if cfg!(windows) {
                    return v_flex().id("compression-help-content").gap_2().text_xs()
                        .child("Windows compresses oversized H.264 MP4 video in software and keeps AAC audio if present.")
                        .child("100 MB video uploads may not be supported on all GitHub plans.");
                }
                let mut table = v_flex()
                    .id("compression-help-content")
                    .gap_3()
                    .text_xs()
                    .child(
                        div()
                            .font_semibold()
                            .text_sm()
                            .child("Compression comparison"),
                    );
                for (mode, quality, time, size) in [
                    ("Mode", "Screen detail", "Time", "File size"),
                    ("Fast · H.264", "Baseline", "1×", "Baseline"),
                    ("Quality · H.264", "Highest", "~1.8×", "~4% smaller"),
                    ("Smaller · AV1", "High", "~2.1×", "~19% smaller"),
                ] {
                    table = table.child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().w(px(148.)).flex_none().child(mode))
                            .child(div().flex_1().min_w_0().child(quality))
                            .child(div().w(px(56.)).flex_none().child(time))
                            .child(div().w(px(100.)).flex_none().child(size)),
                    );
                }
                table
                    .child(div().h(px(1.)).bg(cx.theme().border))
                    .child("100 MB video uploads may not be supported on all GitHub plans.")
            })
    }
}
