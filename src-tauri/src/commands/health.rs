use serde::Serialize;

use crate::db;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub app_name: String,
    pub sqlite_ok: bool,
}

pub fn health_from_probe(probe: Result<i64, String>) -> Result<AppHealth, String> {
    match probe {
        Ok(1) => Ok(AppHealth {
            app_name: "My Photos".to_string(),
            sqlite_ok: true,
        }),
        Ok(value) => Err(format!("SELECT 1 retornou {value}")),
        Err(message) => Err(message),
    }
}

#[tauri::command]
pub fn app_health() -> Result<AppHealth, String> {
    health_from_probe(db::select_one())
}

#[cfg(test)]
mod tests {
    use super::health_from_probe;

    #[test]
    fn probe_of_one_reports_my_photos_ready() {
        let health = health_from_probe(Ok(1)).expect("probe");
        assert_eq!(health.app_name, "My Photos");
        assert_eq!(health.sqlite_ok, true);
    }

    #[test]
    fn probe_other_than_one_is_an_error() {
        let message = health_from_probe(Ok(2)).expect_err("not one");
        assert_eq!(message, "SELECT 1 retornou 2");
    }

    #[test]
    fn probe_error_is_returned_unchanged() {
        let message = health_from_probe(Err("disco".to_string())).expect_err("probe error");
        assert_eq!(message, "disco");
    }
}
