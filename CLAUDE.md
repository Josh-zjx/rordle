# Project guide

Rordle is a Rust Wordle game with a Slint desktop UI, a WASM library, and an
optional native solver benchmark.

## Commands

```bash
cargo build --locked --bin game
cargo run --bin game
cargo test --locked --all-targets --features solver
cargo clippy --locked --all-targets --features solver -- -D warnings
cargo fmt --check
cargo run --release --features solver --bin solver -- --check
cargo run --release --features solver --bin solver -- --word cigar
cargo check --locked --target wasm32-unknown-unknown --all-targets
scripts/build-wasm.sh
```

The solver module and binary require the native-only `solver` feature. GUI and
WASM builds do not need it. All word data is embedded at compile time; executables
can run from any working directory.

## Architecture

- `src/game.rs`: validated `Word` values, shared duplicate-aware grading, game
  state, and a shared immutable word list. String-based grading and answer
  setters return `Result`; malformed input leaves game state unchanged.
- `src/solver.rs`: entropy scoring over all allowed guesses, incremental
  filtering, and a lazy second-guess cache. `Solver::bind` is infallible.
  `add_pattern` validates the word and immediately applies feedback.
- `src/roget.rs`: native benchmark. Rayon distributes games; worker-local
  statistics are reduced without shared counters. `--check` enforces regression
  limits, and `--word WORD` prints one solve trace.
- `src/ui.rs`: Slint callbacks and the flat 30-cell board model.
- `src/play.rs`: desktop entry point.
- `src/web.rs`: WASM startup and panic hook.
- `src/build.rs` and `ui/window.slint`: compile-time Slint UI integration.
- `scripts/build-wasm.sh` and `web/index.html`: WASM packaging and browser host.

## Solver invariants

The complete allowed-guess dictionary intentionally remains the initial answer
hypothesis pool. An empty pool intentionally retains the lowest-index,
zero-entropy guess fallback. Do not replace either behavior.

The game, candidate filtering, and entropy partitions use the same Wordle
duplicate-letter rules. The second-guess cache is populated per observed
`tares` pattern, and only applies after exactly one observation. Filtering
retains prior constraints; reset reuses vector capacity.

## Data and verification

`data/answer` and `data/candidate` are embedded JSON word arrays. Loading
validates five lowercase ASCII letters per word and a nonempty answer list.
`data/cache` is a historical unused file.

Library tests cover malformed inputs, duplicate grading, externally supplied
feedback, candidate retention, cache correctness, and representative solves.
The native benchmark checks the entire answer list. See `SOLVER_PERF.md` for
measurement commands and the current algorithm.
