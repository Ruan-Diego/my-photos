use std::path::Path;
#[cfg(windows)]
use std::path::{Component, Prefix};

use super::model::VolumeClass;

pub(crate) fn classify_seek_penalty(incurs: Option<bool>) -> VolumeClass {
    match incurs {
        Some(true) => VolumeClass::Hdd,
        Some(false) => VolumeClass::Ssd,
        None => VolumeClass::Unknown,
    }
}

pub(crate) fn volume_root_of(root: &Path) -> String {
    #[cfg(windows)]
    {
        for component in root.components() {
            if let Component::Prefix(prefix) = component {
                if let Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) = prefix.kind() {
                    let letter = (letter as char).to_ascii_uppercase();
                    return format!("{letter}:\\");
                }
                return String::new();
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = root;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::model::VolumeClass;
    use super::{classify_seek_penalty, volume_root_of};

    #[test]
    fn seek_penalty_true_is_hdd() {
        assert_eq!(classify_seek_penalty(Some(true)), VolumeClass::Hdd);
    }

    #[test]
    fn seek_penalty_false_is_ssd() {
        assert_eq!(classify_seek_penalty(Some(false)), VolumeClass::Ssd);
    }

    #[test]
    fn absent_seek_penalty_is_unknown() {
        assert_eq!(classify_seek_penalty(None), VolumeClass::Unknown);
    }

    #[test]
    fn volume_root_of_uppercases_a_drive_letter() {
        assert_eq!(volume_root_of(Path::new(r"d:\temp\bench")), r"D:\");
    }
}
