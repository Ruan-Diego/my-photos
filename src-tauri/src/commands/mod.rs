pub mod albums;
pub mod backup;
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

    const CONFIG: &str = include_str!("../../tauri.conf.json");
    const CARGO: &str = include_str!("../../Cargo.toml");

    #[test]
    fn sec03_the_window_gets_only_what_the_app_uses() {
        let capability: serde_json::Value = serde_json::from_str(CAPABILITY).unwrap();
        for permission in capability["permissions"].as_array().unwrap() {
            let permission = permission.as_str().unwrap();
            // App commands, and the events the indexing progress arrives by. No core:default
            // (windows, menus, tray), no file system, shell, opener or http plugin.
            assert!(
                permission.starts_with("allow-") || permission == "core:event:default",
                "{permission} is more than the app uses"
            );
        }
    }

    #[test]
    fn sec04_no_network_by_default() {
        let config: serde_json::Value = serde_json::from_str(CONFIG).unwrap();
        let csp = config["app"]["security"]["csp"].as_str().expect("a CSP is set");
        for directive in csp.split(';').map(str::trim) {
            for source in directive.split_whitespace().skip(1) {
                let remote = source.starts_with("http") && !source.ends_with(".localhost");
                assert!(!remote && source != "*" && source != "https:", "{directive} reaches the network");
            }
        }
        assert!(csp.contains("default-src 'self'") && csp.contains("script-src 'self';"), "{csp}");
        assert!(!csp.contains("unsafe-eval"), "{csp}");
        // Crates that talk to the network. An updater (F37) must be opt-in and listed here.
        let dependencies = &CARGO[CARGO.find("[dependencies]").unwrap()..];
        for crate_name in ["reqwest", "hyper", "ureq", "curl", "tauri-plugin-http", "tauri-plugin-updater", "tauri-plugin-opener"] {
            assert!(!dependencies.contains(&format!("\n{crate_name} ")), "{crate_name} in Cargo.toml");
        }
    }
}
