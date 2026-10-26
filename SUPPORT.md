# Support

## Getting help

[Overview](README.md) · [Setup](docs/SETUP.md) · [Walkthrough](docs/WALKTHROUGH.md)

For a usage question or reproducible bug, search existing issues before opening
a new one.

When reporting a problem, include:

- The app version and commit shown in **App Preferences → About**.
- The macOS version and Mac architecture.
- The output of `rustc --version`, `cargo --version`, and `gh --version` when the
  problem concerns local development.
- Clear reproduction steps and the expected and actual behavior.
- Sanitized error messages or screenshots.

Remove GitHub tokens, private repository names, private URLs, and confidential
images before sharing logs or screenshots.

## Compatibility

Gatto requires macOS 13 or newer. The download supports Apple Silicon Macs;
Intel Macs need a [source build](CONTRIBUTING.md#development-setup).

## Common problems

| Symptom                                                          | What to check                                                                                                                                                                                                                                                                                                                                      |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| GitHub CLI works in Terminal but Gatto cannot find it            | Application Logs shows the resolved CLI path. Gatto checks `GH_PATH`, `PATH`, `/opt/homebrew/bin/gh`, and `/usr/local/bin/gh`. For a custom installation, launch the executable with `GH_PATH=/absolute/path/to/gh` set; a GUI app may not inherit your shell’s Homebrew path.                                                                     |
| Account or organization does not load or a repository is missing | Enter a personal account or organization login, not a URL or `owner/repository`. Run `gh auth status --hostname github.com` to check the active GitHub CLI account. That account must have access to the selected owner’s repositories, including any private repository you need. For SAML single sign-on, authorize its token for SSO on GitHub. |
| An account switch is not reflected in Gatto                      | Restart Gatto after switching accounts with GitHub CLI; Gatto caches its session in memory.                                                                                                                                                                                                                                                        |
| File is rejected                                                 | Use PNG, JPEG, GIF, WebP, MP4, MOV, or WebM within your selected limit: 10 MB by default, optionally 100 MB for videos. Files must be nonempty and readable, with a supported extension. Choose Compress for an oversized video in the main window. Export corrupt or unsupported videos as H.264 MP4.                                             |
| Compression fails                                                | Use a shorter, SDR recording with one video track and at most one audio track. Check free disk space. Native hardware H.264 encoding and decoder support are required; some WebM codecs need another export. Canceling or failing leaves the original unchanged.                                                                                   |
| Video does not play                                              | Click its play tile to open the default macOS video app. Install a compatible player or export as H.264 MP4. WebM and some MOV codecs are not supported by every player/browser.                                                                                                                                                                   |
| GIF does not animate                                             | Animation plays in the separate image preview while active. macOS Reduce Motion holds the first frame.                                                                                                                                                                                                                                             |
| Clipboard action finds no image                                  | Copy actual image data or an image/video file in Finder; a text URL or Markdown snippet is not media data.                                                                                                                                                                                                                                         |
| A normal upload fails                                            | Inspect the attachment error and Application Logs, check network/account access, then click Upload to retry. Keep the window open; closing clears drafts.                                                                                                                                                                                          |
| Quick Copy fails                                                 | Configure an account or organization and pin an accessible repository. Copy the image or Finder file again and invoke Quick Copy again; this action clears its failed draft.                                                                                                                                                                       |
| Global shortcut will not register                                | Use at least two modifiers and a supported key. If another app owns the chord, record a different shortcut. No Accessibility permission is required.                                                                                                                                                                                               |
| Shortcut recorder keeps waiting or retains the previous shortcut | Quit any other Gatto copies, including source builds and local bundles, then retry with one copy running. A second copy can intercept the chord before the recorder receives it; each copy also retains its own preferences in memory.                                                                                                             |
| A window still opens at login after upgrading                    | Open the updated app once to refresh its existing login item, or toggle Start at Login off and on.                                                                                                                                                                                                                                                 |
| Start at Login stopped working after moving the app              | Put the app in its final location, then toggle Start at Login off and on.                                                                                                                                                                                                                                                                          |
| `gatto://` URLs do not open                                      | Install and open `Gatto.app`; a standalone development executable does not register the URL scheme.                                                                                                                                                                                                                                                |

Gatto defaults to a 10 MB (10,000,000-byte) limit per image or video for Free-plan
compatibility. [App Preferences](docs/SETUP.md#4-set-your-preferences) offers an
unchecked **Allow videos up to 100 MB** option for eligible paid-plan users.
It applies to videos only; Gatto does not verify your GitHub plan. If GitHub
rejects a larger upload, verify the repository owner's plan and your membership,
collaborator status, or personal paid plan. Otherwise disable the option and
remove/reimport the video to compress below 10 MB.
Transfer progress can reach 100% while waiting for GitHub's
response. See [uploads, cancellation, and retries](docs/WALKTHROUGH.md#bulk-uploads-and-retries)
for timeout behavior and recovery; canceling cannot remove a remote attachment
GitHub already accepted.

## Local data and recovery

Gatto writes these per-user files:

- `~/Library/Application Support/Gatto/settings.json`: account or organization, pinned and
  last-selected repositories, shortcut, copy preferences, and large-video opt-in.
- `~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`: Start at Login executable path.

Video staging creates private `gatto-video-*` files, and compression uses private
`gatto-compress-*` directories, in the per-user macOS temporary directory.
Staging snapshots contain the selected original video/audio or its accepted
compressed copy; they are never saved in settings. A snapshot is deleted when
its last draft, upload, or viewer reference is released; viewer references last
until the external application exits. Failed compression attempts are removed.
Forced termination may leave either kind of temporary data for macOS cleanup.

If malformed or unreadable settings prevent use, quit Gatto, move `settings.json`
somewhere safe, and reopen the app to configure it again. To clear a stale login
item, remove only the exact LaunchAgent file above while Gatto is quit. Moving
settings alone does not disable Start at Login.

## App updates

See [Setup's update instructions](docs/SETUP.md#updates). If installation fails,
hover **↻ Update** for details and open Application Logs, then click to retry.
Offline release checks are retried by Sparkle's four-hour scheduler. Only
bundled updater-enabled releases can update themselves; versions through v0.14.0
need a new DMG or Homebrew install once. Source builds do not check for updates.

Updater state and Sparkle defaults live in
`~/Library/Preferences/com.jeremy-chandler.gatto.plist`, with caches in
`~/Library/Caches/com.jeremy-chandler.gatto`. The selected release channel lives in
`~/Library/Application Support/Gatto/settings.json`. Reset app returns the channel
to Stable and invalidates the previous channel's update offer; Sparkle's check
schedule remains separate. A beta waiting for Stable is expected behavior, not
an installation failure. Gatto keeps the beta until Stable catches up.

This beta adds an optional settings field that older stable builds ignore; it
does not migrate the settings/data schema. If manually reinstalling an older
build, quit Gatto and back up `settings.json` first. Older builds may discard
fields they do not recognize when saving. Future betas that change persisted data
must document their own rollback limits; the channel control does not downgrade
or migrate data.

Homebrew users can uninstall with `brew uninstall --cask jeremyc2/gatto/gatto`;
add `--zap` to remove the cask's declared per-user settings and caches too.

## Finder integration

Gatto's [Finder integration](docs/SETUP.md#open-files-from-finder) ships in the app:
**Open With → Gatto** for supported media and a **Send to Gatto** service. Install
and open the current `Gatto.app` once. A plain `cargo run` executable does not
register document types or services. For local testing, build a fresh bundle with
`./scripts/build-app.sh`, then open the printed app path. Building a DMG alone
only packages the bundle you already have.

If Gatto is absent from Open With, choose **Other…** and select the intended app.
If multiple copies are installed, quit Gatto and launch the intended bundle before
retrying. Finder service placement and visibility are controlled by macOS;
**Send to Gatto** may appear under **Services**. If disabled, enable it in
**System Settings → Keyboard → Keyboard Shortcuts → Services**.

If an upload, import, update, or dialog is already active, finish it and try again.
If the service says a selected file could not be located, select it again in
Finder and retry; a file reference can become stale after the file is deleted.
Unsupported or unreadable files show a warning; mixed service selections retain
supported files. Nothing uploads until you click Upload.

If you installed the earlier development branch's Automator workflow, remove
`~/Library/Services/Send to Gatto.workflow` to avoid a duplicate entry. The current
app does not install or update that separate workflow.

## Reset and uninstall

**App Preferences → Reset app** asks for confirmation, removes saved preferences
and the login item, disables the global shortcut, and clears the main upload state.
It leaves GitHub CLI authentication, GitHub-hosted attachments, and Rooftops scorecards
in Downloads intact. Reset cannot undo an upload request already sent to GitHub.
Drafts and upload URLs are held in memory; closing the main window or quitting
loses them.

To uninstall, reset the app, choose **Quit** from the menu bar, and move
`Gatto.app` to the Trash. If the app cannot open, quit it and remove the two files
listed in [Local data and recovery](#local-data-and-recovery). Keep or remove
Rooftops scorecards as you prefer. If installed while testing an earlier branch,
remove the legacy [Finder workflow](#finder-integration) separately.

## Development builds

For Rust linker or SDK errors, install or update the Xcode Command Line Tools.
Confirm `xcode-select -p` prints a valid developer directory. After a macOS or
Xcode upgrade, accepting the Xcode license or reinstalling the tools may be needed.
See [Contributing](CONTRIBUTING.md#development-setup) for build prerequisites.

## Collecting diagnostics

Open **Application Logs** from the main window or App Preferences and choose
**Copy all**. Logs cover lifecycle, image processing, GitHub CLI, and uploads.
They retain at most 500 events and disappear on quit, so collect them before restarting. Gatto redacts common GitHub token prefixes, but logs can contain
local paths, account/organization and repository names, filenames, descriptions, and server
errors. Review and sanitize them before posting to a public issue. Never include
output from `gh auth token`.

Security issues must be reported privately according to [SECURITY.md](SECURITY.md).
