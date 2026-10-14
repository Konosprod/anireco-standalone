# anireco-standalone

![Anime Reco](docs/screenshot.png)

Pick an anime, get recommendations, side by side with AniList's own community picks so you can see where they agree.

A native desktop app: one executable plus one data file, no server, no browser. Builds for Linux, Windows and macOS.

## How it works

- **Data**: ~20k AniList anime, each with an embedding of its title and synopsis, computed once by [anime_reco](https://github.com/Konosprod/miniature-octo-lamp) and exported to `data/anime.db` (SQLite).
- **Recommendations**: the closest anime by cosine similarity, minus the same franchise (sequels, spin-offs…).
- **App**: Rust + [egui](https://github.com/emilk/egui), with all data loaded in memory at startup.

## Run

Download the executable for your OS from the [latest release](https://github.com/Konosprod/anireco-standalone/releases/latest). On first launch, the app downloads its database (`anime.db`, ~400 MB, attached to the release) next to the executable.

From a clone:

```sh
cargo run --release
```

`data/anime.db` is stored with [Git LFS](https://git-lfs.com): install it before cloning, or run `git lfs pull` afterwards, otherwise you only get a small pointer file.
