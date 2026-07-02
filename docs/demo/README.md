# README demo GIF

`demo.gif` is the animated demo at the top of the project README. It runs
`argot check --staged` against a real, calibrated model and shows the colored
severity glyph, the `↳` evidence line, and the hunk body — the terminal UX *is*
the product screenshot, so it's scripted and re-renderable rather than
hand-recorded.

## Files

- **`demo.tape`** — the [VHS](https://github.com/charmbracelet/vhs) script (styling + the typed command).
- **`receipts.py`** — the planted out-of-voice hunk (wrong exception type + a novel-token helper). The same hunk the README quotes verbatim.
- **`render.sh`** — reproducible driver: fits argot on the pinned FastAPI benchmark checkout, plants `receipts.py`, and records the GIF.

## Re-render

Whenever `argot check`'s output format changes, regenerate so the GIF can't
drift like a hand-written text sample:

```sh
brew install vhs        # once — pulls ttyd + ffmpeg
just build              # ensure target/release/argot is current
docs/demo/render.sh     # clones FastAPI on first run, then records demo.gif
```

The hit shown is byte-identical to the sample in the README, because both come
from the same binary against the same pinned checkout.
