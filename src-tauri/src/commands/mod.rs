pub mod albums;
pub mod duplicates;
pub mod favorites;
pub mod fileops;
pub mod problems;
pub mod details;
pub mod health;
pub mod library;
pub mod locations;
pub mod onboarding;
pub mod people;
pub mod search;
pub mod settings;
pub mod timeline;
pub mod viewer;

#[cfg(test)]
mod tests {
    // Once permissions/ defines one app permission, Tauri denies every app command
    // the capability does not allow: the call fails in the UI with no Rust error.
    const LIB: &str = include_str!("../lib.rs");
    const PERMISSIONS: &[&str] = &[
        include_str!("../../permissions/app-health.toml"),
        include_str!("../../permissions/library.toml"),
    ];
    const CAPABILITY: &str = include_str!("../../capabilities/default.json");

    fn registered() -> Vec<&'static str> {
        let start = LIB.find("generate_handler![").expect("handler list") + "generate_handler![".len();
        let end = start + LIB[start..].find(']').expect("end of handler list");
        LIB[start..end]
            .split(',')
            .filter_map(|path| path.trim().rsplit("::").next())
            .filter(|name| !name.is_empty())
            .collect()
    }

    /// The permission identifier that allows `command`, if any.
    fn permission_for(command: &str) -> Option<String> {
        let allow = format!("commands.allow = [\"{command}\"]");
        PERMISSIONS.iter().find_map(|file| {
            file.split("[[permission]]").find(|block| block.contains(&allow)).and_then(|block| {
                let line = block.lines().find(|line| line.starts_with("identifier"))?;
                Some(line.split('"').nth(1)?.to_string())
            })
        })
    }

    #[test]
    fn acl01_every_registered_command_is_allowed() {
        let commands = registered();
        assert!(commands.len() >= 4, "{commands:?}");
        let capability: serde_json::Value = serde_json::from_str(CAPABILITY).unwrap();
        let granted: Vec<&str> = capability["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|permission| permission.as_str())
            .collect();
        for command in commands {
            let permission = permission_for(command).unwrap_or_else(|| panic!("no permission allows {command}"));
            assert!(granted.contains(&permission.as_str()), "capability misses {permission}");
        }
    }
}
