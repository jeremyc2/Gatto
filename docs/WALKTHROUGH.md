# Gatto walkthrough

[Overview](../README.md) · [Setup](SETUP.md) · [Support](../SUPPORT.md)

Start with [setup](SETUP.md) to install Gatto and connect your account or organization.
The main window's question-mark icon opens this walkthrough in your browser.

## 1. Choose a repository

Open Gatto from its menu bar icon and choose the repository to upload through.
For frequent destinations, [pin a repository in App Preferences](SETUP.md#3-choose-your-account-or-organization).

In App Preferences, press **Escape** or click **Done** to return to the main page.
Escape also works from a focused text field or toolbar control. If a dialog is
open, Escape dismisses the dialog first. While recording a global shortcut,
**Escape** cancels recording first; press it again to return to the main page.

## 2. Stage an image or video

Return to the main window and paste an image, drag a file into the drop zone, or
click the drop zone to select a file. Gatto accepts PNG, JPEG, GIF, and WebP
images, plus MP4, MOV, and WebM videos. You can also copy an image or video file
in Finder and paste it into Gatto; a text URL is not a video clipboard payload.
Unsupported file types produce an error notification when selected, dropped, or
pasted.

By default, each file must be at most **10 MB** (10,000,000 bytes), so the video
limit works with Free and paid GitHub plans. Eligible paid-plan users can enable
**Allow videos up to 100 MB** in [App Preferences](SETUP.md#4-set-your-preferences).
This raises only the video limit to 100,000,000 bytes; images and GIFs stay at
10 MB. Gatto does not verify eligibility. For videos above the selected limit,
Gatto offers **Cancel** or **Compress** before staging.
Gatto checks for empty, unreadable, oversized, and invalid video containers
before uploading. H.264 MP4 offers the widest playback compatibility.

<img src="images/walkthrough/06-video-staged.png" alt="Gatto with a video ready to upload and a play tile for opening its macOS player" width="620">

<img src="images/walkthrough/03-image-staged.png" alt="Gatto with its rocket cat artwork staged and ready to upload" width="620">

The attachment stays in Gatto while it uploads. You can edit its description, remove it, or preview it before sending.

### Compress a larger video

When a selected video exceeds your 10 MB or 100 MB limit, choose **Compress** to create a smaller
H.264 MP4 using macOS's hardware encoder, or **Cancel** to skip that video.
Compression runs locally, preserves its original pixel dimensions and
orientation, and leaves the source file unchanged. Existing audio becomes
mono AAC at 64 kbps; silent clips stay silent. Gatto caps video at about 30 fps
while preserving source timing. Visual quality can decrease.

<img src="images/walkthrough/08-video-too-large.png" alt="Gatto offers Cancel or Compress for an oversized video" width="620">

The progress panel shows the compression pass and percentage. Use **Cancel
compression** to stop that file. Gatto stages a compressed copy only after the
MP4 finishes, its dimensions are verified, and its actual size is below the selected limit.
You still choose **Upload** separately. In a batch, the choices are offered in
file order; canceling skips that file and preserves the order of the others.

<img src="images/walkthrough/09-video-compressing.png" alt="Gatto's local video compression progress and cancellation control" width="620">

Bitrate depends on duration so shorter clips can keep more detail. Gatto can
retry once at a lower bitrate if the first completed file is too large. A
long or complex video may still need trimming. macOS decoder/encoder support
is required; some WebM codecs, HDR recordings, multiple media tracks, and
unsupported dimensions need an SDR H.264 MP4 export first. No extra encoder
application is required. Quick Copy reports oversized videos and directs you
to the main window to choose compression; it does not compress without consent.

### Record a clear demonstration

For best results:

- **Keep the recording under 15 seconds.** Show one interaction or bug per clip.
- Record the relevant app or window. Keep text large enough to read without
  needing the viewer to enlarge the player.
- Start just before the action and pause briefly on the result. Trim setup,
  waiting, repeated attempts, and long stretches of scrolling.
- Move the pointer deliberately and avoid rapid zooming or panning. Mostly
  static screens compress well while keeping text easier to read.
- Include audio only when it helps explain the action. Keep speech short and
  background noise low; remove notifications and private information before
  recording.
- Prefer H.264 MP4 for broad playback compatibility. Gatto also accepts MOV
  and WebM. If the video exceeds your selected limit, choose **Compress** when importing it;
  trim the clip if compression cannot fit it at the original resolution.

Compression keeps the original pixel dimensions but can soften text or motion.
A short, focused recording generally gives a clearer result than a long clip
compressed into the same size limit.

## 3. Upload and copy the Markdown

Click **Upload**. After GitHub accepts the attachment, its status changes to **Uploaded** and the attachment gains a copy button.

<img src="images/walkthrough/04-upload-complete.png" alt="Gatto showing an uploaded image with a Markdown copy action" width="620">

Normal uploads leave the clipboard unchanged. Click an attachment’s copy button
to copy its snippet. Press **Command+C** to copy every successful upload in
attachment order, separated by blank lines.
For a single upload, the shortcut copies just that attachment; its hint appears after
the first success. Paste the Markdown into an issue, pull request, comment,
discussion, or Markdown file. By default, images copy in this form:

```md
<img width="640" height="480" alt="Gatto rocket cat" src="https://github.com/user-attachments/assets/…" />
```

The copied snippet is an HTML image element supported in GitHub Markdown. Its
width and height use the image's pixel dimensions (the example above is 640 ×
480); GitHub fits larger images to the available space. Gatto does not recover
the display size of an image copied from another app or a Retina scale factor.
To use Markdown image syntax instead, enable **Copy images as Markdown** in
[App Preferences](SETUP.md#4-set-your-preferences). The checkbox is unchecked by
default. With it enabled, the same image copies as:

```md
![Gatto rocket cat](https://github.com/user-attachments/assets/…)
```

Markdown snippets keep the description as alt text but omit width and height.
Changing the preference also changes the next copy of images already uploaded
in the current window; you do not need to upload them again. Individual copy,
Quick Copy, and bulk copy use the selected format. Videos always copy as a bare
attachment URL, which GitHub turns into a player in a PR description:

```md
https://github.com/user-attachments/assets/…
```

Bulk copy separates every attachment with a blank line so videos can render in
their own paragraphs. Video descriptions stay in Gatto; the copied video URL
has no image alt-text field.

## Bulk uploads and retries

Enable **Bulk upload** before selecting several files in the file picker. Dropping
or pasting several files into an empty draft enables bulk upload automatically.
You can add more images or videos to the same batch by dropping files or pasting
again. In single-attachment mode, adding more files opens a replacement dialog;
choose **Switch to bulk upload** (or press **B** in that dialog) to keep the existing
attachment and add every new file in order. **Cancel** keeps the current attachment;
**Replace attachment** replaces it with the first new file.
The bulk toggle is disabled during uploads and while more than one attachment
remains; remove attachments until at most one remains to return to single-attachment mode.

**Upload** sends each pending or failed attachment in order. Each attachment shows
its own status. If part of a batch fails, successful uploads remain available to
copy. Click **Upload** again to retry pending or failed attachments; already uploaded
attachments are skipped. **Copy all** uses the same completed results as **Command+C**.
Removing an attachment removes it from Gatto’s current list; it does not delete
the attachment from GitHub.

The 100 MB preference is unavailable during uploads or compression. If you
switch it off with a larger video already staged, Gatto blocks that upload.
Remove the video and add it again to choose compression below 10 MB. Already
uploaded attachments remain available to copy.

Use **Edit description** to change an image's alt text. The default is
its filename. This changes the copied Markdown, including after an upload; it
does not rename the file uploaded to GitHub.

During uploads, each attachment shows **Queued**, transfer progress, or
**Waiting for GitHub…** after all bytes have entered the HTTP upload stream.
The percentage is transfer progress, not confirmation that GitHub saved the
attachment. Only **Uploaded** confirms GitHub returned its attachment URL.

<img src="images/walkthrough/07-mixed-upload-progress.png" alt="A mixed image and video batch with a completed image, transfer progress, and a cancel action" width="620">

**Cancel upload** aborts the current request and stops the rest of the batch.
Completed URLs remain available; canceled and pending attachments can be
retried. Cancellation, a lost connection, or a timeout cannot undo a file GitHub
already received, so retrying an uncertain result can create another copy.
Connection attempts time out after 15 seconds; uploads stop after 90 seconds
without stream/response activity or five minutes overall. Errors explain
size/format problems, authentication/access failures, rate limits, or server
and network failures. Retrying is always an explicit Upload action.

Closing the main window cancels active compression and uploads and clears
attachments and results. Copy your Markdown before closing; drafts and upload
history are not saved between sessions.

Gatto’s menu bar, Dock, and app window share the same icon. Gatto remains in the
menu bar when windows close. Its Dock icon appears while a window is open;
clicking it focuses open windows or reopens the main page.
**Command+W** closes the focused window; **Command+Q** quits Gatto. These actions
also appear in the app menus, and the cat icon's menu includes **Quit** with **⌘Q**.
**Escape** closes the focused extra window, including Application logs and image
previews. The main window keeps its draft; Escape still dismisses its dialogs or
returns from App Preferences.
Press **Command+,** while Gatto is focused to open App Preferences, or choose
**App Preferences** from either the **Gatto** application menu beside the Apple
menu or the cat icon's dropdown (the menu bar extra). Both show **⌘,**.

## Inspect an image

Click an attachment's preview to open the separate preview window. Use its toolbar
to zoom, fit the image, or move between images. Drag or scroll to pan, pinch to
zoom, or hold **Command** or **Control** while scrolling to zoom around the pointer.
The percentage is relative to the fitted image size. Animated GIFs play with
their recorded frame timing and loop in the preview. Animation pauses when the
window is inactive; macOS Reduce Motion shows the first frame.

Click a video's play tile to open it in the default macOS video application
(often QuickTime Player). That separate application supplies playback, audio,
seeking, and codec error messages. WebM may require a compatible player; if
macOS cannot launch one, Gatto reports the failure. A container passing upload
validation does not guarantee its codec plays in every application or browser.
Videos uploaded without compression retain their original bytes and audio. Each private
temporary video copy stays available while staged or while the launched viewer
application is running, even after its draft is removed.

| Key | Preview action |
| --- | --- |
| **Command+=** or **Command+Plus** | Zoom in. |
| **Command+Minus** | Zoom out. |
| **Command+0** | Fit to window and reset pan. |
| **← / →** | Previous / next image. |
| **Escape** | Hide the preview window. |

## Faster clipboard uploads

With at least one repository pinned, these actions appear in Gatto’s menu bar menu:

- **Preview from Clipboard** stages the image or copied Finder media file in the main window and shows **⌘V**.
- **Quick Copy** uploads it in the background, copies its Markdown, and sends a macOS notification when it finishes. When its global shortcut is enabled, the menu shows the registered shortcut. The hint follows changes in App Preferences and disappears while the shortcut is disabled or being recorded.

To customize the enabled global shortcut in App Preferences, click **Record
shortcut** and press a supported chord with at least two modifier keys.

When multiple repositories are pinned, Quick Copy uses the most recently selected pinned
repository, falling back to the alphabetically first pinned repository. Quick Copy works
independently of open windows and their staged attachments. On failure, it leaves the
clipboard unchanged and sends a notification; copy the media and invoke it again.

## Automate Gatto with custom URLs

The [README’s URL table](../README.md#automate) lists the four supported actions.
Use them from Shortcuts, shell scripts, launchers, or other macOS automation.

From Terminal, use macOS's `open` command:

```sh
open "gatto://quick-copy"
```

Quick Copy requires a configured account or organization and at least one pinned repository. Gatto chooses
the preferred pinned repository using the same rules as the menu bar and global keyboard shortcut.
Install and open `Gatto.app` for macOS to register its URL scheme.

Action URLs intentionally accept no query parameters, fragments, repository names, or additional
path components; repository selection remains controlled by App Preferences.

## Diagnostics and recovery

The logs icon opens **Application Logs**. For collecting diagnostics, resetting
preferences, or uninstalling, see [Support](../SUPPORT.md).

## Install an update

A small **↑ Update** appears next to the header controls and in the menu bar menu
when a new release is available. Click it once to install and restart. Finish
active uploads first. **Updating…** shows progress; **↻ Update** retries a failure.
See [Setup](SETUP.md#updates) for timing and the first updater-enabled install.

To check immediately, choose **Gatto → Check for Updates…** in the macOS menu
bar. If a release is available, a small native pop-up shows its version and a
link to the GitHub release. Choose **Update** to install and restart, or **Cancel**
to leave it for later. Active uploads must finish before installation can start.
The command also reports when no update is available or the check fails.
Scheduled checks remain quiet and keep the existing Update controls current.

<img src="images/walkthrough/05-update.png" alt="Gatto's small Update control beside the header controls" width="620">

## Start quietly at login

Enable **Start at Login** in App Preferences to start Gatto in the menu bar at
macOS login, without opening a window. Quick Copy and update checks remain
available. Choose **Open App** from the menu bar, or open Gatto in Finder, when
you want its window. Opening the app normally still shows the main window.
