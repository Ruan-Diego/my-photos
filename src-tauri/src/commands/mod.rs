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
pub mod update;
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
        // Crates that talk to the network. The updater (F37) is the one exception: opt-in,
        // registered only in release builds, asked only by the person (sec05).
        let dependencies = &CARGO[CARGO.find("[dependencies]").unwrap()..];
        for crate_name in ["reqwest", "hyper", "ureq", "curl", "tauri-plugin-http", "tauri-plugin-opener"] {
            assert!(!dependencies.contains(&format!("\n{crate_name} ")), "{crate_name} in Cargo.toml");
        }
        assert!(config["plugins"].get("updater").is_none(), "a development build has no updater");
    }

    const RELEASE_CONFIG: &str = include_str!("../../tauri.release.conf.json");
    const UPDATE: &str = include_str!("update.rs");
    const SETTINGS: &str = include_str!("../settings/mod.rs");

    /// F37: the updater is the only network code, and it stays quiet unless the person asks.
    #[test]
    fn sec07_the_updater_is_opt_in() {
        // Registered through `update::register`, which needs the release config's key.
        assert!(LIB.contains("commands::update::register(app)?"));
        assert!(!LIB.contains("tauri_plugin_updater"), "the plugin is never registered unconditionally");
        assert!(UPDATE.contains("if configured(app.config())"));
        // "Procurar atualizações ao abrir" starts off.
        assert!(SETTINGS.contains("flag(connection, UPDATE_CHECKS, false)"));
        // Updates come signed over HTTPS and install without asking for administrator.
        let release: serde_json::Value = serde_json::from_str(RELEASE_CONFIG).unwrap();
        let updater = &release["plugins"]["updater"];
        assert!(updater["endpoints"].as_array().unwrap().iter().all(|url| url.as_str().unwrap().starts_with("https://")));
        assert_eq!(updater["windows"]["installMode"], "passive");
        assert_eq!(release["bundle"]["createUpdaterArtifacts"], true);
    }

    const PACKAGE: &str = include_str!("../../../package.json");

    #[test]
    fn pkg01_one_version_and_a_per_user_portuguese_installer() {
        let config: serde_json::Value = serde_json::from_str(CONFIG).unwrap();
        let package: serde_json::Value = serde_json::from_str(PACKAGE).unwrap();
        // The installer takes the version from package.json; Cargo.toml must say the same.
        assert_eq!(config["version"], "../package.json");
        let version = package["version"].as_str().unwrap();
        assert!(CARGO.contains(&format!("\nversion = \"{version}\"\n")), "Cargo.toml is not at {version}");
        let bundle = &config["bundle"];
        assert_eq!(bundle["targets"], serde_json::json!(["nsis"]));
        let nsis = &bundle["windows"]["nsis"];
        // Per user: installs without administrator, like the library it opens on E:.
        assert_eq!(nsis["installMode"], "currentUser");
        assert_eq!(nsis["languages"], serde_json::json!(["PortugueseBR"]));
        assert_eq!(config["productName"], "My Photos");
    }

    const GITIGNORE: &str = include_str!("../../.gitignore");

    /// F37: the release installer carries the LGPL ffmpeg where `locate_ffmpeg` looks
    /// first (beside the executable), with its license and where its source is.
    #[test]
    fn pkg02_the_release_installer_carries_the_lgpl_ffmpeg() {
        let release: serde_json::Value = serde_json::from_str(RELEASE_CONFIG).unwrap();
        let resources = release["bundle"]["resources"].as_object().expect("resources map");
        let targets: Vec<&str> = resources.values().filter_map(|target| target.as_str()).collect();
        // Flat names, no folder: the install directory is the executable's.
        for tool in ["ffmpeg", "ffprobe"] {
            let name = format!("{tool}.exe");
            assert!(targets.contains(&name.as_str()), "{name} is not installed beside the app");
        }
        // The installed folder, as the installer lays it out, is one `locate_ffmpeg` accepts.
        #[cfg(windows)]
        {
            let scratch = crate::db::tests::Scratch::new("pkg02");
            for target in &targets {
                std::fs::write(scratch.path.join(target), b"").unwrap();
            }
            assert!(crate::thumbs::video::locate_in([scratch.path.clone()]).is_some());
        }
        assert!(targets.contains(&"ffmpeg-LICENSE.txt") && targets.contains(&"ffmpeg-README.txt"), "LGPL license and source");
        // Fetched by scripts/fetch-ffmpeg.ps1, never committed.
        assert!(resources.keys().all(|source| source.starts_with("binaries/ffmpeg/")));
        assert!(GITIGNORE.lines().any(|line| line.trim() == "/binaries/"));
        // Development builds stay without it (and still find one on PATH).
        let config: serde_json::Value = serde_json::from_str(CONFIG).unwrap();
        assert!(config["bundle"].get("resources").is_none());
    }
}
