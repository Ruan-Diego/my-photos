use rusqlite::Connection;

pub fn select_one() -> Result<i64, String> {
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::select_one;
    use std::env;
    use std::fs;
    use std::path::Path;

    #[test]
    fn select_one_returns_1() {
        let directory = env::current_dir().expect("cwd");
        let before = database_file_names(&directory);

        assert_eq!(select_one().expect("SELECT 1"), 1);
        assert_eq!(database_file_names(&directory), before);
    }

    #[test]
    fn second_call_returns_1_without_creating_a_db_file() {
        let directory = env::current_dir().expect("cwd");
        let before = database_file_names(&directory);

        assert_eq!(select_one().expect("first SELECT 1"), 1);
        assert_eq!(select_one().expect("second SELECT 1"), 1);
        assert_eq!(database_file_names(&directory), before);
    }

    fn database_file_names(directory: &Path) -> Vec<String> {
        let mut names = fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("read {directory:?}: {error}"))
            .map(|entry| {
                entry
                    .unwrap_or_else(|error| panic!("dir entry: {error}"))
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with(".db"))
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}
