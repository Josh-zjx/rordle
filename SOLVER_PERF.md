# Solver correctness and performance

The solver now uses the same duplicate-aware Wordle grading as the game for
simulation, filtering, and entropy partitions. Earlier benchmarks in this file
used letter-presence-only feedback; their timings and guess sequences are not
a correctness baseline for this version.

## Current results

A release run over all 2,309 answers produced:

- 9,511 total guesses, averaging 4.119099177132958.
- Zero games exceeding six guesses and zero unsolved games.
- About 6.1 seconds for the full benchmark in the review environment, with two
  available CPUs. Timings depend on hardware and system load.
- A fresh-process solve of `cigar` took roughly 28–61 ms, including word-list
  loading, second-guess computation, and the remaining guesses.

The first second-guess request now computes only the observed opening pattern.
Previously, even a single game synchronously populated the complete table;
the review measured approximately 1.5 seconds for that first lookup. A complete
batch still needs many patterns, and proper duplicate grading does more work
per pair than the former bitset approximation.

## Design

- Validated `Word` values enforce exactly five lowercase ASCII letters.
  The shared grader consumes exact matches before misplaced letters.
- The immutable word list is shared across games. Its solver-only array of
  validated words is initialized lazily and is excluded from GUI-only/WASM
  builds.
- Candidate indices are filtered in place. Reset clears and refills existing
  storage; there is no parallel validity bitmap or redundant survivor count.
- One scoring implementation serves both fresh searches and cached results.
  Highest entropy wins, with the lowest dictionary index breaking ties.
- Each of the 243 opening patterns has its own lazy cache entry. Cache
  initialization does not hold locks while running Rayon work, avoiding nested
  pool deadlocks. Concurrent misses may compute the same entry independently.
- Single-game searches score guesses in parallel. The batch benchmark
  distributes games across Rayon workers and scores each worker's game
  serially, avoiding nested scheduling. Statistics use local accumulation and
  reduction, without shared mutex counters.
- Normal debug builds do not recompute and print a full candidate ranking.

The complete allowed-guess dictionary intentionally remains the initial
hypothesis pool. An empty pool intentionally returns the lowest-index guess
with zero entropy. Neither behavior changed.

## Reproduce

```bash
cargo test --locked --all-targets --features solver
cargo clippy --locked --all-targets --features solver -- -D warnings
cargo run --locked --release --features solver --bin solver -- --check
cargo run --locked --release --features solver --bin solver -- --word cigar
```

The native-only `solver` feature enables the solver module and binary.
The benchmark's `--check` mode exits unsuccessfully on any unsolved game, any
game exceeding six guesses, or an average above 4.2. Unsolved games contribute
their actual attempted guesses (capped at 21) to the reported average.

Tests cover malformed inputs, an independent duplicate-grading reference over
59,049 word pairs, external feedback accumulation, preserved candidate-pool
behavior, lazy caching, deterministic search, and updated golden solve traces.
