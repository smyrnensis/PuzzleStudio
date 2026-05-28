# Upstream Checkout

Place the PS Next working copy here:

```bash
git clone https://github.com/david-pfx/PuzzleScriptNext.git PS_EXTRACTION/upstream/PuzzleScriptNext
```

`PS_EXTRACTION/upstream/PuzzleScriptNext/` is ignored by the parent repository
so the external source tree does not get mixed into PuzzleBuilder commits.

If normal clone is unstable, use a local fork checkout created outside this
repository and copy or move it to this path.
