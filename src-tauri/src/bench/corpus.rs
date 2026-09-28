use std::fs;
use std::path::{Component, Path, Prefix};

use super::model::BenchError;

pub(crate) fn file_bytes(index: u32) -> [u8; 4096] {
    let mut bytes = [0u8; 4096];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = ((index + offset as u32) % 251) as u8;
    }
    bytes
}

pub(crate) fn relative_path(index: u32) -> String {
    let year = 2000 + (index / 1200);
    let month = ((index / 100) % 12) + 1;
    format!("f03/{year}/{month:02}/item-{index:07}.bin")
}

pub(crate) fn validate(root: &Path, item_count: u32) -> Result<(), BenchError> {
    if item_count < 1 || item_count > 150_000 {
        return Err(BenchError::CountOutOfRange);
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        return Err(BenchError::RootNotAbsolute);
    }
    #[cfg(windows)]
    validate_windows(root)
}

#[cfg(windows)]
fn validate_windows(root: &Path) -> Result<(), BenchError> {
    if is_volume_root(root) {
        return Err(BenchError::RootIsVolume);
    }
    if !is_drive_absolute(root) || has_dot_component(root) {
        return Err(BenchError::RootNotAbsolute);
    }
    let Some(parent) = root.parent().filter(|parent| !parent.as_os_str().is_empty()) else {
        return Err(BenchError::ParentMissing);
    };
    if !parent.exists() {
        return Err(BenchError::ParentMissing);
    }
    if root.exists() {
        let mut entries = fs::read_dir(root).map_err(|_| BenchError::RootNotEmpty)?;
        if entries.next().is_some() {
            return Err(BenchError::RootNotEmpty);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_drive_absolute(root: &Path) -> bool {
    if !root.is_absolute() {
        return false;
    }
    matches!(
        root.components().next(),
        Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
    )
}

#[cfg(windows)]
fn is_volume_root(root: &Path) -> bool {
    let mut saw_drive = false;
    for component in root.components() {
        match component {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(_) | Prefix::VerbatimDisk(_) => saw_drive = true,
                _ => return false,
            },
            Component::RootDir => {}
            _ => return false,
        }
    }
    saw_drive
}

#[cfg(windows)]
fn has_dot_component(root: &Path) -> bool {
    root.components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{file_bytes, relative_path, validate};

    #[test]
    fn file_bytes_are_4096_and_each_byte_is_index_plus_offset_mod_251() {
        let bytes = file_bytes(250);
        assert_eq!(bytes.len(), 4096);
        for (offset, byte) in bytes.iter().enumerate() {
            assert_eq!(*byte, ((250 + offset as u32) % 251) as u8);
        }
    }

    #[test]
    fn relative_path_uses_slash_and_the_year_month_formula() {
        assert_eq!(relative_path(0), "f03/2000/01/item-0000000.bin");
        assert_eq!(relative_path(100), "f03/2000/02/item-0000100.bin");
        assert_eq!(relative_path(1199), "f03/2000/12/item-0001199.bin");
        assert_eq!(relative_path(1200), "f03/2001/01/item-0001200.bin");
    }

    #[test]
    fn count_bounds_reject_zero_and_150001_and_accept_the_inclusive_ends() {
        let parent = scratch("count");
        let root = parent.join("corpus");
        assert_eq!(
            validate(&root, 0).unwrap_err().as_code(),
            "count_out_of_range"
        );
        assert_eq!(
            validate(&root, 150_001).unwrap_err().as_code(),
            "count_out_of_range"
        );
        assert!(validate(&root, 1).is_ok());
        assert!(validate(&root, 150_000).is_ok());
        assert!(!root.exists());
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn relative_and_unc_roots_are_not_absolute() {
        assert_eq!(
            validate(Path::new(r"relative\root"), 1)
                .unwrap_err()
                .as_code(),
            "root_not_absolute"
        );
        assert_eq!(
            validate(Path::new(r"\\server\share\root"), 1)
                .unwrap_err()
                .as_code(),
            "root_not_absolute"
        );
    }

    #[test]
    fn parent_dir_component_is_not_absolute() {
        let parent = scratch("dots");
        let marker = parent.join("marker.bin");
        fs::write(&marker, b"keep").unwrap();
        let root = parent.join("..").join("elsewhere");
        assert_eq!(
            validate(&root, 1).unwrap_err().as_code(),
            "root_not_absolute"
        );
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn volume_root_is_rejected() {
        for root in [r"C:\", r"C:", r"\\?\C:\"] {
            assert_eq!(
                validate(Path::new(root), 1).unwrap_err().as_code(),
                "root_is_volume",
                "{root}"
            );
        }
    }

    #[test]
    fn missing_parent_creates_nothing() {
        let parent = scratch("missing");
        let absent_parent = parent.join("does-not-exist");
        let root = absent_parent.join("corpus");
        assert_eq!(
            validate(&root, 1).unwrap_err().as_code(),
            "parent_missing"
        );
        assert!(!absent_parent.exists());
        assert!(!root.exists());
        let _ = fs::remove_dir_all(&parent);
    }

    #[test]
    fn empty_root_is_usable_and_absent_root_stays_absent() {
        let parent = scratch("empty");
        let empty = parent.join("empty");
        fs::create_dir(&empty).unwrap();
        assert!(validate(&empty, 1).is_ok());
        assert!(fs::read_dir(&empty).unwrap().next().is_none());

        let absent = parent.join("absent");
        assert!(validate(&absent, 1).is_ok());
        assert!(!absent.exists());

        let occupied = parent.join("occupied");
        fs::create_dir(&occupied).unwrap();
        let marker = occupied.join("marker.bin");
        fs::write(&marker, b"keep").unwrap();
        assert_eq!(
            validate(&occupied, 1).unwrap_err().as_code(),
            "root_not_empty"
        );
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
        let _ = fs::remove_dir_all(&parent);
    }

    fn scratch(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "my-photos-f03-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
