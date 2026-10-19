# Gatto walkthrough

[Overview](../README.md) · [Setup](SETUP.md) · [Support](../SUPPORT.md)

Start with [setup](SETUP.md) to install Gatto and connect your account or organization.
The main window's question-mark icon opens this walkthrough in your browser.

## 1. Choose a repository

Open Gatto from its menu bar icon and choose the repository to upload through.
For frequent destinations, [pin a repository in App Preferences](SETUP.md#3-choose-your-account-or-organization).

In App Preferences, press **Escape** or click **Done** to return to the main page.
While recording a global shortcut, **Escape** cancels recording first.

## 2. Stage an image

Return to the main window and paste an image, drag an image into the drop zone, or click the drop zone to select a file. PNG, JPEG, GIF, and WebP images are supported.

<img src="images/walkthrough/03-image-staged.png" alt="Gatto with its rocket cat artwork staged and ready to upload" width="620">

The attachment stays in Gatto while it uploads. You can edit its description, remove it, or preview it before sending.

## 3. Upload and copy the Markdown

Click **Upload**. After GitHub accepts the attachment, its status changes to **Uploaded** and the attachment gains a copy button.

<img src="images/walkthrough/04-upload-complete.png" alt="Gatto showing an uploaded image with a Markdown copy action" width="620">

Normal uploads leave the clipboard unchanged. Click an attachment’s copy button
to copy that image. Press **Command+C** to copy every successful upload in
attachment order, separated by newlines.
For a single upload, the shortcut copies just that image; its hint appears after
the first success. Paste the Markdown into an issue, pull request, comment,
discussion, or Markdown file. The result has this form:

```md
![Gatto rocket cat](https://github.com/user-attachments/assets/…)
```

## Bulk uploads and retries

Enable **Bulk upload** before selecting or pasting several files. You can add more
images to the same batch by dropping files or pasting again. In single-image mode,
adding another image opens a replacement dialog; choose **Switch to bulk upload**
(or press **B** in that dialog) to keep the existing image and add the new one.
The bulk toggle is disabled during uploads and while more than one attachment
remains; remove attachments until at most one remains to return to single-image mode.

**Upload** sends each pending or failed attachment in order. Each attachment shows
its own status. If part of a batch fails, successful uploads remain available to
copy. Click **Upload** again to retry pending or failed images; already uploaded
images are skipped. **Copy all** uses the same completed results as **Command+C**.
Removing an attachment removes it from Gatto’s current list; it does not delete
the image from GitHub.

Use **Edit description** to change an image's Markdown alt text. The default is
its filename. This changes the copied Markdown, including after an upload; it
does not rename the file uploaded to GitHub.

Closing the main window clears its attachments and results. Copy your Markdown
before closing; drafts and upload history are not saved between sessions.

Gatto’s menu bar, Dock, and app window share the same icon. Gatto remains in the
menu bar when windows close. Its Dock icon appears while a window is open;
clicking it focuses open windows or reopens the main page.
**Command+W** closes the focused window; **Command+Q** quits Gatto. These actions
also appear in the app menus, and the cat icon's menu includes **Quit** with **⌘Q**.
Press **Command+,** while Gatto is focused to open App Preferences, or choose
**App Preferences** from either the **Gatto** application menu beside the Apple
menu or the cat icon's dropdown (the menu bar extra). Both show **⌘,**.

## Inspect an image

Click an attachment's preview to open the separate preview window. Use its toolbar
to zoom, fit the image, or move between images. Drag or scroll to pan, pinch to
zoom, or hold **Command** or **Control** while scrolling to zoom around the pointer.
The percentage is relative to the fitted image size.

| Key | Preview action |
| --- | --- |
| **Command+=** or **Command+Plus** | Zoom in. |
| **Command+Minus** | Zoom out. |
| **Command+0** | Fit to window and reset pan. |
| **← / →** | Previous / next image. |
| **Escape** | Hide the preview window. |

## Faster clipboard uploads

With at least one repository pinned, these actions appear in Gatto’s menu bar menu:

- **Preview from Clipboard** stages the image in the main window and shows **⌘V**.
- **Quick Copy** uploads it in the background, copies its Markdown, and sends a macOS notification when it finishes. When its global shortcut is enabled, the menu shows the registered shortcut. The hint follows changes in App Preferences and disappears while the shortcut is disabled or being recorded.

To customize the enabled global shortcut in App Preferences, click **Record
shortcut** and press a supported chord with at least two modifier keys.

When multiple repositories are pinned, Quick Copy uses the most recently selected pinned
repository, falling back to the alphabetically first pinned repository. Quick Copy works
independently of open windows and their staged images. On failure, it leaves the
clipboard unchanged and sends a notification; copy the image and invoke it again.

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
