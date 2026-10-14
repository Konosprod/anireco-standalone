//! Ports the scoring logic from the original Python project's api/crud.py:
//! cosine similarity over embeddings, genre Jaccard, rank-weighted tag
//! overlap (Ruzicka similarity), and transitive franchise exclusion. No
//! popularity term -- deliberately removed upstream.

use crate::store::{Anime, Store};
use std::collections::{HashMap, HashSet};

pub const CANDIDATE_POOL_SIZE: usize = 40;
pub const RESULT_LIMIT: usize = 10;
pub const ANILIST_RESULT_LIMIT: usize = 10;

const SIMILARITY_WEIGHT: f64 = 0.6;
const GENRE_WEIGHT: f64 = 0.15;
const TAG_WEIGHT: f64 = 0.25;

fn cosine_similarity(a: &Anime, b: &Anime) -> f64 {
    if a.norm == 0.0 || b.norm == 0.0 {
        return 0.0;
    }
    let dot: f64 = a
        .vector
        .iter()
        .zip(b.vector.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    dot / (a.norm * b.norm)
}

fn genre_jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.len() + b.len() - intersection;
    intersection as f64 / union as f64
}

/// Ruzicka similarity (weighted Jaccard): sum(min) / sum(max) over the union of tags.
fn weighted_tag_overlap(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let mut keys: HashSet<&String> = a.keys().collect();
    keys.extend(b.keys());
    if keys.is_empty() {
        return 0.0;
    }

    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for k in keys {
        let va = *a.get(k).unwrap_or(&0.0);
        let vb = *b.get(k).unwrap_or(&0.0);
        numerator += va.min(vb);
        denominator += va.max(vb);
    }
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

/// Transitive closure of same-franchise media (sequels, spin-offs, ...) to
/// exclude. AniList's relations are direct edges only (S1->S2, S2->S3, ...),
/// not a fully-connected franchise graph, so we BFS through them.
fn franchise_closure(store: &Store, source_id: i64) -> HashSet<i64> {
    let source = &store.all[store.by_id[&source_id]];

    let mut visited: HashSet<i64> = HashSet::from([source_id]);
    let mut frontier: HashSet<i64> = source.franchise_neighbors.clone();

    while !frontier.is_empty() {
        let new_ids: HashSet<i64> = frontier.difference(&visited).cloned().collect();
        if new_ids.is_empty() {
            break;
        }
        visited.extend(new_ids.iter().cloned());

        let mut next_frontier = HashSet::new();
        for id in &new_ids {
            if let Some(&idx) = store.by_id.get(id) {
                next_frontier.extend(store.all[idx].franchise_neighbors.iter().cloned());
            }
        }
        frontier = next_frontier;
    }

    visited.remove(&source_id);
    visited
}

/// Retrieves a candidate pool by cosine similarity, excludes same-franchise
/// media, then re-ranks the pool with a composite score blending embedding
/// similarity, genre overlap, and rank-weighted tag overlap. Returns indices
/// into `store.all`.
pub fn recommend(store: &Store, source_id: i64) -> Vec<usize> {
    let Some(&source_idx) = store.by_id.get(&source_id) else {
        return Vec::new();
    };
    let source = &store.all[source_idx];

    let mut excluded = franchise_closure(store, source_id);
    excluded.insert(source_id);

    let mut pool: Vec<(usize, f64)> = store
        .all
        .iter()
        .enumerate()
        .filter(|(_, candidate)| !excluded.contains(&candidate.id))
        .map(|(idx, candidate)| (idx, cosine_similarity(source, candidate)))
        .collect();

    pool.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pool.truncate(CANDIDATE_POOL_SIZE);

    let mut scored: Vec<(usize, f64)> = pool
        .iter()
        .map(|&(idx, similarity)| {
            let candidate = &store.all[idx];
            let genre_score = genre_jaccard(&source.genre_set, &candidate.genre_set);
            let tag_score = weighted_tag_overlap(&source.tag_weights, &candidate.tag_weights);
            let score = SIMILARITY_WEIGHT * similarity
                + GENRE_WEIGHT * genre_score
                + TAG_WEIGHT * tag_score;
            (idx, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(RESULT_LIMIT);

    scored.into_iter().map(|(idx, _)| idx).collect()
}

/// Resolves AniList's own community-voted recommendations for the given
/// anime against our dataset. Entries AniList recommends that aren't in our
/// dataset are silently skipped. Returns indices into `store.all`.
pub fn anilist_recommendations(store: &Store, source_id: i64) -> Vec<usize> {
    let Some(&source_idx) = store.by_id.get(&source_id) else {
        return Vec::new();
    };
    let source = &store.all[source_idx];

    source
        .recommended_ids
        .iter()
        .filter_map(|id| store.by_id.get(id).copied())
        .take(ANILIST_RESULT_LIMIT)
        .collect()
}
