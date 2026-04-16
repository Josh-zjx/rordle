# Faster Wordle Solver

Performance and correctness refactor of `src/solver.rs`. The goal was to speed
up a single run of the solver without changing which word it chooses for any
given answer. Behavior is preserved bit-for-bit: on the full 2309-answer
benchmark, the average trial count is unchanged at `4.125595495885665`.

## Results

| Metric              | Before   | After    |
|---------------------|----------|----------|
| `solve_all` wall    | 68.26 s  | 1.30 s   |
| `solve_one` single  | ~hundreds ms | ~9 ms |
| Average trial       | 4.125595495885665 | 4.125595495885665 |
| Failures / unsolved | 0 / 0    | 0 / 0    |

~52× speedup on the benchmark, with no regression in guess quality.

## What changed

### `src/game.rs` — word list layout
`WordList` now carries two cache-friendly views populated once at load:

- `candidate_bytes: Box<[[u8; 5]]>` — fixed-size byte arrays for every word,
  avoiding the UTF-8 indirection on every letter access.
- `candidate_bitvecs: Box<[u32]>` — precomputed 26-bit letter set per word, so
  grading no longer rebuilds the set in the inner loop.

### `src/solver.rs` — five layered optimizations

1. **Cleanup.** Removed the dead `data/cache` read in `Solver::bind` and the
   unused `current_candidate: String` field.
2. **Byte layout.** `grade_pair` and `try_match` rewritten to operate on
   `&[u8; 5]` with a hoisted bitvec (`grade_pair_bytes`, `try_match_bytes`).
3. **Skip-list.** Added `valid_indices: Vec<u32>` alongside the existing
   `valid_table`. `filter_valid_word` rebuilds it in lockstep, and
   `calculate_score` iterates only the surviving candidates instead of walking
   all ~13k with a `valid_word(i)` branch.
4. **Parallel scoring.** The outer `for i in 0..N` search in `new_guess` is
   now a Rayon `par_iter().reduce()` with deterministic tie-break
   (highest score, lowest index). The `parallel_equals_serial` test pins that
   the parallel reduction agrees with a strict-greater serial scan.
5. **Second-guess table.** A 243-entry `OnceLock` table keyed by the possible
   gradings of the opening word `"tares"` stores the best round-1 guess (and
   its score, bitwise). At round 1, `new_guess` is a constant-time lookup
   instead of a full entropy search over ~13k candidates. Populated lazily on
   first access; ~seconds of startup amortized across every subsequent solve.

### Semantic quirk preserved

`solver::grade_pair` and `game::grade_guess` use opposite letter-set
directions — `grade_pair` probes the guess's letter against the answer's
bits, whereas the correct Wordle semantics (`grade_guess`, `try_match`)
probe the other way. The two diverge on pairs like
`grade_pair("abbed", "beads")`. This quirk is intentional here because
"fixing" it would change the solver's choices for a non-trivial subset of
answers. The `grade_pair_quirk_pinned` test locks the current values so
nobody accidentally "cleans it up" without re-baselining the golden
sequences.

## Tests added

- `golden_guess_sequences` — ten full solve traces pinned to current output.
- `grade_pair_quirk_pinned` — four exact pair values lock the reversed
  semantics.
- `calculate_score_bitwise_stable` — `f64::to_bits()` snapshot of the round-0
  entropy; catches floating-point reordering.
- `parallel_equals_serial` — parallel reduction matches serial scan on a
  non-round-1 state.
- `second_guess_table_matches_fresh_search` — sampled pattern indices match
  what a fresh search produces.

## Out of scope / future work

- Fixing the `grade_pair` / `grade_guess` semantic inconsistency (would change
  solver output; needs re-baselined golden sequences).
- Replacing the 8-manual-thread pool in `roget.rs::solve_all` with a flat
  Rayon iterator (cleaner, and removes the nested-parallelism
  oversubscription seen today; irrelevant to single-solve latency).
- Serializing the full 12,974² pair matrix to disk with a header/hash
  (`data/precompute_pair` is scratch from an earlier experiment — left
  untracked; `.gitignore` updated to keep it out).
