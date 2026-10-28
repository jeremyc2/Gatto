# Gatto walkthrough

[Overview](../README.md) · [Setup](SETUP.md) · [Support](../SUPPORT.md)

Start with [setup](SETUP.md) to install Gatto and connect your account or organization.
Open this walkthrough from **Help → Walkthrough** in the macOS menu bar.

## 1. Choose a repository

Open Gatto from its menu bar icon and choose the repository to upload through.
For frequent destinations, [pin a repository in App Preferences](SETUP.md#3-choose-your-account-or-organization).

Click **Done** or press **Escape** to return from App Preferences. If a dialog
is open or you're recording a shortcut, Escape dismisses that first.

## 2. Stage an image or video

Return to the main window and paste an image, drag a file into the drop zone, or
click the drop zone to select a file. Gatto accepts PNG, JPEG, GIF, and WebP
images, plus MP4, MOV, and WebM videos. You can also copy an image or video file
in Finder and paste it into Gatto. To add a video, copy the file itself rather
than a link to it.

By default, each file must be at most **10 MB**. Eligible paid-plan users can enable
**Allow videos up to 100 MB** in [App Preferences](SETUP.md#4-set-your-preferences).
Check that your destination repository's plan allows larger videos before
enabling this. Images and GIFs stay limited to 10 MB. For larger videos, choose
**Compress** to try making them fit, or **Cancel** to skip them.

<img src="images/walkthrough/framed/06-video-staged.png" alt="A video ready to upload" width="800">

<img src="images/walkthrough/framed/03-image-staged.png" alt="Gatto with a screenshot staged and ready to upload" width="800">

Before uploading, you can edit an attachment's title and description, remove it, or click
it to preview it.

### Projects and Scratchpad

Use Scratchpad for your work, or create projects to organize long-term work into
different categories.
Quick Copy is separate: it uploads in the background without using Scratchpad.
Draft from Clipboard and Side-by-Side Comparison use the project in the open main
window, or Scratchpad when no main window is open.

Use **Create Project** at the bottom of the sidebar. Enter a distinct name and an
optional description. When starting from a nonempty Scratchpad, choose whether to
transfer its attachments. Transferring moves them into the new project; otherwise
Scratchpad keeps them and the new project starts empty. New folders are created
in `~/.gatto/projects`. Existing folders are never overwritten.

Projects appear in last-updated order. Search filters by name. The project list
scrolls between search at the top and the pinned Scratchpad and Create Project
controls at the bottom. **Command+B** hides or shows the entire sidebar.
The main heading shows **Project: name**, or **Scratchpad**.

Use a project's **⋯** menu for **Edit metadata** or **Delete**. Edit metadata changes
its name and optional multiline description without moving its folder. Names must
be distinct, ignoring case. Delete immediately removes the project folder and all
contents. The sidebar picks up project folders in `~/.gatto/projects` when it
refreshes and drops folders removed outside the app. The Dock's right-click menu
opens up to five recently updated projects.

Click the main window's question mark for the project name, optional multiline
description, storage size, selected repository, and creation and update times.
The macOS **Help** menu links to the repository, walkthrough, setup, and support.

<img src="images/walkthrough/framed/10-project.png" alt="A project with sidebar search, pinned Scratchpad and Create Project controls, and an uploaded attachment" width="800">

Each workspace saves attachment copies, titles, descriptions, marker notes, order,
completed upload URLs, and its own selected repository. Repository pins remain
app-wide. Switching back to Scratchpad restores only its own work and repository.
Selecting the current workspace does nothing. If work is pending when switching,
a short dialog lets you continue working or cancel it. In-progress disk writes
must finish before switching.

Previously uploaded, unchanged attachments reopen as **Uploaded**, ready to copy
without uploading again.
Metadata edits and reordering preserve URLs. Changing image pixels or externally
modifying a media file clears only that attachment's cached URL. Failed or
interrupted uploads reopen as drafts.

Drag an attachment's surface to reorder it. The full card follows the pointer,
other cards shift, and the list scrolls near its edges. **Escape** restores the
starting order. Over the sidebar, the card becomes a compact indicator so destinations
stay visible; returning to the attachment area restores the full card. The thumbnail
uses a grab cursor; there is no separate drag handle.

Use the checkboxes or **Select All** to select attachments. The selection's **⋯**
menu offers **Remove** and **Move to project**. An individual attachment's **⋯**
menu also has **Edit metadata** for its title and description, plus **Annotate** and
**Clone** for images. Clone appends an independent draft with the same image,
description, and editable annotations.
For a PNG, JPEG, or WebP image that is not already a comparison, it also offers
**Convert to side-by-side comparison**.
Move opens a project picker that excludes the current project, with a **Create
Project** button to create a destination for the selected attachments. With no
destinations, only the creation option appears. You can also drop a card onto a
sidebar project. Moves retain attachment identity and uploaded URLs; failed moves
preserve the originals.

<img src="images/walkthrough/framed/12-attachment-reorder.png" alt="Attachment checkboxes, Select All, and pinned sidebar controls" width="800">

If saving fails, restore folder access or free disk space and choose **Retry save**.
**Create Project** in the sidebar can preserve the current attachments in a new
project. Keep its `gatto-project.json` and `gatto-media/` together when backing it up.
For annotated images, original sources and editable annotations are saved beside
the rendered images in `gatto-media/`, with annotation JSON that references the
repository schema. Removing an attachment also removes its saved source and annotation files.

### Send files from Finder

Select one or more supported files in Finder, then choose **Open With → Gatto**
from the right-click menu, or use **Send to Gatto** (possibly under **Services**).
Both are included in the [installed app](SETUP.md#open-files-from-finder). Gatto launches if needed, or brings its main
window forward, and stages your selection for review. Click **Upload** yourself
when ready.

Every accepted file is appended in selection order.

Finish an active upload, import, update, or dialog before sending another
selection. If Gatto is busy it keeps the current draft and asks you to invoke the
action again. You can stage before configuring GitHub, then finish setup before
uploading.

### Compress a larger video

When a selected video exceeds your limit, choose **Compress** to make a smaller
copy, or **Cancel** to skip it. Your original file stays unchanged. Compression
can reduce picture and audio quality; preview the copy before sharing it.

<img src="images/walkthrough/framed/08-video-too-large.png" alt="Gatto offers Cancel or Compress for an oversized video" width="800">

Wait for compression to finish, then click **Upload** when you're ready. Use
**Cancel compression** to skip that file. In a batch, you can continue with the
remaining files.

<img src="images/walkthrough/framed/09-video-compressing.png" alt="A video being compressed before upload" width="800">

If compression fails or the result is still too large, trim the video or export
it as a standard H.264 MP4 and try again. If Quick Copy reports that a video is
too large, add it in the main window to choose compression.

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

## 3. Upload and copy the Markdown

Click **Upload** and wait for **Uploaded**, then click the attachment's copy
button to copy its snippet.

<img src="images/walkthrough/framed/04-upload-complete.png" alt="Gatto showing an uploaded image with a Markdown copy action" width="800">

Uploading alone does not copy anything. Press **Command+C** to copy all completed
uploads in attachment order, or use an attachment's copy button to copy just one.
Paste the Markdown into an issue, pull request, comment, discussion, or Markdown
file. By default, images copy in this form:

```md
<img width="1408" height="784" alt="Interface review" src="https://github.com/user-attachments/assets/…" />
```

To use Markdown image syntax instead, enable **Copy images as Markdown** in
[App Preferences](SETUP.md#4-set-your-preferences). The same image then copies as:

```md
![Interface review](https://github.com/user-attachments/assets/…)
```

Both formats use your title as alt text. The preference also applies to
Quick Copy and previously uploaded images; copy them again to use the new
format. Videos copy as a link that GitHub can display as a player:

```md
https://github.com/user-attachments/assets/…
```

When pasting a video link, keep it on its own line. Video titles and descriptions are for
your reference in Gatto and aren't included in the copied link.

## Uploads and retries

Select, drop, or paste one or more files. Every new attachment is appended to the
current workspace.

Click **Upload** to send the batch. If part of a batch fails, successful uploads remain available to
copy. Click **Upload** again to retry pending or failed attachments; already uploaded
attachments are skipped. **Copy all** uses the same completed results as **Command+C**.
Removing an attachment removes it from Gatto’s current list; it does not delete
the attachment from GitHub.

Finish any upload or compression before changing the 100 MB preference. If you
turn it off with a larger video already staged, remove the video and add it again
to compress it below 10 MB.

Use an attachment’s **⋯ → Edit metadata** menu item to edit **Title**, which defaults to
the filename and supplies an image's alt text, and the optional multiline
**Description**. Description is included only as an escaped `data-description`
attribute on a copied HTML image tag. Markdown image syntax and video links omit
it. Editing these fields updates copied snippets, including after an upload,
without renaming or uploading the file again.

<img src="images/walkthrough/framed/11-attachment-details.png" alt="The Edit attachment dialog with a Title field and multiline Description field" width="800">

Wait for **Uploaded** before trying to copy an attachment. If you see
**Waiting for GitHub…**, leave the upload running until it finishes or reports
an error.

<img src="images/walkthrough/framed/07-mixed-upload-progress.png" alt="A three-file batch with an uploaded image, a video sending, a queued timeline image, and a cancel action" width="800">

Click **Cancel upload** to stop the upload and the rest of the batch. Completed
uploads remain available to copy; click **Upload** to retry the unfinished files.
Cancellation, a lost connection, or a timeout cannot undo a file GitHub
already received, so retrying an uncertain result can create another copy.
If an upload fails, follow the error message and click **Upload** to try again.

### Closing Gatto

Scratchpad and project attachments, completed upload URLs, and repository selections
are saved on disk. Finish or cancel active work and let pending saves complete
before closing or quitting.

Gatto remains in the menu bar when you close its windows. Choose **Open App**
from its menu bar menu to open it again.
**Command+W** closes the focused window; **Command+Q** quits Gatto. These actions
also appear in the app menus.
**Escape** dismisses the active modal, including while typing in its fields.
Dialog validation errors appear inside the dialog. Without a modal, Escape closes
Application logs and other extra windows. In the image preview,
it only clears focus and selection; it never closes the preview. The main window
keeps its draft; Escape still dismisses its dialogs or returns from App Preferences.
Press **Command+,** while Gatto is focused to open App Preferences, or choose
**App Preferences** from the **Gatto** application menu or its menu bar icon.

## Inspect an image

Click a staged image to open it in the editor. Add a note, point out a detail,
or pixelate an area before uploading. Your original file stays unchanged.

<img src="images/walkthrough/framed/image-editor.png" alt="Gatto image editor with a note and an arrow on a screenshot" width="820">

Choose a drawing tool, then adjust its color, size, or fill in the tool options.
Hover over a tool to see its name and keyboard shortcut.

### Get a closer look

Use **+** and **−** at the bottom left to zoom. Click the percentage for
**Zoom to fit** or **Zoom to 100%**. Choose the hand tool and drag to move around
the image; scrolling and pinch-to-zoom work too.

Use the arrows to move between staged images. You can preview GIFs here, but
cannot edit them. If a GIF isn't playing, bring the editor to the front and
check whether macOS Reduce Motion is enabled.

### Add a note or callout

Choose the text tool and click where you want to write. For a longer note, drag
out a text box first so the words wrap. Double-click existing text to change it.
Click away when you're finished, or press **Escape** to finish typing.

<img src="images/walkthrough/framed/image-editor-text-editing.png" alt="A note being edited directly on a screenshot, with text options beside it" width="820">

Use an arrow to point to a detail, a shape to highlight an area, or the pen to
draw freely. Drag to draw an arrow or set a shape's size. You can also click to
place a shape, then resize it later. Double-click a shape or arrow to add a label
that moves with it.

The small chevron in the bottom toolbar opens more shapes. After selecting a
shape you've drawn, use **Shape** in its options to change it. Try a transparent
fill to keep the image visible, or a soft fill to highlight an area. The same
panel lets you change colors, line styles, size, and opacity.

<img src="images/walkthrough/framed/image-editor-shapes.png" alt="The shape menu offers callout shapes, symbols, and directional arrows" width="820">

Most tools switch back to selection after drawing. Turn on the lock in the
toolbar if you'd like to keep using the same tool.

### Add numbered steps

Choose **Numbered marker** (**N**) and click the image to add numbered steps.
Select a marker to move it, change its color, or write a description in the
area at the bottom. Drag a corner to resize all markers together.

Only the numbers appear on the image. When you copy it after uploading, the
steps and descriptions appear as a numbered list underneath.

<img src="images/walkthrough/framed/image-editor-markers.png" alt="Two numbered markers with a description being edited below the image" width="820">

### Adjust what you've added

Choose the selection tool, then click a mark to move it. For a transparent shape,
click its outline or label. Drag a corner to resize; drag just outside a corner
to rotate. Hold **Shift** to keep proportions while resizing or to rotate in
small, even steps. An arrow's handles let you move its ends and bend its path.

To work on several marks at once, drag a selection box from an empty area.
You can move, resize, rotate, or restyle them together. Click an empty area to
clear the selection.

<img src="images/walkthrough/framed/image-editor-group.png" alt="A note and its arrow selected together inside a shared blue resize box" width="820">

Use **Undo** or **Redo** to revisit a change, delete to remove selected marks,
or **Clone** to duplicate them. You can also copy and paste marks
between images in the editor. Right-click a mark to bring it forward or send it
behind other marks.

### Pixelate an area

Choose the pixelation brush at the right end of the toolbar. Move over the image
to preview the effect, then click or drag to apply it. Use the size slider to
cover a smaller or larger area. Text, arrows, and shapes stay visible above it.

<img src="images/walkthrough/framed/image-editor-brush.png" alt="The pixelation brush hides a local file path, with its size slider at the upper right" width="820">

Pixelation stays where you paint it; it cannot be moved or erased with the
eraser. Use **Undo** to remove a stroke. Check the result before sharing.
Your original file on disk stays unchanged.

### Save and upload

Click **Save** when you're happy with the result. Gatto saves the changed images
back to your staged attachments and closes the editor. Your descriptions stay
in place. If you edited an image that was already uploaded, upload it again to
get a link to the new version.

You can reopen a saved image to adjust its marks, including after switching
projects or restarting the app.
Edited images upload as PNGs; recipients cannot edit the marks separately.
Uploaded images must fit the 10 MB limit. If the rendered image exceeds it,
reduce the image size or marks extending far beyond its edges before saving.

**Discard** is active only when there are unsaved changes. Reopening a saved
image or undoing back to its saved version leaves it inactive.
When active, **Discard** asks for confirmation, then closes the editor without
your unsaved changes. Your last saved version stays staged and its marks remain
editable.

Closing the window with unsaved changes gives you a choice: **Save**,
**Discard**, or **Cancel** to keep editing.

<img src="images/walkthrough/framed/image-editor-exit.png" alt="The editor asks whether to save changes before closing, with Discard, Cancel, and Save choices" width="820">

Finish any import or upload before saving. If image export fails, the editor
stays open with your work intact. Scratchpad and projects keep their staged
attachments and uploaded links when the main window closes.

A few useful shortcuts:

| Shortcut                            | Action                                                      |
| ----------------------------------- | ----------------------------------------------------------- |
| **Command+S**                       | Save changed images and close the editor.                   |
| **Command+Z** / **Shift+Command+Z** | Undo / redo.                                                |
| **Delete**                          | Remove selected marks.                                      |
| **Escape**                          | Finish typing or clear the selection; keep the editor open. |
| **Command+W**                       | Close the editor, asking about unsaved changes.             |

### Preview a video

Click a video's play tile to open it in your usual macOS video player, often
QuickTime Player. Use that app to play, pause, seek, or adjust the volume.
WebM may need a compatible player; Gatto reports an error if it cannot open one.
A video that uploads successfully may still need a different player or format
for some viewers.

## Compare two images

Choose **Advanced → Side-by-Side Comparison** from Gatto’s menu. Select, paste, or drop a
PNG, JPEG, or WebP image for each side. Once both are ready, Gatto combines them
into one attachment and appends it to the current workspace.

To convert an existing image, choose **Convert to side-by-side comparison** from
that attachment's **⋯** menu. The existing image is filled in. Choose **Left** or
**Right** for the new image, then add it in the remaining slot. The comparison
replaces the original attachment in the same position.

Click the combined attachment to open both images together in the editor.
Captions start as **Before** and **After**; double-click either one to rename it.
This example uses **orange cat** and **black cat**. Click **Save**, then upload
and copy the comparison like any other image.

<img src="images/walkthrough/framed/comparison-editor.png" alt="Gatto’s comparison editor with matching pixel-editor views labeled orange cat and black cat, with the same gardener outfit and shovel" width="820">

## Faster clipboard uploads

The menu bar includes Draft from Clipboard and Side-by-Side Comparison. Quick Copy
appears when a repository is pinned:

- **Draft from Clipboard** adds the image or copied Finder media file to the main window, ready for you to review and upload.
- **Quick Copy** uploads it immediately, copies its Markdown, and notifies you when it finishes.

To use Quick Copy from another app, enable its global shortcut in App Preferences.
To change it, click **Record shortcut** and press a key combination with at least
two modifier keys.

When multiple repositories are pinned, Quick Copy uses Scratchpad's last-selected
repository if it is pinned, falling back to the alphabetically first pinned
repository. Quick Copy works independently of open windows and their staged attachments. On failure, it leaves the
clipboard unchanged and sends a notification; copy the media and invoke it again.

## Automate Gatto with custom URLs

The [README’s URL table](../README.md#automate) lists the five supported actions.
Use them from Shortcuts, shell scripts, launchers, or other macOS automation.

From Terminal, use macOS's `open` command:

```sh
open "gatto://quick-copy"
```

Before using these links, install and open `Gatto.app`. For Quick Copy, configure
your account or organization and pin at least one repository in App Preferences.

The URL names match the menu actions: **Draft from Clipboard**,
**Quick Copy**, **App Preferences**, and **Side-by-Side Comparison**, alongside
opening Gatto.

Use the URLs exactly as listed. To change the destination repository, select
and pin it in App Preferences rather than adding it to the URL.

## Diagnostics and recovery

The logs icon opens **Application Logs**. For collecting diagnostics, resetting
preferences, or uninstalling, see [Support](../SUPPORT.md).

## Install an update

Choose **Stable** or **Beta** under **Release channel** in **App Preferences**.
Choose Beta if you want to try prerelease versions. Switching channels checks
for updates but doesn't install one. If you switch from Beta back to Stable,
keep using your installed version until a newer stable release is available.

When an update is available, click **↑ Update** in the main window or menu bar
menu to install it and restart Gatto. Finish active uploads first. If an update
fails, click **↻ Update** to retry.
See [Setup](SETUP.md#updates) if you need help getting updates.

To check immediately, choose **Gatto → Check for Updates…** in the macOS menu
bar. If a release is available, review its release notes, then choose **Update**
to install and restart, or **Cancel** to leave it for later.

<img src="images/walkthrough/framed/05-update.png" alt="An update available to install" width="800">

## Start quietly at login

Enable **Start at Login** in App Preferences to start Gatto in the menu bar at
macOS login, without opening a window. Quick Copy and update checks remain
available. Choose **Open App** from the menu bar, or open Gatto in Finder, when
you want its window. Opening the app normally still shows the main window.
