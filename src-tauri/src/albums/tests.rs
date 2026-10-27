use rusqlite::{params, Connection};

use super::{album_cards, album_folder, rebuild_takeout_albums, AlbumCard};
use crate::db::MIGRATIONS;
use crate::takeout::{AlbumHit, AlbumMetadata, Confidence, DiscoveryReport, MediaHit};

const NOW: &str = "2026-10-27T12:00:00Z";
const EXPORT: &str = "takeout-20260921T183943Z";

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

fn media(connection: &Connection, path: &str, kind: &str, taken_at: Option<&str>, hidden: bool) -> i64 {
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, taken_at,
               is_hidden, created_at, updated_at)
             VALUES (1, ?1, ?1, 'jpg', ?2, ?3, ?4, 't', 't')",
            params![path, kind, taken_at, hidden],
        )
        .unwrap();
    connection.last_insert_rowid()
}

fn hit(path: &str) -> MediaHit {
    MediaHit { relative_path: path.into(), confidence: Confidence::None, json_relative_path: None, metadata: None, reason: None }
}

fn metadata(path: &str, title: Option<&str>, description: Option<&str>) -> AlbumHit {
    AlbumHit {
        relative_path: path.into(),
        metadata: AlbumMetadata {
            title: title.map(Into::into),
            description: description.map(Into::into),
            date_unix: None,
            date: None,
            latitude: None,
            longitude: None,
            altitude: None,
        },
    }
}

fn path(part: u32, rest: &str) -> String {
    format!("{EXPORT}-{part}-001/Takeout/Google Fotos/{rest}")
}

#[test]
fn alb01_folders_under_google_photos_are_albums() {
    assert_eq!(album_folder(&path(1, "Viagem/a.jpg")).as_deref(), Some(&*format!("{EXPORT}/Takeout/Google Fotos/Viagem")));
    assert_eq!(album_folder(&path(2, "Viagem/metadata.json")), album_folder(&path(1, "Viagem/a.jpg")), "parts merge");
    assert_eq!(album_folder("Takeout/Google Photos/Trip/a.jpg").as_deref(), Some("Takeout/Google Photos/Trip"));
    assert_eq!(album_folder(&path(1, "a.jpg")), None, "loose in Google Fotos");
    assert_eq!(album_folder(&path(1, "Viagem/sub/a.jpg")), None, "deeper than an album");
    assert_eq!(album_folder("Ruan/2020/a.jpg"), None, "outside the Takeout");
}

#[test]
fn alb02_rebuild_names_links_updates_and_drops_albums() {
    let mut connection = database();
    let a = media(&connection, &path(1, "Viagem/a.jpg"), "photo", Some("2022-08-10T10:00:00Z"), false);
    let b = media(&connection, &path(2, "Viagem/b.mp4"), "video", Some("2022-08-14T10:00:00Z"), false);
    let c = media(&connection, &path(1, "Fotos de 2021/c.jpg"), "photo", Some("2021-01-05T10:00:00Z"), false);
    let d = media(&connection, &path(1, "Fotos de 2021/d.jpg"), "photo", None, true);
    media(&connection, "Ruan/2020/e.jpg", "photo", None, false);
    let report = DiscoveryReport {
        media: vec![
            hit(&path(1, "Viagem/a.jpg")),
            hit(&path(2, "Viagem/b.mp4")),
            hit(&path(1, "Fotos de 2021/c.jpg")),
            hit(&path(1, "Fotos de 2021/d.jpg")),
            hit("Ruan/2020/e.jpg"),
            hit(&path(1, "Vazio/sumiu.jpg")),
        ],
        albums: vec![metadata(&path(2, "Viagem/metadata.json"), Some("Viagem Europa 2022"), Some("Lisboa e Porto"))],
        problems: Vec::new(),
    };
    assert_eq!(rebuild_takeout_albums(&mut connection, 1, &report, NOW).unwrap(), 3);

    let cards = album_cards(&connection).unwrap();
    assert_eq!(
        cards,
        vec![
            AlbumCard {
                id: cards[0].id,
                name: "Viagem Europa 2022".into(),
                count: 2,
                first: Some("2022-08-10T10:00:00Z".into()),
                last: Some("2022-08-14T10:00:00Z".into()),
                covers: vec![a, b],
            },
            // The hidden item is linked but neither counts nor covers.
            AlbumCard {
                id: cards[1].id,
                name: "Fotos de 2021".into(),
                count: 1,
                first: Some("2021-01-05T10:00:00Z".into()),
                last: Some("2021-01-05T10:00:00Z".into()),
                covers: vec![c],
            },
        ],
        "\"Vazio\" has no indexed media and is not shown"
    );
    let description: Option<String> = connection
        .query_row("SELECT description FROM albums WHERE id = ?1", [cards[0].id], |row| row.get(0))
        .unwrap();
    assert_eq!(description.as_deref(), Some("Lisboa e Porto"));
    let linked: i64 = connection.query_row("SELECT count(*) FROM media_albums WHERE media_id = ?1", [d], |row| row.get(0)).unwrap();
    assert_eq!(linked, 1);

    // Next run: the trip lost its metadata.json and a photo; 2021 is gone.
    let viagem = cards[0].id;
    let report = DiscoveryReport { media: vec![hit(&path(1, "Viagem/a.jpg"))], albums: Vec::new(), problems: Vec::new() };
    assert_eq!(rebuild_takeout_albums(&mut connection, 1, &report, NOW).unwrap(), 1);
    let cards = album_cards(&connection).unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!((cards[0].id, cards[0].name.as_str(), cards[0].count), (viagem, "Viagem", 1), "same album, folder name");
    let albums: i64 = connection.query_row("SELECT count(*) FROM albums", [], |row| row.get(0)).unwrap();
    assert_eq!(albums, 1);
}
