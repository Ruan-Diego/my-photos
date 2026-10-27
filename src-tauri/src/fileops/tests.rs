use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use super::*;
use crate::db::MIGRATIONS;

const NOW: &str = "2026-10-27T20:00:00Z";
const LATER: &str = "2026-11-27T20:00:00Z";

struct Library {
    root: PathBuf,
    connection: Connection,
}

impl Drop for Library {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn library(name: &str) -> Library {
    let root = std::env::temp_dir().join(format!("myphotos-fileops-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".myphotos")).unwrap();
    let connection = Connection::open_in_memory().unwrap();
    connection.pragma_update(None, "foreign_keys", "ON").unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', ?1, 't')", [root.to_str().unwrap()])
        .unwrap();
    Library { root, connection }
}

impl Library {
    fn folder(&self, path: &str) {
        fs::create_dir_all(on_disk(&self.root, path)).unwrap();
        self.connection
            .execute("INSERT INTO folders (library_id, relative_path, created_at) VALUES (1, ?1, 't')", [path])
            .unwrap();
    }

    /// A media file (and its JSON when `json`), indexed.
    fn media(&self, path: &str, json: bool) -> i64 {
        fs::write(on_disk(&self.root, path), path.as_bytes()).unwrap();
        let json_path = json.then(|| format!("{path}.supplemental-metadata.json"));
        if let Some(json_path) = &json_path {
            fs::write(on_disk(&self.root, json_path), b"{}").unwrap();
        }
        self.connection
            .execute(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, has_takeout_json,
                   json_relative_path, created_at, updated_at)
                 VALUES (1, ?1, ?2, 'jpg', 'photo', ?3, ?4, 't', 't')",
                params![path, name_of(path), json as i64, json_path],
            )
            .unwrap();
        self.connection.last_insert_rowid()
    }

    fn path_of(&self, id: i64) -> (String, Option<String>, String) {
        self.connection
            .query_row(
                "SELECT relative_path, json_relative_path, indexing_status FROM media_items WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        on_disk(&self.root, path).exists()
    }
}

fn paths(folders: &[Folder]) -> Vec<(&str, u64, u64)> {
    folders.iter().map(|f| (f.path.as_str(), f.direct, f.total)).collect()
}

#[test]
fn fop01_tree_counts_direct_and_nested_media() {
    let lib = library("fop01");
    lib.folder("Takeout");
    lib.folder("Takeout/Viagem");
    lib.folder("Vazia");
    lib.media("solta.jpg", false);
    lib.media("Takeout/a.jpg", false);
    lib.media("Takeout/Viagem/b.jpg", true);
    lib.media("Takeout/Viagem/c.jpg", false);
    let tree = folder_tree(&lib.connection).unwrap();
    assert_eq!(paths(&tree), vec![("", 1, 4), ("Takeout", 1, 3), ("Takeout/Viagem", 2, 2), ("Vazia", 0, 0)]);
    assert_eq!(tree[2].depth, 2);
    assert_eq!(tree[2].name, "Viagem");
    let items = folder_items(&lib.connection, "Takeout/Viagem").unwrap();
    assert_eq!(items.iter().map(|i| (i.filename.as_str(), i.has_json)).collect::<Vec<_>>(), vec![("b.jpg", true), ("c.jpg", false)]);
}

#[test]
fn fop02_names_windows_refuses_are_refused() {
    assert_eq!(valid_name("  Portugal ").unwrap(), "Portugal");
    for bad in ["", "a/b", "a\\b", "x:", "que?", "fim.", "CON", "com1", "LPT9.txt", ".myphotos"] {
        assert!(valid_name(bad).is_err(), "{bad}");
    }
    assert!(valid_name("Console").is_ok());
}

#[test]
fn fop03_move_takes_the_json_along_and_skips_name_clashes() {
    let lib = library("fop03");
    lib.folder("Viagem");
    lib.folder("Organizadas");
    let a = lib.media("Viagem/a.jpg", true);
    let b = lib.media("Viagem/b.jpg", false);
    fs::write(lib.root.join("Organizadas").join("b.jpg"), b"other").unwrap();
    let outcome = move_media(&lib.connection, &lib.root, &[a, b], "Organizadas", NOW).unwrap();
    assert_eq!(outcome.done, 1);
    assert_eq!(outcome.skipped.len(), 1);
    assert_eq!(outcome.skipped[0].0, "b.jpg");
    assert_eq!(
        lib.path_of(a),
        ("Organizadas/a.jpg".to_string(), Some("Organizadas/a.jpg.supplemental-metadata.json".to_string()), "ok".to_string())
    );
    assert!(lib.exists("Organizadas/a.jpg") && lib.exists("Organizadas/a.jpg.supplemental-metadata.json"));
    assert!(!lib.exists("Viagem/a.jpg") && !lib.exists("Viagem/a.jpg.supplemental-metadata.json"));
    assert_eq!(lib.path_of(b).0, "Viagem/b.jpg");
    assert!(lib.exists("Viagem/b.jpg"));
    // A busca acompanha o caminho novo.
    let found: i64 = lib
        .connection
        .query_row("SELECT rowid FROM media_search WHERE media_search MATCH 'organizadas'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(found, a);
}

#[test]
fn fop04_create_and_rename_folder_repaths_everything_inside() {
    let mut lib = library("fop04");
    lib.folder("Viagem");
    lib.folder("Viagem/Dia 1");
    let a = lib.media("Viagem/Dia 1/a.jpg", true);
    assert_eq!(create_folder(&lib.connection, &lib.root, "Viagem", "Dia 2", NOW).unwrap(), "Viagem/Dia 2");
    assert!(lib.exists("Viagem/Dia 2"));
    assert!(create_folder(&lib.connection, &lib.root, "Viagem", "dia 2", NOW).is_err());
    assert_eq!(rename_folder(&mut lib.connection, &lib.root, "Viagem", "Portugal 2022").unwrap(), "Portugal 2022");
    assert!(lib.exists("Portugal 2022/Dia 1/a.jpg") && !lib.exists("Viagem"));
    assert_eq!(
        lib.path_of(a),
        (
            "Portugal 2022/Dia 1/a.jpg".to_string(),
            Some("Portugal 2022/Dia 1/a.jpg.supplemental-metadata.json".to_string()),
            "ok".to_string()
        )
    );
    let tree = folder_tree(&lib.connection).unwrap();
    assert_eq!(
        tree.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
        vec!["", "Portugal 2022", "Portugal 2022/Dia 1", "Portugal 2022/Dia 2"]
    );
    assert!(rename_folder(&mut lib.connection, &lib.root, "", "x").is_err());
}

#[test]
fn fop05_trash_and_restore_a_media_with_its_json() {
    let mut lib = library("fop05");
    lib.folder("Viagem");
    let a = lib.media("Viagem/a.jpg", true);
    let outcome = trash_media(&lib.connection, &lib.root, &[a], NOW).unwrap();
    assert_eq!(outcome.done, 1);
    assert!(!lib.exists("Viagem/a.jpg") && !lib.exists("Viagem/a.jpg.supplemental-metadata.json"));
    assert_eq!(lib.path_of(a).2, "missing_file");
    assert_eq!(folder_tree(&lib.connection).unwrap()[0].total, 0);
    let trash = trash_list(&lib.connection).unwrap();
    assert_eq!(trash.len(), 1);
    assert_eq!((trash[0].name.as_str(), trash[0].folder.as_str(), trash[0].media_id), ("a.jpg", "Viagem", Some(a)));
    assert_eq!(trash[0].purge_after, "2026-11-26T20:00:00Z");

    restore(&mut lib.connection, &lib.root, trash[0].id, NOW).unwrap();
    assert!(lib.exists("Viagem/a.jpg") && lib.exists("Viagem/a.jpg.supplemental-metadata.json"));
    assert_eq!(lib.path_of(a).2, "ok");
    assert!(trash_list(&lib.connection).unwrap().is_empty());
    assert_eq!(fs::read_dir(lib.root.join(".myphotos").join("trash")).unwrap().count(), 0);
}

#[test]
fn fop06_trash_and_restore_a_whole_folder() {
    let mut lib = library("fop06");
    lib.folder("Rascunhos");
    lib.folder("Rascunhos/Sub");
    let a = lib.media("Rascunhos/a.jpg", false);
    let b = lib.media("Rascunhos/Sub/b.jpg", true);
    lib.media("fora.jpg", false);
    trash_folder(&mut lib.connection, &lib.root, "Rascunhos", NOW).unwrap();
    assert!(!lib.exists("Rascunhos"));
    assert_eq!(paths(&folder_tree(&lib.connection).unwrap()), vec![("", 1, 1)]);
    let trash = trash_list(&lib.connection).unwrap();
    assert_eq!((trash[0].kind.as_str(), trash[0].items), ("folder", 2));
    restore(&mut lib.connection, &lib.root, trash[0].id, LATER).unwrap();
    assert!(lib.exists("Rascunhos/Sub/b.jpg"));
    assert_eq!((lib.path_of(a).2, lib.path_of(b).2), ("ok".to_string(), "ok".to_string()));
    assert_eq!(paths(&folder_tree(&lib.connection).unwrap()), vec![("", 1, 3), ("Rascunhos", 1, 2), ("Rascunhos/Sub", 1, 1)]);
}

#[test]
fn fop07_restore_refuses_to_overwrite_and_restore_all_reports_it() {
    let mut lib = library("fop07");
    lib.folder("Viagem");
    let a = lib.media("Viagem/a.jpg", false);
    let b = lib.media("Viagem/b.jpg", false);
    trash_media(&lib.connection, &lib.root, &[a, b], NOW).unwrap();
    fs::write(lib.root.join("Viagem").join("a.jpg"), b"new").unwrap();
    let outcome = restore_all(&mut lib.connection, &lib.root, NOW).unwrap();
    assert_eq!(outcome.done, 1);
    assert_eq!(outcome.skipped[0].0, "a.jpg");
    assert_eq!(fs::read(lib.root.join("Viagem").join("a.jpg")).unwrap(), b"new");
    assert_eq!(trash_list(&lib.connection).unwrap().len(), 1);
}

#[test]
fn fop08_after_30_days_the_trash_is_emptied_for_good() {
    let mut lib = library("fop08");
    lib.folder("Viagem");
    let a = lib.media("Viagem/a.jpg", true);
    let b = lib.media("Viagem/b.jpg", false);
    trash_media(&lib.connection, &lib.root, &[a], NOW).unwrap();
    trash_media(&lib.connection, &lib.root, &[b], "2026-11-20T00:00:00Z").unwrap();
    let thumb = thumb_path(&lib.root, a, ThumbSize::Small);
    fs::create_dir_all(thumb.parent().unwrap()).unwrap();
    fs::write(&thumb, b"jpg").unwrap();
    assert_eq!(purge_expired(&mut lib.connection, &lib.root, "2026-11-26T19:59:59Z").unwrap(), 0);
    assert_eq!(purge_expired(&mut lib.connection, &lib.root, LATER).unwrap(), 1);
    let left: i64 = lib.connection.query_row("SELECT count(*) FROM media_items", [], |row| row.get(0)).unwrap();
    assert_eq!(left, 1);
    assert!(!thumb.exists());
    let trash = trash_list(&lib.connection).unwrap();
    assert_eq!(trash.len(), 1);
    assert_eq!(trash[0].media_id, Some(b));
    assert_eq!(fs::read_dir(lib.root.join(".myphotos").join("trash")).unwrap().count(), 1);
}

#[test]
fn fop09_paths_helpers() {
    assert_eq!(parent_of("a/b/c.jpg"), "a/b");
    assert_eq!(parent_of("c.jpg"), "");
    assert_eq!(name_of("a/b"), "b");
    assert!(under("A/b.jpg", "a"));
    assert!(!under("ab/c.jpg", "a"));
    assert_eq!(slot_of(".myphotos/trash/2026-3/Pasta/x.jpg").as_deref(), Some(".myphotos/trash/2026-3"));
    assert_eq!(slot_of("Fotos/a.jpg"), None);
    assert_eq!(slot_of(".myphotos/trash/../x"), None);
    assert_eq!(slot_of(".myphotos/library.db/x/y"), None);
    let _ = Path::new("");
}
