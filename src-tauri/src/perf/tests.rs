use super::*;
use crate::search::{search, search_ids, SearchQuery};

#[test]
fn chrono_like_matches_known_dates() {
    assert_eq!(chrono_like(0), "1970-01-01T00:00:00Z");
    assert_eq!(chrono_like(1_700_000_000), "2023-11-14T22:13:20Z");
    assert_eq!(chrono_like(951_782_400), "2000-02-29T00:00:00Z");
}

#[test]
#[ignore = "F32: mede 300 mil itens; rode com --ignored --nocapture"]
fn perf01_every_screen_on_300k_items() {
    let path = std::env::temp_dir().join(format!("myphotos-perf-{}.db", std::process::id()));
    let start = Instant::now();
    let connection = build(&path, ITEMS);
    println!("build {ITEMS} itens: {} ms", start.elapsed().as_millis());
    let mut lines = Vec::new();
    let mut time = |name: &str, ms: u128| {
        println!("{name:<28} {ms:>6} ms");
        lines.push((name.to_string(), ms));
    };
    time("timeline", best(|| crate::commands::timeline::timeline_items(&connection).unwrap().ids.len()));
    time("library_summary", best(|| crate::commands::library::summary(&connection, true).unwrap().media_count));
    let text = SearchQuery { text: "album 042".into(), ..Default::default() };
    time("search texto", best(|| search_ids(&connection, &text).unwrap()));
    let person = SearchQuery { people: vec![7], ..Default::default() };
    time("search pessoa", best(|| search_ids(&connection, &person).unwrap()));
    let place = SearchQuery { place: Some("lisbon".into()), ..Default::default() };
    time("search local", best(|| search_ids(&connection, &place).unwrap()));
    let page = SearchQuery { text: "2022".into(), limit: Some(200), ..Default::default() };
    time("search página", best(|| search(&connection, &page).unwrap()));
    time("search_options", best(|| crate::search::search_options(&connection).unwrap()));
    time("people", best(|| crate::commands::people::people_cards(&connection).unwrap().len()));
    time("locations", best(|| crate::commands::locations::locations(&connection).unwrap()));
    time("albums", best(|| crate::albums::album_cards(&connection).unwrap().len()));
    time("duplicates", best(|| crate::duplicates::duplicates(&connection).unwrap().groups.len()));
    time("problems overview", best(|| crate::problems::overview(&connection, true).unwrap()));
    time("folder_tree", best(|| crate::fileops::folder_tree(&connection).unwrap().len()));
    time("folder_items", best(|| crate::fileops::folder_items(&connection, "takeout-0/Takeout/Google Fotos/Album 000").unwrap().len()));
    time("settings", best(|| crate::settings::settings(&connection, true).unwrap()));
    drop(connection);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(path.with_extension("db-wal"));
    let _ = std::fs::remove_file(path.with_extension("db-shm"));
}

