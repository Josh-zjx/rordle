# Solver correctness and performance

The solver now uses the same duplicate-aware Wordle grading as the game for
simulation, filtering, and entropy partitions. Earlier benchmarks in this file
used letter-presence-only feedback; their timings and guess sequences are not
a correctness baseline for this version.

## Current results

A release run over all 2,309 answers produced:

- 9,403 total guesses, averaging 4.072325682113469 (previously 9,511 and
  4.119099177132958, before ties preferred possible answers).
- Zero games exceeding six guesses and zero unsolved games.
- About 2.5 seconds for the full benchmark on an i7-9700K with two available
  CPUs, down from 6.1 seconds on the same machine before the grading kernel
  (62 G instructions and 100 M branch misses, previously 162 G and 438 M).
  Timings depend on hardware and system load.
- A fresh-process solve of `cigar` took roughly 8–9 ms there, including
  word-list loading, second-guess computation, and the remaining guesses.

The first second-guess request now computes only the observed opening pattern.
Previously, even a single game synchronously populated the complete table;
the review measured approximately 1.5 seconds for that first lookup. A complete
batch still needs many patterns, and proper duplicate grading does more work
per pair than the former bitset approximation.

## Design

- Validated `Word` values enforce exactly five lowercase ASCII letters.
  The shared grader consumes exact matches before misplaced letters.
- The immutable word list is shared across games. Its solver-only array of
  validated words is initialized lazily, on the first solver search.
- Candidate indices are filtered in place. Reset clears and refills existing
  storage; there is no parallel validity bitmap or redundant survivor count.
- Each search transposes the candidate pool into five letter columns padded
  to 16-answer chunks. A branchless kernel grades one guess against a whole
  chunk in `u8` lanes, which LLVM vectorizes (SSE2 on the x86-64 baseline,
  simd128 on WASM via `.cargo/config.toml`). It encodes the same duplicate
  rules as `Word::grade`, and tests check the two agree exhaustively over a
  three-letter alphabet and across the dictionary. Sixteen lanes measured
  faster than 32 even with AVX2, because late-round pools are small and pad
  whole chunks; `target-cpu=native` gained only about 4%.
- The entropy sum visits only occupied buckets, via a bitmap, in ascending
  pattern order. Results are bit-identical to scanning all 243 buckets, and the
  scan's unpredictable zero-count branch no longer dominates small pools.
- One scoring implementation serves both fresh searches and cached results.
  Highest entropy wins. Among equal-entropy guesses, one still consistent
  with the feedback wins, because it may end the game immediately; the
  lowest dictionary index breaks any remaining tie. Answers are stored after
  the non-answer words, so an index-only tie-break always chose a guess that
  could not win.
- Each of the 243 opening patterns has its own lazy cache entry. Cache
  initialization does not hold locks while running Rayon work, avoiding nested
  pool deadlocks. Concurrent misses may compute the same entry independently.
- With the `solver` feature, single-game searches score guesses in parallel;
  without it (default GUI and WASM hints) they run serially. The batch
  benchmark distributes games across Rayon workers and scores each worker's
  game serially, avoiding nested scheduling. Statistics use local accumulation and
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

The native-only `solver` feature enables Rayon-parallel search and the
benchmark binary. The solver module itself is always built for the GUI hint.
The benchmark's `--check` mode exits unsuccessfully on any unsolved game, any
game exceeding six guesses, or an average above 4.2. Unsolved games contribute
their actual attempted guesses (capped at 21) to the reported average.

Tests cover malformed inputs, an independent duplicate-grading reference over
59,049 word pairs, external feedback accumulation, preserved candidate-pool
behavior, lazy caching, deterministic search, and updated golden solve traces.
