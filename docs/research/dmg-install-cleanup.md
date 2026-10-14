# Reducing DMG installation cleanup

Investigated 2026-10-14 against Apple documentation, Apple's installed command
manual, and first-party open-source implementations. The implementation follows
the user's preference for silent cleanup; see [issue #4](https://github.com/jeremyc2/Gatto/issues/4).

## Implemented direction

Release preparation now creates the installer volume and records its unique
volume UUID in Gatto's Info.plist before signing/notarization. The completed DMG
preserves that UUID. This replaces the initial idea of inspecting source-bundle
files and signatures on mounted volumes: the installed app matches its own
recorded identity against system disk metadata instead.

After the window and menu bar are ready, a background worker acts only when the
actual executable and bundle are in `/Applications/Gatto.app` or the current
user's `Applications/Gatto.app`. It requires one current-user, read-only disk
image with the recorded UUID; rechecks its identity; and requests ordinary native
ejection. Absent, ambiguous, changed, or busy images are harmless. The app adds
no cleanup UI and leaves the downloaded DMG untouched. No persistent cleanup
setting is written: subsequent installed launches can retry, and an already
ejected disk is a no-op. Ordinary two-argument preview builds create a different
UUID and do not opt into this pairing.

Sources: [cleanup module](../../src/installer_cleanup.rs),
[packaging script](../../scripts/build-dmg.sh),
[release script](../../scripts/release.sh), and
[architecture](../ARCHITECTURE.md).

## Findings

The current behavior is ordinary for a drag-to-Applications disk image. Opening
the downloaded `.dmg` attaches a virtual disk and mounts its filesystem. The
visible **Install Gatto** disk and the downloaded `.dmg` file are separate things:
ejecting the mounted disk leaves the file on disk. An installed app is another
copy, outside that image. Apple describes disk images as files containing files
and folders and distinguishes their saved filename from the mounted name.
Sources: [Apple Disk Utility guide](https://support.apple.com/guide/disk-utility/dskutl11888/mac)
and Apple's local [hdiutil manual](/usr/share/man/man1/hdiutil.1),
`BACKGROUND` and `detach` sections.

The Applications icon inside Gatto's DMG is a symlink to `/Applications`; it does
not itself need ejecting. `scripts/build-dmg.sh` builds an HFS+ image, adds that
symlink and the app, and converts the image to compressed read-only UDZO.
`packaging/configure-dmg.applescript` sets up Finder during the build. It is not
an installer process running on the user's machine. `src/main.rs` has no
installation cleanup behavior. Sources: [build script](../../scripts/build-dmg.sh),
[Finder layout script](../../packaging/configure-dmg.applescript),
[startup](../../src/main.rs), [release guide](../RELEASING.md).

Gatto has one app-specific contributor to possible confusion: startup uses
`QuitMode::Explicit`, and closing a window hides and retains it. **Inference:**
if someone launches the copy on the image, closing its window can leave that
process running and keep the image busy. They should choose **Quit** from Gatto's
menu, then launch the copy in Applications. This does not establish that the
reported friction was caused by direct-image launch. Sources:
[startup](../../src/main.rs) and
[window-lifecycle ADR](../adr/0001-retain-macos-windows-to-avoid-touch-bar-crashes.md).

**Inference:** a passive image cannot notice the completion of a Finder copy by
itself. No supported image setting that runs an eject action after a Finder drag
was found. Doing cleanup at that moment would need additional running software
that observes or performs the installation. First successful launch from the
installed copy is a simpler trigger within this app's control.

## Permissions and failure cases

Apple gives ordinary Finder eject instructions without requiring an
administrator action. It identifies another app or user using files as a reason
ejection can fail. Thus normal same-user installer ejection should not require
special administrator permission; this is not a guarantee for other users'
mounts or managed Macs. Source:
[Apple storage/eject guide](https://support.apple.com/guide/mac-help/mchl027f1d66/mac).

Apple's `hdiutil detach` uses Disk Arbitration to unmount filesystems and detach
the image; `-force` ignores open files. The public Disk Arbitration APIs also
provide unmount and eject operations and callbacks that report failure. The
force-unmount option explicitly allows unmounting while files are active.
Sources: Apple's local [hdiutil manual](/usr/share/man/man1/hdiutil.1),
[DADiskUnmount](https://developer.apple.com/documentation/diskarbitration/dadiskunmount(_:_:_:_:)),
[DADiskEject](https://developer.apple.com/documentation/diskarbitration/dadiskeject(_:_:_:_:)),
[unmount options](https://developer.apple.com/documentation/diskarbitration/dadiskunmountoptions),
[eject callback](https://developer.apple.com/documentation/diskarbitration/dadiskejectcallback).

For the simple installed-app flow, AppKit also provides
`NSWorkspace.unmountAndEjectDevice(at:)`, accepting a volume URL, reporting errors,
and allowing use from any app thread. **Recommendation:** consider this public
native API before introducing a subprocess for ejection. Source:
[Apple NSWorkspace API](https://developer.apple.com/documentation/appkit/nsworkspace/unmountandejectdevice(at:)).

**Recommendation:** never force an installer eject. Skip or retry briefly when
the image is busy, including when the user launched the source app from the
image. An installed copy should only clean up after it has successfully started.
An ejection failure must not prevent the app from opening.

Deleting the backing download is a separate file operation. macOS protects
Downloads, Desktop, Documents, iCloud Drive, and network volumes with consent
rules. Download cleanup can therefore introduce a privacy prompt or fail even
when ejection works. Source:
[Apple file-access security guide](https://support.apple.com/guide/security/secddd1d86a6/web).
Sandboxed apps additionally require an appropriate file-access entitlement or
user-selected access; knowing a path does not confer permission. Sources:
[App Sandbox configuration](https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox)
and [sandbox file access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox).

The release script signs with hardened runtime and does not specify an App
Sandbox entitlement. This distinction does not remove macOS privacy protection.
Source: [release script](../../scripts/release.sh).

**Recommendation, revised for the user's preference:** prioritize silent automatic
ejection and leave the download alone in the initial implementation. Add no
cleanup buttons, instructions, or permission prompts. A future automatic Trash
operation would need to identify the exact backing file and establish that it
can operate with existing access, without prompting. Do not scan Downloads for
names such as `gatto*.dmg`, ask for Full Disk Access for cleanup, or use Finder
automation to work around file permissions.

## Real implementations

Chromium contains a self-install flow for apps launched from disk images. It asks
to install, copies the app, and launches a helper in the installed bundle with
the source image's device identity. Cleanup verifies disk-image media, obtains
the backing file, retries normal whole-disk unmount, ejects, and moves the image
file to Trash. It leaves cleanup incomplete on failure. Its backing-file lookup
uses IOKit; comments acknowledge the lack of a straightforward public API for
that mapping. This is evidence of feasibility, not a guarantee of prompt-free
cleanup under Gatto's permissions or after a Finder drag. Source:
[Chromium install_from_dmg.mm](https://chromium.googlesource.com/chromium/src/+/HEAD/chrome/browser/mac/install_from_dmg.mm),
particularly `LaunchInstalledApp`, `MediaResidesOnDiskImage`, and
`EjectAndTrashDiskImage` (retrieved 2026-10-14, blob
`b822ec667f7a05b75312e275c7d3af0acf75d6c3`).

LetsMove is another implementation: after copying/relaunching it schedules a
delayed non-forced `hdiutil detach`. It identifies the containing device with
`statfs` and `hdiutil info -plist`. Its old code also contains quarantine removal
and Finder AppleScript fallbacks; those are not recommendations for this app.
Source:
[Potion Factory LetsMove](https://github.com/potionfactory/LetsMove/blob/master/PFMoveApplication.m),
`ContainingDiskImageDevice`, the delayed-detach block, and `Relaunch`.

## Identification and launch design

Apple's installed manual documents `hdiutil info -plist` as information about
attached images. On the investigated Mac, its output includes `image-path`,
`owner-uid`, and `system-entities` with `mount-point`/`dev-entry` fields.
This schema observation is local evidence, not a permanent API contract. The
same manual now prefers `diskutil` replacements for multiple image operations;
the exact command availability must be checked against Gatto's macOS 13 minimum.
Sources: Apple's local [hdiutil manual](/usr/share/man/man1/hdiutil.1),
read-only `hdiutil info -plist` inspection, and
[minimum version](../../packaging/Info.plist).

**Initial proposal, superseded by the metadata-only UUID pairing above:**

1. Act only once per installed release after successful startup, when the app is
   running from an approved installed location rather than a mounted image.
2. Enumerate actual attached disk images, not arbitrary removable disks.
3. Require a single unambiguous current-user candidate containing the expected
   app. Validate its bundle identifier, release/build identity, valid signature,
   and expected signing identity against the running installed app. The display
   name **Install Gatto** alone is insufficient; duplicate volume names and old
   images are possible. Future packaging could add a versioned installer marker.
4. Retain the resolved device, mount, and exact backing-file identity. Recheck
   immediately before ejecting because a device identifier can be reused.
5. Request normal whole-image ejection asynchronously with a bounded wait.
   Fail quietly; never force it or show a manual-eject hint.
6. Leave the backing download untouched in the initial implementation. Do not
   turn a cleanup failure into a user task or a request for more access.

This is conservative identification of a likely installer; a plain Finder copy
does not provide this app with a definitive source-image handoff. When multiple
equally valid candidates exist, skip automatic cleanup. A self-install workflow
can carry the source identity explicitly, as Chromium does, but adds copy,
overwrite, authorization, and relaunch work.

Reading source-bundle metadata on mounted volumes is also file access, so this
proposal does not establish that discovery will always be prompt-free. Testing
must include the signed app and fresh privacy state. **Recommendation:** if the
needed identification would introduce a new permission prompt, skip automatic
cleanup rather than expanding access merely to remove installation clutter.

App Translocation complicates self-install and source-image detection. Apple DTS
says downloaded apps may execute from randomized paths, the triggering conditions
are undocumented and have changed, and there is no supported way to detect
translocation or recover the original path. **Recommendation:** preserve the
Finder drag flow and do not rely on private translocation APIs or treat every
non-Applications launch as a disk-image launch. Source:
[Apple DTS App Translocation Notes](https://developer.apple.com/forums/thread/724969).

## Suggested scope

The user prefers to avoid additional user-facing instructions and manual effort.

**Chosen scope:** retain the existing drag-to-Applications presentation and eject
the paired installer silently after installed launch. Add no instructions,
cleanup controls, dialogs, or manual recovery tasks. Downloaded-file cleanup
remains outside this implementation. UUID pairing avoids opening mounted-volume
files for identification. Verify the complete browser-downloaded release with
fresh privacy state before shipping.

Before shipping, verify a browser-downloaded notarized release on supported
macOS versions: installed launch, direct image launch, translocated launch,
existing/new app versions, duplicate mounted images, busy volume, already-ejected
volume, another user's mount, protected backing-file folders, and denied
file-access permission. This research establishes feasibility; it is not an
end-to-end validation of Gatto's future implementation.

## Local verification

On 2026-10-14, on a Mac reporting macOS 27.0.1, the coordinating agent created isolated tiny read-only test images
and mounted/ejected them as the existing unprivileged user (UID 501). Normal
`hdiutil detach` succeeded without administrator authentication. A process whose
current working directory was on the volume made detach refuse with
`Resource busy` (exit 16); releasing it allowed normal detach to succeed. A
separate native `NSWorkspace.unmountAndEjectDevice(at:)` check succeeded too.
Ejection preserved each backing `.dmg`. Only owned temporary test images/files
were cleaned up; the pre-existing user's **Install Gatto** mount was left alone.

These are local experimental observations, not web-source claims. The tests ran
from the terminal context, not released Gatto, and do not establish TCC behavior
for the signed app, all supported macOS versions, or other users' mounts.
Canonicalization was needed for `/var` versus `/private/var` mount paths.

Implementation checks also passed with a disposable Developer ID signed bundle
using the production cleanup module and an isolated home/Applications folder.
The packaging script's prepared UUID survived compression, and the copied app's
resource signature verified. Installed launch ejected the idle paired image and
preserved its backing file; direct image launch, duplicate UUID mounts, busy
volumes, and already-ejected images behaved as intended. Pre-existing mounts
were untouched. These checks used a signed verification executable, not a
browser-downloaded notarized release. An additional check packaged and signed the
actual optimized Gatto executable, launched it with a disposable home/Applications
folder, and confirmed normal window/menu-bar startup, automatic ejection, and
continued application operation. The downloaded image remained. Preparation of
an already sealed app was also rejected without changing its Info.plist.
The browser-download/notarization and other supported-OS release checks above
still apply.

The full Rust suite (43 tests), formatting, all-target checking, Clippy with all
features and warnings denied, and the repository's dependency policy checks
passed. Dependency checks retain existing duplicate-version/license-allowance
warnings. No dependency versions changed.
