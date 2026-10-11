# Gatto walkthrough

Gatto turns a local image into GitHub-ready Markdown. This walkthrough covers setup, normal and
background uploads, keyboard shortcuts, and the `gatto://` automation scheme.

## 1. Set up the app

The first-run screen points you to App Preferences. Gatto will not contact GitHub until an organization has been configured.

<img src="images/walkthrough/01-first-run.png" alt="Gatto's first-run screen with Application Logs, App Preferences, walkthrough help, and an Open App Preferences button" width="620">

Open **App Preferences** using the settings icon, enter the GitHub organization whose repositories
you want to use, and save it. Gatto uses the active GitHub CLI account. If GitHub CLI is not yet
installed, follow its [installation instructions](https://cli.github.com/), then run `gh auth login`
and sign in to an account that can access the organization. The logs icon opens Application Logs,
and the question-mark icon opens this walkthrough in your default browser.

## 2. Choose a repository

Select the repository that should own the uploaded attachment. Pin repositories you use regularly to keep them at the top of the picker and enable the menu bar's clipboard shortcuts.

<img src="images/walkthrough/02-preferences.png" alt="Gatto App Preferences with an organization, pinned repository, and global Quick Copy shortcut settings" width="620">

Settings are stored locally. GitHub authentication remains managed by GitHub CLI. App Preferences
also contains Start at Login, the optional global Quick Copy shortcut, and **Close the window after
copy**.

## 3. Stage an image

Return to the main window and paste an image, drag an image into the drop zone, or click the drop zone to select a file. PNG, JPEG, GIF, and WebP images are supported.

<img src="images/walkthrough/03-image-staged.png" alt="Gatto with its rocket cat artwork staged and ready to upload" width="620">

The attachment stays in Gatto while it uploads. You can edit its description, remove it, or preview it before sending.

## 4. Upload and copy the Markdown

Click **Upload**. After GitHub accepts the attachment, its status changes to **Uploaded** and the attachment gains a copy button.

<img src="images/walkthrough/04-upload-complete.png" alt="Gatto showing an uploaded image with a Markdown copy action" width="620">

Copy the generated Markdown and paste it into a GitHub issue, pull request, discussion, or Markdown file. The result has this form:

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
images are skipped. **Copy all** copies only successful uploads in attachment
order, separated by newlines. Removing an attachment removes it from Gatto's
current list; it does not delete the image from GitHub.

Use **Edit description** to change an image's Markdown alt text. The default is
its filename. This changes the copied Markdown, including after an upload; it
does not rename the file uploaded to GitHub.

Closing the main window clears its attachments and results. Copy your Markdown
before closing; drafts and upload history are not saved between sessions.

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

After pinning a repository, copy an image and use one of these actions from Gatto's menu bar icon:

- **Preview from Clipboard** stages the image in the main window.
- **Quick Copy** uploads it in the background, copies its Markdown, and sends a macOS notification when it finishes.

You can also set a global Quick Copy keyboard shortcut in App Preferences. Once enabled it starts
as **Command+Shift+G**, and you can change it to another supported chord: click **Record shortcut**,
then press the chord you want to use. The shortcut must contain at least two modifier keys.

When multiple repositories are pinned, Quick Copy uses the most recently selected pinned
repository, falling back to the alphabetically first pinned repository.

## Keyboard copy action

After at least one upload succeeds, the uploader supports this keyboard action:

- **Command+C** copies every uploaded Markdown image snippet, separated by newlines.

For a single upload, it copies just that image's Markdown snippet. A shortcut hint appears after
at least one upload succeeds, in both single-image and bulk upload mode.

This shortcut acts on completed uploads in the open window. The optional global Quick Copy
shortcut is different: it reads a new image from the clipboard and performs a background upload.

## Automate Gatto with custom URLs

The packaged app registers the `gatto` custom URL scheme with macOS. Shortcuts, shell scripts,
launchers, browsers, and other automation tools can invoke the same actions as Gatto's interface:

| URL | Action |
| --- | --- |
| `gatto://open` | Show and activate the main Gatto window. |
| `gatto://preview` | Show the main window and stage the current clipboard image. |
| `gatto://quick-copy` | Upload the clipboard image in the background, copy its Markdown, and notify when finished. |
| `gatto://settings` | Show Gatto and open App Preferences. |

From Terminal, use macOS's `open` command:

```sh
open "gatto://quick-copy"
```

Quick Copy requires a configured organization and at least one pinned repository. Gatto chooses
the preferred pinned repository using the same rules as the menu bar and global keyboard shortcut.
Install and open the packaged `Gatto.app` for macOS to register the URL scheme;
`cargo run` alone does not install its bundle metadata.

Action URLs intentionally accept no query parameters, fragments, repository names, or additional
path components; repository selection remains controlled by App Preferences.

## Reset preferences

Use **App Preferences → Reset app** to return to first-run setup after confirming
the reset. This removes Gatto preferences and Start at Login, disables its global
shortcut, and clears the main upload state. GitHub CLI authentication and already
uploaded GitHub images are unaffected. See [Support](../SUPPORT.md) for recovery
when settings cannot be read.

## Rooftops

Click the cat logo in the main window to play the three-level platformer. The
[Rooftops guide](EASTER_EGG.md) covers controls, checkpoints, and saving a scorecard.

## Updating these screenshots

From the repository root, run:

```sh
./scripts/update-walkthrough-screenshots.sh
```

The script renders the production GPUI interface through its off-screen Metal renderer using fixed, in-memory example data. It does not read or modify Gatto preferences, contact GitHub, use UI automation, or require Screen Recording permission. The command is macOS-only and overwrites the PNG files under `docs/images/walkthrough/`, including the [Rooftops Easter egg](EASTER_EGG.md), its results screen, and the exported score image.
