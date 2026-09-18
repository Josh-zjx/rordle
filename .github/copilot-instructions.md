# Repository conventions

Read `CLAUDE.md` for architecture, build commands, and solver invariants.

- Shared code lives in `src/lib.rs`, `src/game.rs`, and `src/solver.rs`.
  Desktop callbacks live in `src/ui.rs`; both binaries have separate entry points.
- Run `cargo test --locked --all-targets --features solver` to include the
  library tests and solver statistics tests. Testing only `--bin game` runs
  no unit tests.
- Run `cargo clippy --locked --all-targets --features solver -- -D warnings`
  and `cargo fmt --check` for linting.
- The native solver requires `--features solver`; use
  `cargo run --release --features solver --bin solver -- --check` for the
  full regression benchmark.
- Keep WASM builds free of the native solver feature. Verify with
  `cargo check --locked --target wasm32-unknown-unknown --all-targets`.
- Word data is embedded. Runtime execution does not require a particular
  working directory or the historical `data/cache` file.
- Core word APIs validate five lowercase ASCII letters and return errors for
  malformed input. UI tiles display uppercase; submitted guesses are lowercase.
- Game grading, solver scoring, and candidate filtering share duplicate-letter
  semantics. Do not introduce a separate simplified grading implementation.
- The complete guess dictionary as hypothesis pool and the empty-pool fallback
  are intentional solver behavior.
- UI changes may require coordinated updates to `src/ui.rs` and
  `ui/window.slint`, followed by a rebuild.
