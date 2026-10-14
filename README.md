# anireco-standalone

Portable, single-binary rewrite of [anime_reco](https://github.com/konosprod/miniature-octo-lamp): recommend anime based on a chosen anime. No runtime, no database server, no browser window — a real native desktop app, one executable + one data file, works on Linux/macOS/Windows.

Same idea, different implementation: the original project is a FastAPI + PostgreSQL/pgvector web service meant to run as a deployed service. This is a Rust + [egui](https://github.com/emilk/egui) rewrite of the same recommendation logic (embedding similarity + genre/tag overlap + transitive franchise exclusion), storing data in an embedded SQLite file and rendering its own native window instead, for something a single person can just run locally.

## Run it

```sh
cargo run --release -- data/anime.db
```

Opens a native window. First argument: path to the data file (default `data/anime.db`).

## Get the data file

`data/anime.db` isn't committed (it's ~400MB). It's exported from the original project's Postgres database via its `export_sqlite.py` script:

```sh
# from the original anime_reco repo
just export-sqlite /path/to/anireco-standalone/data/anime.db
```

This is a format conversion only — no Scaleway re-embedding, the vectors are copied as-is.

## Build

```sh
cargo build --release
./target/release/anireco-standalone data/anime.db
```

## Cross-platform builds

Unlike the Go/webview prototype this replaced, native GUI windows need OS-provided frameworks (Cocoa on macOS, Win32 on Windows), which realistically can't be cross-compiled from Linux. `.github/workflows/build.yml` instead builds natively on `ubuntu-latest`, `windows-latest` and `macos-latest` GitHub-hosted runners on every push to `main`/PR, and attaches binaries to a GitHub Release when a `v*.*.*` tag is pushed.

## Layout

- `src/main.rs` — loads the data file, starts the native window (eframe).
- `src/store.rs` — loads the SQLite export fully into memory (metadata + vectors); SQLite here is just a portable file format, not queried live.
- `src/recommend.rs` — the scoring logic: cosine similarity, genre Jaccard, rank-weighted tag overlap (Ruzicka similarity), transitive franchise-exclusion BFS.
- `src/app.rs` — the egui UI: search/autocomplete, source card, two side-by-side result columns (ours vs. AniList's own recommendations) with common titles highlighted. Clicking a card opens its AniList page in the system browser (an external link, not the app's own UI shell).

## Notes for future-me

- `egui_extras`'s `"image"` feature pulls in the `image` crate with **zero format decoders enabled** (its `Cargo.toml` sets `default-features = false` and none of egui_extras's own feature flags turn on `image/png` or `image/jpeg`). Cover images loading inconsistently was this — fixed by depending on `image` directly with `default-features = false, features = ["jpeg", "png", "webp"]`. Don't remove that dependency thinking it's unused; cargo won't tell you it's load-bearing.
