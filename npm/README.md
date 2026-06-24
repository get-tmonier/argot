<p align="center">
  <strong>argot</strong> — like ESLint, but for the unwritten rules.
</p>

<p align="center">
  <a href="https://argot.tmonier.com"><strong>argot.tmonier.com</strong></a>
  &nbsp;·&nbsp;
  <a href="https://argot.tmonier.com/docs/">Documentation</a>
  &nbsp;·&nbsp;
  <a href="https://github.com/get-tmonier/argot">GitHub</a>
</p>

---

**argot** is a voice linter. It learns your repo's voice from its own git history, then flags the
hunks whose token distribution diverges from the learned norm — the code that's valid, typed, and
lint-clean, but doesn't sound like anyone on your team wrote it. No model, no cloud, no GPU.

## Install

```sh
npm install -g @tmonier/argot
```

> Requires [`uv`](https://docs.astral.sh/uv/) on your `PATH` for the local scoring engine. The
> [curl installer](https://argot.tmonier.com/docs/) adds it automatically.

## Use

```sh
cd your-repo
argot extract      # walk git history → .argot/dataset.jsonl
argot fit          # build the repo corpus + baseline, then calibrate the threshold
argot check        # score uncommitted changes (or pass a ref/range)
```

Run `extract` and `fit` once; run `check` on every diff. It exits non-zero on a hit, so it drops
into CI like any other linter.

Full guides, the scoring model, and supported languages live at
**[argot.tmonier.com](https://argot.tmonier.com)**.

## License

MIT
