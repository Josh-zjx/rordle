# WASM builds and publication

CI builds and tests automatically. Publication happens only when you manually
run **Release WASM**. A push, pull request, or new tag does not publish a release
or change the live game.

## One-time GitHub setup

1. Commit these files and push them to `main`.
2. In [Settings → Pages](https://github.com/Josh-zjx/rordle/settings/pages),
   select **GitHub Actions** as the build and deployment source.
3. The deployment uses the `github-pages` environment. If you configure its
   deployment branches, allow `main`. Additional required reviewers are optional;
   manually running the workflow is the approval step for this setup.

The workflows use the repository's built-in `GITHUB_TOKEN`; no personal access
token or custom secret is needed. GitHub documents the Pages setup and required
deployment permissions in its [custom workflows guide](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

## Automatic validation

`.github/workflows/rust.yml` appears as **CI** in Actions. Relevant branch pushes
and pull requests run native builds and tests on Linux, Windows, and macOS, the
full solver regression benchmark, formatting and Clippy, and release-tool tests.
Its reusable WASM workflow checks the target, builds with SIMD enabled, installs
the `wasm-bindgen` CLI version from `Cargo.lock`, and runs a Chromium smoke test
of startup, typing, the touch keyboard, hints, submission, and reset.

The WASM job uploads `rordle-wasm-<commit SHA>`, containing:

- `rordle-web.zip`: the HTML, JavaScript bindings, WASM binary, and `build.json`.
- `SHA256SUMS`: the archive checksum.

`build.json` records the source commit, CI run ID, and checksums of the three web
files. Unpublished candidates remain available for 30 days. The CI summary shows
the run ID; the same ID appears at the end of its Actions URL.

## Publish an approved build

1. Wait for the entire **CI** run on `main` to succeed.
2. Open **Actions → Release WASM → Run workflow**, using the `main` branch.
3. Enter a new version such as `v0.1.0`, and the successful CI run ID.
4. Run the workflow. This is the publication approval.

The workflow verifies that the run belongs to this repository's CI workflow,
was a successful default-branch push or manual run, and has an unexpired bundle.
It checks the archive and file checksums and source provenance, then publishes
that exact archive without recompiling. Existing release tags are rejected.

A GitHub Release contains the archive and checksum. GitHub Pages serves:

- Latest approved game: `https://josh-zjx.github.io/rordle/`
- Pinned version: `https://josh-zjx.github.io/rordle/releases/v0.1.0/`

The publication summary shows the actual URLs, including any custom Pages domain.
Each deployment restores older versions from their published `rordle-web.zip`
release assets. Keep those releases and assets to preserve their URLs across
future deployments. The existing automatic native tag-release workflow is
replaced by this manually dispatched web publication workflow.

If publication fails, use **Re-run failed jobs** on that run. In particular, a
Pages failure can be retried after the GitHub Release has already succeeded.
Draft release creation and upload can also be resumed. Do not start a fresh
release with an already published version.

## Embed in a blog

Use a Pages URL for the playable game and GitHub Releases for downloads. Pin the
version when the article should continue demonstrating that exact release:

```html
<iframe
  src="https://josh-zjx.github.io/rordle/releases/v0.1.0/"
  title="Play Rordle"
  loading="lazy"
  width="100%"
  height="1200"
  style="border: 0"
></iframe>
```

Use the root URL instead to follow the latest manually approved publication.

## Local checks

```bash
scripts/build-wasm.sh
npm ci
npx playwright install chromium
npm run test:web
python3 -m unittest discover -s scripts -p '*_test.py'
node --test tests/release-validation.test.cjs
```

For an existing Chromium installation, set `PLAYWRIGHT_CHROMIUM_EXECUTABLE` when
running `npm run test:web`. Set `WEB_ROOT` to test an unpacked release or a Pages
version directory. Browser test dependencies are development tools and are not
included in the published site.
