# EffectCraft file interchange

Use `exec` with the chosen command and an explicit `path`. After importing, save the `.ecproj` and check the imported comp and media. For exports, check the returned report or warnings and confirm the output exists.

- **After Effects** uses `file.importAep` for `.aep` or `.aepx` and `file.exportAep` for `.aep`. Run with `dryRun: true` to inspect the loss report before transferring. Set `allowLosses: true` only when the reported losses are acceptable for the task.
- **Lottie** uses `file.importLottie` and `file.exportLottie` for `.json` or `.lottie`. Check export `warnings` for features that a Lottie player cannot reproduce.
- **Timeline interchange** uses `file.importTimeline` and `file.exportTimeline`. Use Final Cut Pro XML (`.xml`) for Premiere Pro interchange. Use the requested extension for FCPXML, OTIO, EDL, AAF, or OMF. Check `missing` for media placeholders after import and `warnings` plus `prerendered` after export. Keep any generated media alongside the timeline. `prerender` controls whether to bake unsupported layers, all layers, or none.
