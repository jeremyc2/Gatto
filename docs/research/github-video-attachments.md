# GitHub video attachments and dimensioned images

Investigated 2026-10-19 against GitHub documentation, GitHub CLI source,
GitHub's live Markdown renderer, and the current Rust app. No attachments were
uploaded and no issues, pull requests, or comments were published during this
investigation. Rendering probes used GitHub's non-publishing Markdown endpoint.

The findings below record the investigation before implementation. Current
video uploads, GIF playback, streaming, limits, and external macOS previews are
documented in the [walkthrough](../WALKTHROUGH.md) and [architecture](../ARCHITECTURE.md).

## Finding

**Video upload and copy is feasible through the same attachment service Gatto
already uses.** GitHub supports video in pull request descriptions and other
Markdown surfaces. Its first-party CLI sends images and videos through one
uploader to `uploads.github.com/user-attachments/assets`, with `name`,
`content_type`, and `repository_id` query parameters, and receives a JSON `url`.
This matches Gatto's upload route. These are attachment assets, not files added
to the Git repository. Sources: [GitHub video announcement](https://github.blog/changelog/2021-05-13-video-uploads-now-generally-available/),
[first-party uploader source](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/client.go),
[asset model](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/userasset.go),
and [Gatto uploader](../../src/github.rs).

**At investigation time, Gatto's preview could not play video.** It accepts image formats, decodes
their pixels, and draws an `Image` with `window.paint_image`; it has no media
timeline, audio output, or playback controls. Adding accepted file extensions
alone would not produce a working preview. Sources: [model](../../src/model.rs),
[preview](../../src/image_preview.rs), [dependencies](../../Cargo.toml).

## Limits and playback

| Concern                          | Established behavior                                                                                                                                                                             |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Containers                       | GitHub accepts `.mp4`, `.mov`, and `.webm`.                                                                                                                                                      |
| Free repositories                | Videos up to 10 MB when the repository owner is on a free plan.                                                                                                                                  |
| Paid repositories                | Videos up to 100 MB when the repository owner is on a paid plan. For uploads above 10 MB, the uploader must also be an organization member, outside collaborator, or on a paid plan.             |
| Images                           | 10 MB per image/GIF.                                                                                                                                                                             |
| Codec                            | Browser dependent; GitHub recommends H.264 for greatest compatibility. A `.mov` extension alone does not guarantee playback.                                                                     |
| Duration, resolution, frame rate | The cited GitHub attachment documentation publishes no separate bounds. No fixed time limit was established; this is not a guarantee of unlimited duration. File size remains a practical limit. |
| Access                           | Public-repository uploads can be viewed without authentication. Private/internal uploads require access to the associated repository.                                                            |

Source for the table: [GitHub attachment documentation](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files).
The CLI implementation uses binary byte bounds of `10 * 1024 * 1024` for images
and `100 * 1024 * 1024` for videos, leaving the actual video plan allowance to
the server. Source: [first-party asset validation](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/userasset.go).

**Observed GitHub playback default: initially muted, with controls, and no
autoplay.** On the investigation date, the server-rendered video in
[this existing public PR description](https://github.com/JuliaPluto/Pluto.jl/pull/3201)
had `controls="controls" muted="muted"` and no `autoplay` attribute. The PR's
stored body contains only the bare `github.com/user-attachments/assets/...`
URL in its own paragraph. This directly observes GitHub's generated player;
it is not a documented permanent default or proof that every client behaves
identically. The browser controls allow viewers to unmute audio when the file
contains a supported audio track. Muting is a playback setting, not evidence
that GitHub removes audio from the upload. Sources: GitHub's rendered HTML at
the linked PR and [video control/muting semantics](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/video).

## Video insertion syntax and sizing

For ordinary PR descriptions, copy the returned attachment URL as a separate
paragraph, with blank lines around it when surrounding text exists:

```markdown
Recording of the interaction:

https://github.com/user-attachments/assets/RETURNED-ASSET-ID
```

GitHub promotes this to a player. Its first-party CLI deliberately emits bare
URLs for videos and Markdown image syntax for images; inline video links stay
links. Sources: [video asset implementation](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/userasset.go),
[reference fixtures](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/testdata/references_expected.md),
and the existing PR above. Do not wrap video URLs in `![alt](...)`.

An explicit HTML video is a possible way to request dimensions:

```html
<video
  src="https://github.com/user-attachments/assets/RETURNED-ASSET-ID"
  width="640"
  height="360"
  controls
  muted
></video>
```

A read-only probe of GitHub's [Markdown rendering API](https://docs.github.com/en/rest/markdown/markdown)
on the investigation date preserved `src`, `width`, `height`, `controls`, and
`muted`, and removed `autoplay`, `loop`, `playsinline`, and `preload`. This
confirms accepted HTML attributes in that renderer, not a permanent contract
for all PR clients. The same API leaves a bare attachment URL as a link, whereas
the actual PR page turns it into a player. Therefore that generic API alone is
insufficient to test GitHub's attachment unfurling. Prefer GitHub's native bare
URL convention initially; validate explicit video sizing in a PR preview before
promising it as a product feature.

## Dimensioned image HTML

GitHub's own Skills lesson teaches `<img>` with `width` and/or `height` in pixels
because ordinary Markdown image syntax cannot specify size. Gatto can emit:

```html
<img
  width="640"
  height="360"
  alt="Description"
  src="https://github.com/user-attachments/assets/RETURNED-ASSET-ID"
/>
```

Escape user-provided description and URL attribute values. A rendering probe
preserved both dimensions and added responsive maximum-size styling. This
controls display size without resizing uploaded image bytes. Sources:
[GitHub Skills image lesson](https://github.com/skills/communicate-using-markdown/blob/main/.github/steps/4-add-an-image.md)
and the Markdown rendering API linked above. The user's reported recent change
to paste insertion behavior is consistent with this supported syntax; no dated
first-party announcement establishing when the paste UI changed was found.

Gatto already knows validated image pixel dimensions. Those are available for
HTML output. They do not necessarily equal the source application's logical
display size: a Retina screenshot can contain twice as many pixels as display
points. The currently consumed GPUI `Image` supplies image format/bytes, without
the original source view's layout dimensions. Exact fidelity to arbitrary
copied display size would need additional clipboard metadata. Source:
[Gatto clipboard conversion and dimensions](../../src/model.rs).

## CLI/API support and implementation direction

GitHub made `gh --attach` generally available in CLI v2.99.0 on 2026-09-01 for
issue/PR create, edit, and comment commands. Installed `gh` v2.102.0 exposes this
flag. Those commands write a body/comment; no standalone attachment-upload
command appeared in its command inventory. Consequently, Gatto should retain
its direct upload-and-copy flow rather than creating a throwaway comment to
obtain a URL. Sources: [official CLI announcement](https://github.blog/changelog/2026-09-01-github-cli-media-in-issues-pull-requests-and-comments/),
[CLI PR edit manual](https://cli.github.com/manual/gh_pr_edit), and local
`gh --version`, `gh help`, and `gh pr edit --help` inspection.

The announcement guarantees OAuth/classic PAT credentials and repository write
access. Current first-party uploader source also allows fine-grained PATs and
some user-to-server tokens, with the endpoint deciding eligibility. Its allowed
permissions are `WRITE`, `MAINTAIN`, and `ADMIN`; `READ`/`TRIAGE` can yield 404.
GitHub Enterprise Server is excluded. Treat source-observed token additions as
version-specific rather than extending the published guarantee. Source:
[first-party client checks](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/client.go).
No standalone REST-reference contract for the attachment endpoint was
established, although the supported CLI now uses it directly.

**Recommended video work:** introduce image/video variants with separate
validation, metadata, snippet generation, and preview paths. Accept video file
paths through drop, chooser, or clipboard file entries. The present clipboard
image entry represents raster image data, not an arbitrary video payload.
Retain failed-item retry and uploaded URL recovery. Source: [current app input
and upload flows](../../src/app.rs), [Quick Copy](../../src/quick_copy.rs).

For native playback on macOS, Apple's `AVPlayer` with an `AVPlayerView` is a
feasible integration path: Apple provides the media player and controls, while
Gatto would need the native view/window bridge and lifecycle management. This
is a proposed integration, not an existing GPUI preview capability. Sources:
[AVPlayer](https://developer.apple.com/documentation/avfoundation/avplayer),
[AVPlayerView](https://developer.apple.com/documentation/avkit/avplayerview),
[current preview](../../src/image_preview.rs).

Before supporting 100 MB clips, change the current full-file buffering and
upload-body clone to file-backed streaming or another bounded approach; revisit
the fixed 90-second request timeout and add progress/cancellation suitable for
larger media. Preserve original audio/bytes unless conversion is explicitly
requested. The existing first-party CLI streams files instead of cloning them.
Sources: [Gatto uploader](../../src/github.rs), [Gatto staging](../../src/model.rs),
[CLI uploader](https://github.com/cli/cli/blob/ed40e856ffecd31286733876c613a2d3745e12bd/internal/attachments/client.go).

Remaining validation before shipping video: a real authorized upload to the
selected repository, server errors around the applicable plan limit, H.264 MP4
and common MOV/WebM codecs, audio/unmute behavior, long clips that fit the byte
limit, mixed-media batches/retries, native preview lifecycle, and private-repo
viewing. This investigation intentionally did not upload sample media.
