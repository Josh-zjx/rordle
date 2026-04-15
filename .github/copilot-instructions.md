# Copilot instructions for `rordle`

## Build, test, and lint commands

| Purpose | Command | Notes |
| --- | --- | --- |
| Build the desktop app | `cargo build --bin game` | Main CI build target. Also runs `src/build.rs`, which compiles `ui/window.slint`. |
| Build the release binary | `cargo build -r --bin game` | Matches the release workflow. |
| Run the test suite | `cargo test --bin game` | Matches the CI test target. |
| Run a single test | `cargo test --bin game <test_name> -- --exact` | Use Cargo's test-name filter for targeted runs. |
| Lint | `cargo clippy --bin game` | Matches the CI lint target. |
| Exercise the solver path | `cargo run -r --bin solver` | CI uses this as the solver regression run. |

## High-level architecture

- This crate does not have a `lib.rs`. Shared logic lives in `src/game.rs` and `src/solver.rs`, and both binaries wire those modules in directly: `src/play.rs` builds the `game` binary and `src/roget.rs` builds the `solver` binary.
- `src/build.rs` compiles `ui/window.slint` during Cargo builds. `src/play.rs` then pulls the generated Slint types in with `slint::include_modules!()` and drives the UI directly from Rust callbacks.
- `src/game.rs` owns the core game state and rules: loading answers/candidates, validating guesses, grading guesses, tracking rounds, and advancing the game state. The desktop UI and the solver both call into that same flow, so gameplay changes in `src/game.rs` affect both binaries.
- `src/play.rs` acts as the controller for the Slint app. It keeps a `VecModel<CharItem>` for the 6x5 board, updates `MainWindow` properties for coarse state (`level`, `index`, `success`, `failed`, `invalid`), and translates keyboard events into `Guess` values.
- Runtime data is file-based, not embedded. `Game::new()` opens `./data/answer` and `./data/candidate`, and `Solver::bind()` opens `./data/cache`. Those files are JSON arrays stored without extensions, so commands should be run from the repo root or another working directory that preserves those relative paths.
- CI is split by responsibility: `.github/workflows/rust.yml` builds/tests/lints the `game` binary and separately runs `solver` in release mode, while `.github/workflows/release.yml` only publishes the `game` executable.

## Key conventions

- Keep shared gameplay behavior in `src/game.rs` unless a change is truly UI-only or solver-only. The binaries are thin entry points over the same game and guess/match types.
- Preserve the input/display split in `src/play.rs`: tiles store uppercase display text, but submitted guesses are converted to lowercase before `check_valid_guess` and `grade_guess`.
- When changing turn progression or board updates, treat the Slint window properties and the `VecModel<CharItem>` as two parts of the same state machine. Most gameplay changes require coordinated updates to both.
- UI edits flow through Cargo builds. If `ui/window.slint` changes, rebuild with Cargo so the generated Slint bindings stay in sync with `src/play.rs`.
- Repo-specific non-Rust automation lives in `.pre-commit-config.yaml`: whitespace/EOF/YAML checks plus `oxipng` for PNG assets. If a change touches workflow YAML or PNGs, expect those hooks to matter.
