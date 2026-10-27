---
status: accepted
date: 2026-10-12
---

# Retain macOS windows to avoid Touch Bar crashes

Gatto 0.10.3 restored native window destruction, but closing Logs or Rooftops
aborted the entire app on a user's Touch Bar Mac with an uncaught AppKit
`NSRangeException` involving `_NSTouchBarFinderObservation` and `nextResponder`.
Starting with 0.10.4, macOS close handlers hide windows with `orderOut:` and veto
native destruction, retaining one window per type for reuse. We accept the retained
window resources to avoid the suspected teardown path while preserving independent
window closing and the menu bar process.

The exception matches [gpui-kit issue #3192](https://github.com/longbridge/gpui-kit/issues/3192).
Its [maintainer response](https://github.com/longbridge/gpui-kit/issues/3192#issuecomment-5856803075)
points toward GPUI's platform lifecycle or AppKit, rather than gpui-kit's
accessibility additions; the exact root cause remains unresolved.
[navop PR #268](https://github.com/feigeCode/navop/pull/268) documents the same
window-close exception and a hide-and-reuse workaround in another GPUI app.

## Consequences

The draft-clearing behavior below predates persistent workspaces. See the
[current lifecycle](../ARCHITECTURE.md#loading-batches-and-window-lifecycle) for saved-draft behavior.

- Closing affects only that window. Main-window close cancels shortcut recording
  and discards staged attachments and copy results; preview close releases its
  image references; Rooftops resets its run; Logs refreshes on reopening.
- Each window type keeps its existing handle. Repeated opening and closing must
  not create additional hidden windows. Native window resources remain until Quit.
- Dock reopening focuses presented windows without changing their pages. If none
  are presented, it opens the main page, even if the retained main window last
  showed Preferences. Minimized and occluded windows count as presented.
- Quick Copy remains a background service and must never create a visible or
  hidden window. This workaround applies to windows opened for the UI.
- Other platforms continue to use native window destruction.

## Validation and reconsideration

Development and bundled checks passed on the local M4 Pro Mac, which has no Touch
Bar. The affected user supplied the native exception and confirmed a Touch Bar and
Logs/Rooftops close triggers. Version 0.10.4 was released as a best-effort mitigation
after automated checks and signed-bundle close/reopen checks passed locally; it
has not yet been validated on affected hardware.

Do not restore native destruction solely because local tests pass or a development
build appears stable. Revisit this decision when a GPUI/AppKit fix or another
supported approach is available, and verify the signed, bundled app on a Touch
Bar Mac. Exercise repeated closing and reopening of Logs, Rooftops, previews, and
the main window, sibling-window survival, last-window Dock reopening from
Preferences, and windowless Quick Copy before replacing the workaround. Record
the replacement in a superseding ADR.
