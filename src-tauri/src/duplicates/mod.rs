//! F29: duplicates, for information only (nothing is deleted). Identical files share
//! a SHA-256 (F05); similar photos have close perceptual hashes: a 64-bit difference
//! hash (dHash) of the small thumbnail (F12), kept in `media_items.perceptual_hash` and
//! cleared by the scan when the file's content changes.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use image::imageops::FilterType;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::thumbs::{thumb_path, ThumbSize};

/// Up to this many different bits out of 64 is "parecida": the same shot resized,
/// recompressed or lightly edited, not a different photo of the same scene.
pub const SIMILAR_BITS: u32 = 6;

/// dHash: 9×8 grey, one bit per pixel brighter than its right-hand neighbour.
pub fn dhash(image: &image::DynamicImage) -> u64 {
    let small = image.resize_exact(9, 8, FilterType::Triangle).to_luma8();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            hash <<= 1;
            if small.get_pixel(x, y)[0] > small.get_pixel(x + 1, y)[0] {
                hash |= 1;
            }
        }
    }
    hash
}

/// Hashes the photos that have a small thumbnail and no perceptual hash yet. Returns
/// how many got one; `stop()` pauses between files (F17).
pub fn fill_perceptual_hashes(connection: &Connection, root: &Path, stop: &dyn Fn() -> bool) -> rusqlite::Result<usize> {
    let ids: Vec<i64> = connection
        .prepare(
            "SELECT m.id FROM media_items m JOIN thumbnails t ON t.media_id = m.id AND t.size = 'small'
             WHERE m.media_type = 'photo' AND m.perceptual_hash IS NULL AND m.indexing_status != 'missing_file'",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut update = connection.prepare("UPDATE media_items SET perceptual_hash = ?2 WHERE id = ?1")?;
    let mut hashed = 0;
    for id in ids {
        if stop() {
            break;
        }
        // An unreadable thumbnail just stays without a hash; the next run tries again.
        if let Ok(image) = image::open(thumb_path(root, id, ThumbSize::Small)) {
            update.execute(params![id, format!("{:016x}", dhash(&image))])?;
            hashed += 1;
        }
    }
    Ok(hashed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Identical,
    Similar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Copy {
    pub id: i64,
    /// Relative to the library root, `/`-separated.
    pub path: String,
    pub size_bytes: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub taken_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub kind: Kind,
    pub copies: Vec<Copy>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Duplicates {
    pub groups: Vec<Group>,
    /// Photos with a perceptual hash: "parecidas" only covers these.
    pub hashed: u64,
    pub photos: u64,
}

struct Row {
    copy: Copy,
    sha: Option<String>,
    phash: Option<u64>,
}

/// Pairs `(a, b)`, `a < b`, of hashes at most `SIMILAR_BITS` apart (F32: no all-pairs scan).
/// Pigeonhole: split into four 16-bit blocks; two hashes within 6 bits have a block
/// within 1 bit, so each hash only meets those filed under its blocks or their
/// one-bit neighbours. A pair can come out more than once.
pub(crate) fn similar_pairs(hashes: &[u64]) -> Vec<(usize, usize)> {
    const BLOCKS: u32 = 4;
    let block = |hash: u64, at: u32| ((hash >> (at * 16)) & 0xFFFF) as usize;
    // One flat table per block, sorted by key (counting sort), holding the hash next to its
    // index so a probe reads one contiguous run.
    let tables: Vec<(Vec<u32>, Vec<(u64, u32)>)> = (0..BLOCKS)
        .map(|at| {
            let mut starts = vec![0u32; (1 << 16) + 1];
            for &hash in hashes {
                starts[block(hash, at) + 1] += 1;
            }
            for key in 0..1 << 16 {
                starts[key + 1] += starts[key];
            }
            let mut next = starts.clone();
            let mut entries = vec![(0u64, 0u32); hashes.len()];
            for (index, &hash) in hashes.iter().enumerate() {
                let slot = &mut next[block(hash, at)];
                entries[*slot as usize] = (hash, index as u32);
                *slot += 1;
            }
            (starts, entries)
        })
        .collect();
    let mut pairs = Vec::new();
    for (a, &hash) in hashes.iter().enumerate() {
        for (at, (starts, entries)) in tables.iter().enumerate() {
            let key = block(hash, at as u32);
            for probe in std::iter::once(key).chain((0..16).map(|bit| key ^ (1 << bit))) {
                for &(other, b) in &entries[starts[probe] as usize..starts[probe + 1] as usize] {
                    if b as usize > a && (hash ^ other).count_ones() <= SIMILAR_BITS {
                        pairs.push((a, b as usize));
                    }
                }
            }
        }
    }
    pairs
}

fn find(parent: &mut [usize], mut at: usize) -> usize {
    while parent[at] != at {
        parent[at] = parent[parent[at]];
        at = parent[at];
    }
    at
}

/// Identical groups (same SHA-256), most bytes repeated first; then similar groups of
/// photos whose hashes are within `SIMILAR_BITS` and are not all one identical file.
pub fn duplicates(connection: &Connection) -> rusqlite::Result<Duplicates> {
    let rows: Vec<Row> = connection
        .prepare(
            "SELECT id, relative_path, size_bytes, width, height, taken_at, file_hash_sha256, perceptual_hash, media_type
             FROM media_items WHERE is_hidden = 0 AND indexing_status != 'missing_file'
             ORDER BY relative_path",
        )?
        .query_map([], |row| {
            let phash: Option<String> = row.get(7)?;
            let photo = row.get::<_, String>(8)? == "photo";
            Ok(Row {
                copy: Copy {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    size_bytes: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    taken_at: row.get(5)?,
                },
                sha: row.get(6)?,
                phash: phash.filter(|_| photo).and_then(|hex| u64::from_str_radix(&hex, 16).ok()),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let photos: i64 = connection.query_row(
        "SELECT count(*) FROM media_items WHERE is_hidden = 0 AND indexing_status != 'missing_file' AND media_type = 'photo'",
        [],
        |row| row.get(0),
    )?;

    let mut by_sha: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(sha) = row.sha.as_deref() {
            by_sha.entry(sha).or_default().push(index);
        }
    }
    let mut identical: Vec<Vec<usize>> = by_sha.into_values().filter(|members| members.len() > 1).collect();
    identical.sort_by_key(|members| {
        let size = rows[members[0]].copy.size_bytes.unwrap_or(0);
        (std::cmp::Reverse(size * (members.len() as i64 - 1)), rows[members[0]].copy.path.to_lowercase())
    });

    // Similar: union of every pair within SIMILAR_BITS.
    let hashed: Vec<usize> = (0..rows.len()).filter(|&index| rows[index].phash.is_some()).collect();
    let mut parent: Vec<usize> = (0..rows.len()).collect();
    for (a, b) in similar_pairs(&hashed.iter().map(|&index| rows[index].phash.unwrap_or(0)).collect::<Vec<_>>()) {
        let (ra, rb) = (find(&mut parent, hashed[a]), find(&mut parent, hashed[b]));
        if ra != rb {
            parent[rb] = ra;
        }
    }
    let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for &index in &hashed {
        let root = find(&mut parent, index);
        clusters.entry(root).or_default().push(index);
    }
    let mut similar: Vec<Vec<usize>> = clusters
        .into_values()
        .filter(|members| {
            // All one file: already an identical group.
            let first = rows[members[0]].sha.as_deref();
            members.len() > 1 && (first.is_none() || members.iter().any(|&index| rows[index].sha.as_deref() != first))
        })
        .collect();
    similar.sort_by_key(|members| (std::cmp::Reverse(members.len()), rows[members[0]].copy.path.to_lowercase()));

    let group = |kind: Kind, members: Vec<usize>| Group {
        kind,
        copies: members.into_iter().map(|index| rows[index].copy.clone()).collect(),
    };
    let mut groups: Vec<Group> = identical.into_iter().map(|members| group(Kind::Identical, members)).collect();
    groups.extend(similar.into_iter().map(|members| group(Kind::Similar, members)));
    Ok(Duplicates {
        groups,
        hashed: hashed.len() as u64,
        photos: photos.max(0) as u64,
    })
}
