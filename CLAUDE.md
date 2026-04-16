# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rordle is a Rust implementation of Wordle with two binaries:
- `game` — a GUI Wordle game built with Slint
- `solver` — a CLI solver that benchmarks the solving algorithm against all answer words

## Commands

```bash
# Build
cargo build
cargo build --release

# Run the GUI game
cargo run --bin game

# Run the solver benchmark (solves all ~2300 answer words in parallel, prints stats)
cargo run --bin solver --release

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Lint
cargo clippy
```

**Important:** All binaries must be run from the repo root. The game and solver load word lists from `./data/answer`, `./data/candidate`, and `./data/cache` at runtime relative to the current working directory.

## Architecture

### Modules

- `src/game.rs` — Core game logic: `Game`, `Match`, `GuessState`, `GameState`, `WordList`. `WordList` is loaded once via a `OnceLock<Arc<WordList>>` singleton and shared across instances.
- `src/solver.rs` — The `Solver` struct that implements entropy-based word selection. Uses a precomputed `valid_table` (bool vec indexed by candidate position) to track remaining candidates after each guess.
- `src/play.rs` — Binary entry point for the GUI. Wires Slint callbacks (`on_handle_keyboard`, `on_reset`) to `Game` methods.
- `src/roget.rs` — Binary entry point for the solver benchmark. Runs `solve_all()` using 8 threads via `std::thread`.
- `src/build.rs` — Build script: compiles `ui/window.slint` via `slint_build`.
- `ui/window.slint` — Slint UI definition for the 6×5 character grid and status messages.

### Solver Algorithm

The solver selects guesses using information entropy: for each candidate word, it computes the Shannon entropy of the distribution of `grade_pair()` outcomes across all still-valid candidates. The opening word is hard-coded as `"tares"`. After each guess, `add_pattern()` stores the result and `filter_valid_word()` eliminates incompatible candidates.

### Data Files

- `data/answer` — JSON array of valid answer words
- `data/candidate` — JSON array of additional valid guesses (non-answer words); answers are appended to candidates at load time
- `data/cache` — Read by `Solver::bind()` (currently unused in logic but required to exist)

### Slint UI Integration

The `.slint` file is compiled at build time by `slint_build`. The generated Rust types (e.g. `MainWindow`, `CharItem`) are included via `slint::include_modules!()` in `play.rs`. UI state is driven by a flat `Vec<CharItem>` of 30 elements (6 rows × 5 columns), indexed as `row * 5 + col`.
