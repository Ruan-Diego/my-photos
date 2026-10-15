//! SRCH-01..08 on a temp database built from the real migrations. Never `E:\`.

use std::time::Instant;

use rusqlite::{params, Connection};

use super::*;
use crate::db::tests::Scratch;
use crate::db::MIGRATIONS;

const NOW: &str = "2026-10-15T12:00:00.000Z";

struct Db {
    _scratch: Scratch,
    connection: Connection,
}

fn database(label: &str, versions: usize) -> Db {
    let scratch = Scratch::new(&format!("f14-{label}"));
    let connection = Connection::open(scratch.path.join("library.db")).unwrap();
    connection.pragma_update(None, "foreign_keys", "ON").unwrap();
    for migration in &MIGRATIONS[..versions] {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute(
            "INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', ?1)",
            [NOW],
        )
        .unwrap();
    Db { _scratch: scratch, connection }
}

impl Db {
    fn new(label: &str) -> Self {
        database(label, MIGRATIONS.len())
    }

    fn media(&self, path: &str, media_type: &str, taken_at: Option<&str>) -> i64 {
        let filename = path.rsplit('/').next().unwrap();
        self.connection
            .execute(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
                   taken_at, created_at, updated_at)
                 VALUES (1, ?1, ?2, 'jpg', ?3, ?4, ?5, ?5)",
                params![path, filename, media_type, taken_at, NOW],
            )
            .unwrap();
        self.connection.last_insert_rowid()
    }

    fn sql(&self, sql: &str) {
        self.connection.execute_batch(sql).unwrap();
    }

    fn person(&self, media: i64, name: &str) -> i64 {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO people (library_id, name, normalized_name) VALUES (1, ?1, lower(?1))",
                [name],
            )
            .unwrap();
        let person: i64 = self
            .connection
            .query_row("SELECT id FROM people WHERE normalized_name = lower(?1)", [name], |row| row.get(0))
            .unwrap();
        self.connection
            .execute("INSERT INTO media_people (media_id, person_id) VALUES (?1, ?2)", [media, person])
            .unwrap();
        person
    }

    fn find(&self, query: &SearchQuery) -> Vec<i64> {
        search(&self.connection, query).unwrap().hits.into_iter().map(|hit| hit.id).collect()
    }

    fn text(&self, text: &str) -> Vec<i64> {
        self.find(&SearchQuery { text: text.into(), ..SearchQuery::default() })
    }
}

#[test]
fn srch01_the_box_splits_words_dates_and_types() {
    assert_eq!(
        parse_text("praia em 2022"),
        ParsedText { fts: Some("\"praia\"*".into()), year: Some(2022), ..ParsedText::default() }
    );
    assert_eq!(
        parse_text("Fotos de AGOSTO"),
        ParsedText { month: Some(8), media_type: Some(MediaKind::Photo), ..ParsedText::default() }
    );
    assert_eq!(
        parse_text("Março 2021 vídeos ana"),
        ParsedText {
            fts: Some("\"ana\"*".into()),
            year: Some(2021),
            month: Some(3),
            media_type: Some(MediaKind::Video),
        }
    );
    // FTS5 syntax is just text; quotes are doubled.
    assert_eq!(
        parse_text("a OR -b say\"hi\" path:x"),
        ParsedText { fts: Some("\"OR\"* \"-b\"* \"say\"\"hi\"\"\"* \"path:x\"*".into()), ..ParsedText::default() }
    );
    // "mar" is the sea, not March; a second year is a word.
    assert_eq!(parse_text("mar").fts.as_deref(), Some("\"mar\"*"));
    assert_eq!(parse_text("2019 2020").fts.as_deref(), Some("\"2020\"*"));
    assert_eq!(parse_text("   de  "), ParsedText::default());
    assert_eq!(parse_text("12345").fts.as_deref(), Some("\"12345\"*"));
}

#[test]
fn srch02_words_match_names_folders_and_text_without_accents() {
    let db = Db::new("text");
    let beach = db.media("Takeout/Google Fotos/Viagem Bahia/IMG_1234.jpg", "photo", Some("2022-01-10T10:00:00.000Z"));
    let city = db.media("Fotos/2019/DSC_0001.JPG", "photo", Some("2019-05-01T10:00:00.000Z"));
    db.connection
        .execute(
            "UPDATE media_items SET title = 'Pôr do sol', description = 'São Paulo à noite' WHERE id = ?1",
            [city],
        )
        .unwrap();
    assert_eq!(db.text("1234"), [beach]);
    assert_eq!(db.text("img"), [beach]);
    assert_eq!(db.text("bahia"), [beach]);
    assert_eq!(db.text("viag"), [beach]);
    assert_eq!(db.text("sao paulo"), [city]);
    assert_eq!(db.text("SÃO"), [city]);
    assert_eq!(db.text("por sol"), [city]);
    assert_eq!(db.text("paulo bahia"), Vec::<i64>::new());
    assert_eq!(db.text(""), [beach, city]);
    assert_eq!(db.text("\"unbalanced"), Vec::<i64>::new());
}

#[test]
fn srch03_people_follow_their_links_and_renames() {
    let db = Db::new("people");
    let photo = db.media("a.jpg", "photo", None);
    let other = db.media("b.jpg", "photo", None);
    let ana = db.person(photo, "Ana Clara");
    db.person(photo, "João");
    db.person(other, "Ana Clara");
    assert_eq!(db.text("ana"), [other, photo]);
    assert_eq!(db.text("joao"), [photo]);

    db.connection
        .execute("DELETE FROM media_people WHERE media_id = ?1 AND person_id = ?2", [other, ana])
        .unwrap();
    assert_eq!(db.text("ana"), [photo]);

    db.connection.execute("UPDATE people SET name = 'Aninha' WHERE id = ?1", [ana]).unwrap();
    assert_eq!(db.text("aninha"), [photo]);
    assert_eq!(db.text("clara"), Vec::<i64>::new());
    assert_eq!(db.text("joao"), [photo]);
}

#[test]
fn srch04_place_and_note_are_searchable() {
    let db = Db::new("place-note");
    let photo = db.media("a.jpg", "photo", None);
    db.connection
        .execute(
            "INSERT INTO media_locations (media_id, latitude, longitude, source, place_label)
             VALUES (?1, -12.9, -38.5, 'exif', 'Salvador, Bahia')",
            [photo],
        )
        .unwrap();
    assert_eq!(db.text("salvador"), [photo]);
    db.sql("UPDATE media_locations SET place_label = 'Ilhéus'");
    assert_eq!(db.text("ilheus"), [photo]);
    assert_eq!(db.text("salvador"), Vec::<i64>::new());
    db.sql("DELETE FROM media_locations");
    assert_eq!(db.text("ilheus"), Vec::<i64>::new());

    db.connection
        .execute("INSERT INTO user_notes (media_id, note, updated_at) VALUES (?1, 'aniversário da vó', ?2)", params![photo, NOW])
        .unwrap();
    assert_eq!(db.text("aniversario"), [photo]);
    db.sql("UPDATE user_notes SET note = 'formatura'");
    assert_eq!(db.text("formatura"), [photo]);
    db.sql("DELETE FROM user_notes");
    assert_eq!(db.text("formatura"), Vec::<i64>::new());
}

#[test]
fn srch05_dates_types_people_location_and_favorites_filter() {
    let db = Db::new("filters");
    let dec = db.media("dec.jpg", "photo", Some("2021-12-31T23:00:00.000Z"));
    let jan = db.media("jan.jpg", "photo", Some("2022-01-01T01:00:00.000Z"));
    let aug = db.media("aug.mp4", "video", Some("2022-08-10T14:00:00.000Z"));
    let aug_old = db.media("aug-old.jpg", "photo", Some("2019-08-01T00:00:00.000Z"));
    let undated = db.media("undated.jpg", "photo", None);

    assert_eq!(db.text("2022"), [aug, jan]);
    assert_eq!(db.text("dezembro 2021"), [dec]);
    assert_eq!(db.text("agosto"), [aug, aug_old]);
    assert_eq!(db.text("videos"), [aug]);
    assert_eq!(db.text("fotos 2022"), [jan]);
    let query = |query: SearchQuery| db.find(&query);
    assert_eq!(query(SearchQuery { media_type: Some(MediaKind::Photo), ..SearchQuery::default() }), [jan, dec, aug_old, undated]);
    assert_eq!(
        query(SearchQuery { from: Some("2021-12-31".into()), until: Some("2022-08-10".into()), ..SearchQuery::default() }),
        [jan, dec]
    );

    let ana = db.person(aug_old, "Ana");
    assert_eq!(query(SearchQuery { person_id: Some(ana), ..SearchQuery::default() }), [aug_old]);
    db.connection
        .execute("INSERT INTO media_locations (media_id, latitude, longitude, source) VALUES (?1, 1.0, 2.0, 'exif')", [jan])
        .unwrap();
    assert_eq!(query(SearchQuery { has_location: Some(true), ..SearchQuery::default() }), [jan]);
    assert_eq!(query(SearchQuery { has_location: Some(false), ..SearchQuery::default() }), [aug, dec, aug_old, undated]);
    db.connection.execute("UPDATE media_items SET is_favorite = 1 WHERE id = ?1", [dec]).unwrap();
    assert_eq!(query(SearchQuery { favorites_only: true, ..SearchQuery::default() }), [dec]);

    // Hidden and missing files never come back.
    db.connection.execute("UPDATE media_items SET is_hidden = 1 WHERE id = ?1", [jan]).unwrap();
    db.connection.execute("UPDATE media_items SET indexing_status = 'missing_file' WHERE id = ?1", [aug]).unwrap();
    assert_eq!(db.text("2022"), Vec::<i64>::new());
}

#[test]
fn srch06_newest_first_with_paging_and_a_total() {
    let db = Db::new("paging");
    let ids: Vec<i64> = (0..5)
        .map(|n| db.media(&format!("praia/{n}.jpg"), "photo", Some(&format!("202{n}-01-01T00:00:00.000Z"))))
        .collect();
    let undated = db.media("praia/x.jpg", "photo", None);
    let page = search(
        &db.connection,
        &SearchQuery { text: "praia".into(), limit: Some(2), offset: 1, ..SearchQuery::default() },
    )
    .unwrap();
    assert_eq!(page.total, 6);
    assert_eq!(page.hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [ids[3], ids[2]]);
    assert_eq!(page.hits[0].taken_at.as_deref(), Some("2023-01-01T00:00:00.000Z"));
    assert_eq!(page.hits[0].media_type, "photo");
    let last = search(&db.connection, &SearchQuery { offset: 5, ..SearchQuery::default() }).unwrap();
    assert_eq!(last.hits.iter().map(|hit| hit.id).collect::<Vec<_>>(), [undated]);
    let zero = search(&db.connection, &SearchQuery { limit: Some(0), ..SearchQuery::default() }).unwrap();
    assert_eq!(zero.hits.len(), 1);
}

#[test]
fn srch07_moves_deletes_and_rebuild_keep_the_index_true() {
    let db = Db::new("sync");
    let photo = db.media("Antiga/a.jpg", "photo", None);
    db.connection
        .execute("UPDATE media_items SET relative_path = 'Nova/a.jpg' WHERE id = ?1", [photo])
        .unwrap();
    assert_eq!(db.text("antiga"), Vec::<i64>::new());
    assert_eq!(db.text("nova"), [photo]);
    // Unrelated columns do not touch the index.
    db.connection.execute("UPDATE media_items SET size_bytes = 5 WHERE id = ?1", [photo]).unwrap();
    assert_eq!(db.text("nova"), [photo]);

    db.sql("DELETE FROM media_search");
    assert_eq!(db.text("nova"), Vec::<i64>::new());
    rebuild(&db.connection).unwrap();
    assert_eq!(db.text("nova"), [photo]);

    db.connection.execute("DELETE FROM media_items WHERE id = ?1", [photo]).unwrap();
    let rows: i64 = db.connection.query_row("SELECT count(*) FROM media_search", [], |row| row.get(0)).unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn srch08_upgrading_an_indexed_library_fills_the_index() {
    let db = database("upgrade", 2);
    let photo = db.media("Viagem/a.jpg", "photo", None);
    db.connection
        .execute(
            "INSERT INTO people (library_id, name, normalized_name) VALUES (1, 'Bia', 'bia');
             ",
            [],
        )
        .unwrap();
    db.connection.execute("INSERT INTO media_people (media_id, person_id) VALUES (?1, 1)", [photo]).unwrap();
    db.connection.execute_batch(MIGRATIONS[2].sql).unwrap();
    assert_eq!(db.text("viagem"), [photo]);
    assert_eq!(db.text("bia"), [photo]);
}

#[test]
fn srch09_twenty_thousand_media_answer_quickly() {
    let db = Db::new("speed");
    db.sql("BEGIN");
    for n in 0..20_000 {
        let folder = ["Viagem", "Natal", "Praia", "Casa"][n % 4];
        db.media(
            &format!("Takeout/{folder} {}/IMG_{n:05}.jpg", n % 50),
            if n % 10 == 0 { "video" } else { "photo" },
            Some(&format!("20{:02}-{:02}-01T00:00:00.000Z", 10 + n % 15, 1 + n % 12)),
        );
    }
    db.sql("COMMIT");
    let started = Instant::now();
    let page = search(&db.connection, &SearchQuery { text: "praia 2021 fotos".into(), ..SearchQuery::default() }).unwrap();
    let elapsed = started.elapsed();
    assert!(page.total > 0 && page.hits.len() as u64 == page.total.min(u64::from(DEFAULT_LIMIT)));
    eprintln!("srch09: {} matches in {elapsed:?}", page.total);
    // RNF-001 asks for well under a second; debug build, generous bound.
    assert!(elapsed.as_millis() < 2000, "{elapsed:?}");
}
