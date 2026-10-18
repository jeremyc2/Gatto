# Architecture

Gatto is a single-process macOS menu bar application. GPUI owns the
application and window lifecycle, while small modules isolate GitHub access,
platform integration, persistent settings, and shared state.

The public website is a separate static Solid project in `web/`. Vite runs under
Bun for development and builds; Bun prerenders its static HTML. It does not add
a backend to the app. See
[website development](../web/README.md) for its source and tooling boundaries.

## Responsibilities

The application separates these responsibilities within a single local process:

- **Interface and upload state:** show repositories and image previews, coordinate
  uploads and retries, and make uploaded links available to copy.
- **GitHub access:** use GitHub CLI for authentication and repository lookup, then
  send image bytes directly to GitHub's attachment service. Gatto has no backend
  or server of its own.
- **macOS integration:** provide menu bar actions, the optional global shortcut,
  custom URL actions, Start at Login, and silent installer ejection. Interactive
  entry points feed into the app's shared action handling.
- **Preferences and diagnostics:** persist local preferences and keep a bounded,
  in-memory event log available in the Application logs window.

Build and packaging tools supply version information, app metadata, and icons,
and produce the signed release. See [the release guide](RELEASING.md) for the
release workflow.

`installer_cleanup.rs` starts a background worker after the main window and menu
bar are ready. It acts only on a bundled executable in `/Applications/Gatto.app`
or the current user's `Applications/Gatto.app`. Release preparation creates the
installer volume and records its UUID in the app's Info.plist before signing.
The final DMG preserves that UUID through compression. Cleanup reads system disk
metadata through bounded `hdiutil info -plist` and `diskutil info -plist` calls;
it never reads mounted-volume files or opens the downloaded image. It requires
a single current-user, read-only disk image with the recorded UUID, rechecks its
identity, then requests ordinary AppKit ejection. Missing metadata, duplicate
mounts, development/image launches, and ejection failures are harmless. There
are no cleanup dialogs, download deletions, or additional persistent settings.
Each installed launch can retry cleanup; once ejected, later launches do nothing.

Rooftops uses native GPUI elements and the embedded gold-kitten coin sprite,
with the app’s existing dependencies.

## Gattoverse simulation and rendering

The web game uses BitECS 0.4.0's current API and Phaser 4.2.1. Its simulation is
independent of the browser and renderer. The module's interface is
`createSimulation(seed)` with `send(command)`, `step(seconds)`, and `read()`.
Tests use that same interface to play complete runs without a canvas or DOM.

| Location under `web/src/gattoverse/` | Owns |
| --- | --- |
| `simulation/contracts.ts` | Commands and the read-only view shared by callers. |
| `simulation/components.ts`, `world.ts` | Per-world BitECS component stores, entity creation and lifetime. |
| `simulation/generation.ts`, `random.ts` | Seeded roof contents and flight terrain. |
| `simulation/navigation.ts` | Furniture avoidance for tap-to-walk. Its navigation grid does not determine visual placement. |
| `simulation/systems/` | Movement, autonomous behavior, interactions, collisions and stamina. |
| `rendering/` | Phaser textures, directional sprite atlases, camera, depth ordering, scenery and visual feedback. |
| `input/browser.ts` | Keyboard, pointer and thumbstick input translated to commands. |
| `ui/` | Icon HUD, minimap, native pause/help/dialogue dialogs and touch controls. |
| `runtime.ts` | Browser lifecycle and fixed-step scheduling. |

The simulation advances at 60 fixed steps per second, with a bounded catch-up
interval. Each run allocates isolated component stores. Component data owns
positions, velocity, facing, AI targets and pursuit timers, collision extents,
loot state and inventory. Static roof and flight layouts are world resources.
Seeded random sources determine generation and AI decisions. Older roof entities
and flight sections are evicted as the player proceeds.

Phaser receives the view and updates sprites. It has no physics configuration,
input handlers, inventory or game-rule callbacks. Picking returns only a world
coordinate and optional entity ID; the simulation validates the resulting
command. Storage, visibility, native dialogs and keyboard focus stay outside the
simulation. Hiding the page pauses the run and sleeps rendering; resume requires
an explicit action. The only persisted game value is the browser's local best
coin score.

## Data flow

```text
Paste, drop, or file selection
             |
             v
      Validate image data
             |
             v
      Stage attachment in memory
             |
             +------> Show preview state and metadata
             |
             v
Request upload to the configured repository
             |
             v
Read active token from GitHub CLI
             |
             v
Send bytes to GitHub attachment service
             |
             v
Mark the attachment uploaded or failed
             |
             v
Add Markdown copy control to uploaded attachment
```

Attachments remain available after success or failure. Failed attachments can be retried without
selecting them again, and uploaded attachments retain their preview, description editor, and
Markdown copy control.

## Loading, batches, and window lifecycle

Launch restores the remembered repository name and ID from settings without a
GitHub request. Opening the repository picker loads the account or organization's paginated
repository list via GraphQL `repositoryOwner` (owned repositories only) and token through `gh`; uploading can instead resolve just the
remembered repository ID and token when needed. The token is cached in process
memory for the uploader window's lifetime. Reopen that window after changing the
active GitHub CLI account to avoid using a previously loaded session. Quick Copy
reads the current CLI token for each background job.

Image validation, CLI calls, and blocking HTTP uploads run on background tasks.
A batch uploads sequentially; each attachment transitions from preview or failed
to uploading, then uploaded or failed. Retrying skips uploaded attachments. The
copy result is rebuilt from successful attachments in their current order.
Generation counters prevent stale task completions from restoring state after
reset or draft replacement. Reset clears local state; it does not cancel an HTTP
request already sent to GitHub or remove a remote attachment.

The upload client uses a 15-second connection timeout and a 90-second request
timeout. It sends original image bytes to
`https://uploads.github.com/user-attachments/assets` with filename, content type,
and numeric repository ID query parameters. Returned URLs must use HTTPS,
`github.com`, and the `/user-attachments/assets/` path prefix. The endpoint is
hard-coded for GitHub.com; GitHub Enterprise Server hosts are not configurable.

On macOS, closing a window orders it out and vetoes native destruction. GPUI's
window teardown can trigger an uncaught AppKit Touch Bar observer exception
([upstream report](https://github.com/longbridge/gpui-kit/issues/3192)). Each window
type retains and reuses one handle, so repeated closes do not accumulate windows.
Explicit quit mode keeps the menu bar process alive after all windows are hidden.
The main-window controller retains shared diagnostics and menu/shortcut services
and creates the uploader view on first use, loading saved preferences. Main-window
close cancels shortcut recording and discards staged attachments and copy results.
Utility windows close independently: preview close releases its image references,
logs refresh on reopening, and Rooftops resets its run on close. Other platforms
use native destruction.

The rationale, trade-offs, and conditions for removing this workaround are recorded
in [ADR 0001](adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md).

Dock reopening activates existing presented windows without changing their pages.
If none are presented, it opens the main page. AppKit visibility includes occluded
and minimized windows; GPUI's drawing visibility is not used for this decision.
Native programmatic close requests run on the foreground executor outside a GPUI update,
because AppKit's synchronous close callback otherwise re-borrows the application.
The Dock icon hides when no windows are presented. Quit ends the process.

`application_menu` installs native Close Window and Quit Gatto menu items and
unscoped Command+W / Command+Q bindings for every app window. Close captures the
focused window, defers access until keyboard dispatch releases its borrow, then
uses the existing native close request and per-window cleanup callbacks.

Quick Copy runs through a process-lifetime background service with no window or
view dependency. Menu, shortcut, and URL actions all invoke that service. It
snapshots the clipboard, loads saved preferences, requires an account or organization and a
pinned repository, validates the clipboard image (or first copied image file),
and uses the same GitHub upload client and Markdown formatter as normal uploads.
It prefers the last selected repository when pinned, otherwise the first pinned
repository alphabetically. Success writes Markdown to the clipboard and sends a
macOS notification. Failure sends a notification and preserves the clipboard.
Concurrent Quick Copy requests are rejected while a job is running. Open windows
and staged attachments are untouched, and closing a window cannot stop its job.

## App Preferences and local state

The repository owner (personal account or organization), repository selections, shortcut configuration, and general preferences are
saved as JSON in `~/Library/Application Support/Gatto/settings.json`. The GitHub
token and image bytes are never written there. Until an account or organization is set, the main screen only
points the user to App Preferences. The `owner` setting accepts the former
`organization` key when reading existing preferences. Changing owners clears
repository selections, cached IDs, and pins after successfully saving settings.

Start at Login is represented by the exact file
`~/Library/LaunchAgents/com.jeremy-chandler.gatto.plist`. Enabling the
setting writes a LaunchAgent containing the current executable's canonical path;
disabling it removes that one file. The agent runs in the user's Aqua session at
the next login. No privileged helper or system-wide service is installed.

The **Reset app** preference removes both of these per-user files and returns
the in-memory UI to first-run setup. It does not modify GitHub CLI
authentication, which is managed outside of Gatto.

The global Quick Copy shortcut uses macOS hotkey registration rather than global keyboard event
monitoring, so it does not require Accessibility permission. The shortcut is opt-in, must contain
at least two modifiers, and is unregistered when disabled or when the app resets.

The build script accepts an explicit `GIT_COMMIT_HASH` environment value or asks
the local Git executable for the current short commit. It falls back to `unknown`
when neither source is available.

## Dependency compatibility

See [Contributing](../CONTRIBUTING.md#dependencies) for the pinned GPUI/GPUI Kit
pair and upgrade rules.

## Security boundaries

- Authentication remains delegated to GitHub CLI.
- Tokens are requested only when needed and are not written to application config.
- Image content is sent to GitHub and otherwise remains local to the process.
- Custom action URLs do not accept parameters; repository selection remains controlled by saved
  app preferences.
- Upload results are copied after an explicit copy action or a user-configured Quick Copy action.
- Persistent settings contain repository names and app preferences, never tokens or image bytes;
  Start at Login stores the executable path in a per-user LaunchAgent.
- Logs and errors should never include tokens or raw image bytes.
- Diagnostic events are retained only in memory (up to 500 entries) and are lost on quit.
