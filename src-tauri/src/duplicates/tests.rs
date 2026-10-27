use std::fs;

use image::{DynamicImage, Rgb, RgbImage};
use rusqlite::{params, Connection};

use super::{dhash, duplicates, fill_perceptual_hashes, similar_pairs, Kind, SIMILAR_BITS};
use crate::db::MIGRATIONS;
use crate::thumbs::{thumb_path, ThumbSize};

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
        .unwrap();
    connection
}

fn media(connection: &Connection, path: &str, kind: &str, size: i64, sha: Option<&str>, phash: Option<&str>) -> i64 {
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, size_bytes,
               file_hash_sha256, perceptual_hash, created_at, updated_at)
             VALUES (1, ?1, ?1, 'jpg', ?2, ?3, ?4, ?5, 't', 't')",
            params![path, kind, size, sha, phash],
        )
        .unwrap();
    connection.last_insert_rowid()
}

/// A diagonal gradient; `shift` moves it so the hash changes a lot.
fn picture(width: u32, height: u32, shift: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(width, height, |x, y| {
        let value = (((x * 255 / width) + (y * 97 / height) + shift) % 256) as u8;
        Rgb([value, value.wrapping_mul(3), 255 - value])
    }))
}

#[test]
fn dup01_dhash_survives_resizing_but_not_another_picture() {
    let big = dhash(&picture(400, 300, 0));
    let small = dhash(&picture(160, 120, 0));
    let other = dhash(&picture(400, 300, 128));
    assert!((big ^ small).count_ones() <= 4, "{big:016x} vs {small:016x}");
    assert!((big ^ other).count_ones() > 10, "{big:016x} vs {other:016x}");
}

#[test]
fn dup02_groups_identical_files_and_similar_photos() {
    let connection = database();
    let a = media(&connection, "Google Fotos/Fotos de 2022/foto001.jpg", "photo", 3_100_000, Some("aa"), Some("ff00ff00ff00ff00"));
    let b = media(&connection, "Google Fotos/Viagem/foto001.jpg", "photo", 3_100_000, Some("aa"), Some("ff00ff00ff00ff00"));
    // A smaller copy of the same shot: 3 bits apart.
    let c = media(&connection, "Organizadas/foto001(1).jpg", "photo", 600_000, Some("cc"), Some("ff00ff00ff00ff07"));
    // Big identical videos come first.
    let v1 = media(&connection, "a/VID.mp4", "video", 80_000_000, Some("vv"), None);
    let v2 = media(&connection, "b/VID.mp4", "video", 80_000_000, Some("vv"), None);
    // Far apart: 16 bits.
    media(&connection, "outra.jpg", "photo", 1_000, Some("dd"), Some("00ff00ff00ff00ff"));
    // Identical but hidden: not a duplicate of anything shown.
    let hidden = media(&connection, "oculta.jpg", "photo", 1_000, Some("dd"), None);
    connection.execute("UPDATE media_items SET is_hidden = 1 WHERE id = ?1", [hidden]).unwrap();

    let found = duplicates(&connection).unwrap();
    let groups: Vec<(Kind, Vec<i64>)> = found
        .groups
        .iter()
        .map(|group| (group.kind, group.copies.iter().map(|copy| copy.id).collect()))
        .collect();
    assert_eq!(
        groups,
        vec![(Kind::Identical, vec![v1, v2]), (Kind::Identical, vec![a, b]), (Kind::Similar, vec![a, b, c])]
    );
    assert_eq!((found.hashed, found.photos), (4, 4));
}

#[test]
fn dup03_identical_photos_alone_are_not_also_similar() {
    let connection = database();
    media(&connection, "x/a.jpg", "photo", 10, Some("aa"), Some("0000000000000000"));
    media(&connection, "y/a.jpg", "photo", 10, Some("aa"), Some("0000000000000000"));
    let kinds: Vec<Kind> = duplicates(&connection).unwrap().groups.iter().map(|group| group.kind).collect();
    assert_eq!(kinds, vec![Kind::Identical]);
}

#[test]
fn dup04_hashes_come_from_the_small_thumbnails() {
    let connection = database();
    let root = std::env::temp_dir().join(format!("myphotos-dup04-{}", std::process::id()));
    let with_thumb = media(&connection, "a.jpg", "photo", 10, Some("aa"), None);
    let without = media(&connection, "b.jpg", "photo", 10, Some("bb"), None);
    let video = media(&connection, "c.mp4", "video", 10, Some("cc"), None);
    for id in [with_thumb, video] {
        let path = thumb_path(&root, id, ThumbSize::Small);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        picture(300, 225, 0).to_rgb8().save(&path).unwrap();
        connection
            .execute(
                "INSERT INTO thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
                 VALUES (?1, 'small', 300, 225, 1, 'f', 't', 't')",
                [id],
            )
            .unwrap();
    }
    assert_eq!(fill_perceptual_hashes(&connection, &root, &|| false).unwrap(), 1);
    let hash = |id: i64| -> Option<String> {
        connection.query_row("SELECT perceptual_hash FROM media_items WHERE id = ?1", [id], |row| row.get(0)).unwrap()
    };
    assert_eq!(hash(with_thumb), Some(format!("{:016x}", dhash(&image::open(thumb_path(&root, with_thumb, ThumbSize::Small)).unwrap()))));
    assert_eq!((hash(without), hash(video)), (None, None));
    assert_eq!(fill_perceptual_hashes(&connection, &root, &|| false).unwrap(), 0, "done once");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn dup05_block_index_finds_exactly_the_pairs_a_full_scan_finds() {
    // Pseudo-random hashes plus near copies 1..=8 bits away, spread across the blocks.
    let mut hashes = Vec::new();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for index in 0..400u64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        hashes.push(seed);
        let flips = 1 + index % 8;
        let mut near = seed;
        for bit in 0..flips {
            near ^= 1 << ((bit * 9 + index) % 64);
        }
        hashes.push(near);
    }
    let mut expected = Vec::new();
    for a in 0..hashes.len() {
        for b in a + 1..hashes.len() {
            if (hashes[a] ^ hashes[b]).count_ones() <= SIMILAR_BITS {
                expected.push((a, b));
            }
        }
    }
    let mut found = similar_pairs(&hashes);
    found.sort();
    found.dedup();
    assert_eq!(found, expected);
    assert!(expected.len() >= 250, "{}", expected.len());
}
