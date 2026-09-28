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

pub(crate) fn query_seek_penalty(volume_root: &str) -> Option<bool> {
    #[cfg(windows)]
    {
        query_seek_penalty_windows(volume_root)
    }
    #[cfg(not(windows))]
    {
        let _ = volume_root;
        None
    }
}

#[cfg(windows)]
fn query_seek_penalty_windows(volume_root: &str) -> Option<bool> {
    // SPEC_DEVIATION: Cargo enables windows-sys feature Win32_Security.
    // Reason: windows-sys 0.61.2 gates CreateFileW on that feature because the
    // signature references SECURITY_ATTRIBUTES. The listed FileSystem feature is not enough.
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, StorageDeviceSeekPenaltyProperty,
        DEVICE_SEEK_PENALTY_DESCRIPTOR, STORAGE_PROPERTY_QUERY,
    };

    let letter = volume_root
        .chars()
        .next()
        .filter(|letter| letter.is_ascii_alphabetic())?;
    let path = format!(r"\\.\{}:", letter.to_ascii_uppercase());
    let wide: Vec<u16> = OsStr::new(&path).encode_wide().chain(Some(0)).collect();

    unsafe {
        let handle = CreateFileW(
            wide.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return None;
        }

        let query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceSeekPenaltyProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut descriptor = DEVICE_SEEK_PENALTY_DESCRIPTOR::default();
        let mut returned = 0u32;
        let ok = DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const STORAGE_PROPERTY_QUERY as *const _,
            std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            &mut descriptor as *mut DEVICE_SEEK_PENALTY_DESCRIPTOR as *mut _,
            std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
            &mut returned,
            std::ptr::null_mut(),
        );
        let _ = CloseHandle(handle);
        if ok == 0 {
            None
        } else {
            Some(descriptor.IncursSeekPenalty)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::model::VolumeClass;
    use super::{classify_seek_penalty, query_seek_penalty, volume_root_of};

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

    #[test]
    fn temp_volume_seek_penalty_classifies_without_panicking() {
        let root = volume_root_of(std::env::temp_dir().as_path());
        let penalty = query_seek_penalty(&root);
        let class = classify_seek_penalty(penalty);
        assert!(root.ends_with(":\\"), "{root}");
        assert!(matches!(
            (penalty, class),
            (Some(true), VolumeClass::Hdd)
                | (Some(false), VolumeClass::Ssd)
                | (None, VolumeClass::Unknown)
        ));
    }
}
