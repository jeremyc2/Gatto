## Summary

Describe the problem and the solution in a few sentences.

## Related issue

Link the issue, if one exists.

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets`
- [ ] `cargo deny --locked check --hide-inclusion-graph advisories licenses bans sources`
- [ ] I manually verified the affected workflow, when applicable.

## UI changes

Add screenshots or a short recording for visible changes, or write “Not applicable.”

## Checklist

- [ ] The change is focused and does not include unrelated work.
- [ ] I added or updated tests for behavior changes.
- [ ] I updated documentation and the changelog where appropriate.
- [ ] I did not include credentials, private repository content, or generated build output.
