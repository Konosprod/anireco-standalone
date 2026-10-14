//! Loads the exported anime dataset (SQLite file produced by the original
//! Python project's export_sqlite.py) fully into memory. At our scale
//! (~20k anime) this is simpler and fast enough than querying SQLite live --
//! SQLite here is just a portable on-disk format.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Tag {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub rank: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RelationNode {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Relation {
    #[serde(rename = "relationType")]
    relation_type: String,
    node: RelationNode,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RecMedia {
    id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RecNode {
    #[serde(default)]
    rating: f64,
    #[serde(rename = "mediaRecommendation")]
    media_recommendation: Option<RecMedia>,
}

#[derive(Debug, Clone, Deserialize)]
struct RecommendationEntry {
    node: RecNode,
}

#[derive(Debug, Default)]
pub struct Anime {
    pub id: i64,
    pub title_english: String,
    pub title_native: String,
    pub title_romaji: String,
    pub description: String,
    pub genres: Vec<String>,
    pub site_url: String,
    pub cover: String,

    // Precomputed once at load time for scoring.
    pub genre_set: HashSet<String>,
    pub tag_weights: HashMap<String, f64>,
    pub franchise_neighbors: HashSet<i64>,
    pub recommended_ids: Vec<i64>,
    pub vector: Vec<f32>,
    pub norm: f64,
}

impl Anime {
    pub fn display_title(&self) -> &str {
        if !self.title_romaji.is_empty() {
            &self.title_romaji
        } else if !self.title_english.is_empty() {
            &self.title_english
        } else {
            &self.title_native
        }
    }
}

pub struct Store {
    pub by_id: HashMap<i64, usize>,
    pub all: Vec<Anime>,
}

/// Same AniList relationTypes the original crud.py treats as "same franchise".
const FRANCHISE_RELATION_TYPES: &[&str] = &[
    "SEQUEL",
    "PREQUEL",
    "SIDE_STORY",
    "SPIN_OFF",
    "ALTERNATIVE",
    "SUMMARY",
];

fn vector_norm(v: &[f32]) -> f64 {
    v.iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt()
}

fn decode_vector(blob: &[u8]) -> Vec<f32> {
    let (chunks, _remainder) = blob.as_chunks::<4>();
    chunks.iter().map(|c| f32::from_le_bytes(*c)).collect()
}

pub enum LoadError {
    /// None of the candidate paths exist.
    Missing,
    /// The file is a Git LFS pointer (repo cloned without git-lfs), not the database.
    LfsPointer(PathBuf),
    /// The file exists but couldn't be read as our database.
    Invalid(PathBuf, rusqlite::Error),
}

/// Loads the first candidate path that exists.
pub fn load_first(candidates: &[PathBuf]) -> Result<(Store, PathBuf), LoadError> {
    let path = candidates
        .iter()
        .find(|p| p.is_file())
        .ok_or(LoadError::Missing)?;

    if is_lfs_pointer(path) {
        return Err(LoadError::LfsPointer(path.clone()));
    }

    load(path)
        .map(|store| (store, path.clone()))
        .map_err(|err| LoadError::Invalid(path.clone(), err))
}

fn is_lfs_pointer(path: &Path) -> bool {
    const PREFIX: &[u8] = b"version https://git-lfs";
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    metadata.len() < 1024 && std::fs::read(path).is_ok_and(|bytes| bytes.starts_with(PREFIX))
}

fn load(path: &Path) -> rusqlite::Result<Store> {
    // Read-only: a plain `open` would silently create an empty file.
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT id, title_english, title_native, title_romaji, description, genres, tags, \
                site_url, cover, relations, recommendations, vector FROM anime",
    )?;

    let mut rows = stmt.query([])?;
    let mut all = Vec::new();

    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let title_english: Option<String> = row.get(1)?;
        let title_native: Option<String> = row.get(2)?;
        let title_romaji: Option<String> = row.get(3)?;
        let description: Option<String> = row.get(4)?;
        let genres_json: String = row.get(5)?;
        let tags_json: String = row.get(6)?;
        let site_url: Option<String> = row.get(7)?;
        let cover: Option<String> = row.get(8)?;
        let relations_json: String = row.get(9)?;
        let recommendations_json: String = row.get(10)?;
        let vector_blob: Vec<u8> = row.get(11)?;

        let genres: Vec<String> = serde_json::from_str(&genres_json).unwrap_or_default();
        let tags: Vec<Tag> = serde_json::from_str(&tags_json).unwrap_or_default();
        let relations: Vec<Relation> = serde_json::from_str(&relations_json).unwrap_or_default();
        let recs: Vec<RecommendationEntry> =
            serde_json::from_str(&recommendations_json).unwrap_or_default();

        let genre_set: HashSet<String> = genres.iter().cloned().collect();
        let tag_weights: HashMap<String, f64> = tags
            .iter()
            .map(|t| (t.name.clone(), t.rank / 100.0))
            .collect();

        let franchise_neighbors: HashSet<i64> = relations
            .iter()
            .filter(|r| {
                FRANCHISE_RELATION_TYPES.contains(&r.relation_type.as_str())
                    && r.node.kind == "ANIME"
            })
            .map(|r| r.node.id)
            .collect();

        let mut recs_sorted = recs;
        recs_sorted.sort_by(|a, b| {
            b.node
                .rating
                .partial_cmp(&a.node.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let recommended_ids: Vec<i64> = recs_sorted
            .iter()
            .filter_map(|r| r.node.media_recommendation.as_ref().map(|m| m.id))
            .collect();

        let vector = decode_vector(&vector_blob);
        let norm = vector_norm(&vector);

        all.push(Anime {
            id,
            title_english: title_english.unwrap_or_default(),
            title_native: title_native.unwrap_or_default(),
            title_romaji: title_romaji.unwrap_or_default(),
            description: description.unwrap_or_default(),
            genres,
            site_url: site_url.unwrap_or_default(),
            cover: cover.unwrap_or_default(),
            genre_set,
            tag_weights,
            franchise_neighbors,
            recommended_ids,
            vector,
            norm,
        });
    }

    let by_id: HashMap<i64, usize> = all.iter().enumerate().map(|(i, a)| (a.id, i)).collect();

    Ok(Store { by_id, all })
}
