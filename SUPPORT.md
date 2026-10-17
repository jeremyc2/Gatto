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

| Symptom | What to check |
| --- | --- |
| GitHub CLI works in Terminal but Gatto cannot find it | Application Logs shows the resolved CLI path. Gatto checks `GH_PATH`, `PATH`, `/opt/homebrew/bin/gh`, and `/usr/local/bin/gh`. For a custom installation, launch the executable with `GH_PATH=/absolute/path/to/gh` set; a GUI app may not inherit your shell’s Homebrew path. |
| Account or organization does not load or a repository is missing | Enter a personal account or organization login, not a URL or `owner/repository`. Run `gh auth status --hostname github.com` to check the active GitHub CLI account. That account must have access to the selected owner’s repositories, including any private repository you need. For SAML single sign-on, authorize its token for SSO on GitHub. |
| An account switch is not reflected in Gatto | Restart Gatto after switching accounts with GitHub CLI; Gatto caches its session in memory. |
| Image is rejected | Use a readable PNG, JPEG, GIF, or WebP. Files must also have a supported extension. Convert HEIC, TIFF, SVG, or other formats first. |
| Clipboard action finds no image | Copy actual image data or an image file in Finder; a text URL or Markdown snippet is not image data. |
| A normal upload fails | Inspect the attachment error and Application Logs, check network/account access, then click Upload to retry. Keep the window open; closing clears drafts. |
| Quick Copy fails | Configure an account or organization and pin an accessible repository. Copy the image again and invoke Quick Copy again; this action clears its failed draft. |
| Global shortcut will not register | Use at least two modifiers and a supported key. If another app owns the chord, record a different shortcut. No Accessibility permission is required. |
| Start at Login stopped working after moving the app | Put the app in its final location, then toggle Start at Login off and on. |
| `gatto://` URLs do not open | Install and open `Gatto.app`; a standalone development executable does not register the URL scheme. |

Gatto has no app-defined upload size limit; GitHub’s response determines whether
an otherwise valid image is accepted.

## Local data and recovery

Gatto writes these per-user files:

- `~/Library/Application Support/Gatto/settings.json`: account or organization, pinned and
  last-selected repositories, shortcut, and copy preferences.
- `~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`: Start at Login executable path.

If malformed or unreadable settings prevent use, quit Gatto, move `settings.json`
somewhere safe, and reopen the app to configure it again. To clear a stale login
item, remove only the exact LaunchAgent file above while Gatto is quit. Moving
settings alone does not disable Start at Login.

## Reset and uninstall

**App Preferences → Reset app** asks for confirmation, removes saved preferences
and the login item, disables the global shortcut, and clears the main upload state.
It leaves GitHub CLI authentication, GitHub-hosted images, and Rooftops scorecards
in Downloads intact. Reset cannot undo an upload request already sent to GitHub.
Drafts and upload URLs are held in memory; closing the main window or quitting
loses them.

To uninstall, reset the app, choose **Quit** from the menu bar, and move
`Gatto.app` to the Trash. If the app cannot open, quit it and remove the two files
listed in [Local data and recovery](#local-data-and-recovery). Keep or remove
Rooftops scorecards as you prefer.

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
