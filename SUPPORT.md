# Support

## Getting help

Start with the setup and troubleshooting sections in [README.md](README.md). For a
usage question or reproducible bug, search the repository's existing issues before
opening a new one.

When reporting a problem, include:

- The app version and commit shown in **App Preferences → About**.
- The macOS version and Mac architecture.
- The output of `rustc --version`, `cargo --version`, and `gh --version` when the
  problem concerns local development.
- Clear reproduction steps and the expected and actual behavior.
- Sanitized error messages or screenshots.

Remove GitHub tokens, private repository names, private URLs, and confidential
images before sharing logs or screenshots.

## Common problems

| Symptom | What to check |
| --- | --- |
| GitHub CLI works in Terminal but Gatto cannot find it | Application Logs shows the resolved CLI path. Gatto checks `GH_PATH`, `PATH`, `/opt/homebrew/bin/gh`, and `/usr/local/bin/gh`. For a custom installation, launch the executable with `GH_PATH` set. |
| Organization or repository does not load | Enter an organization login, not a URL or personal account. Check `gh auth status --hostname github.com` and the active account's access. For an organization with SSO, check its token authorization. |
| An account switch is not reflected in Gatto | Restart Gatto after switching accounts with GitHub CLI; Gatto caches its session in memory. |
| Image is rejected | Use a readable PNG, JPEG, GIF, or WebP. Files must also have a supported extension. Convert HEIC, TIFF, SVG, or other formats first. |
| Clipboard action finds no image | Copy actual image data or an image file in Finder; a text URL or Markdown snippet is not image data. |
| A normal upload fails | Inspect the attachment error and Application Logs, check network/account access, then click Upload to retry. Keep the window open; closing clears drafts. |
| Quick Copy fails | Configure an organization and pin an accessible repository. Copy the image again and invoke Quick Copy again; this action clears its failed draft. |
| Global shortcut will not register | Use at least two modifiers and a supported key. If another app owns the chord, record a different shortcut. No Accessibility permission is required. |
| Start at Login stopped working after moving the app | Put the app in its final location, then toggle Start at Login off and on. |
| `gatto://` URLs do not open | Install and open `Gatto.app`; a standalone development executable does not register the URL scheme. |

GitHub.com is the only configured host. Gatto does not provide a GitHub Enterprise
Server host preference or its own upload size limit; GitHub's response determines
whether an otherwise valid image is accepted.

## Local data and recovery

Gatto writes these per-user files:

- `~/Library/Application Support/Gatto/settings.json`: organization, pinned and
  last-selected repositories, shortcut, and copy preferences.
- `~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`: Start at Login executable path.

Use **App Preferences → Reset app** for a confirmed reset. If malformed or
unreadable settings prevent normal use, quit Gatto first, move `settings.json`
somewhere safe, and reopen the app to configure it again. To clear a stale login
item manually, remove only the exact LaunchAgent file above while Gatto is quit.
Moving settings alone does not disable Start at Login.

Reset does not remove GitHub-hosted attachments, GitHub CLI authentication, or
Rooftops PNG scorecards in Downloads. It also cannot undo an upload request that
has already reached GitHub. Drafts and upload URLs are held in memory and cannot
be recovered after closing the main window or quitting.

## Collecting diagnostics

Open **Application Logs** from the main window or App Preferences and choose
**Copy all**. Logs retain at most 500 events and disappear on quit, so collect them
before restarting. Gatto redacts common GitHub token prefixes, but logs can contain
local paths, organization/repository names, filenames, descriptions, and server
errors. Review and sanitize them before posting to a public issue. Never include
output from `gh auth token`.

Security issues must be reported privately according to [SECURITY.md](SECURITY.md).
