//! F11: finds the library disk by its volume serial number instead of a fixed drive
//! letter, and keeps `libraries.root_path` pointing at wherever that disk is mounted now.
//! The database lives on the library disk (AD-011), so "disconnected" means no library.

#[cfg(test)]
mod tests;

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

/// AD-007: the drive letter the library had when it was set up; tried first.
pub const PREFERRED_ROOT: &str = r"E:\";

/// The database inside the library root (AD-011).
pub const DB_RELATIVE: &str = r".myphotos\library.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Windows volume serial as `dir` prints it: `1A2B-3C4D`.
    pub serial: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocateError {
    /// No mounted drive has `.myphotos\library.db`.
    NotFound,
    /// More than one does and none is `E:\`.
    Ambiguous,
}

impl LocateError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::NotFound => "library_disk_missing",
            Self::Ambiguous => "library_disk_ambiguous",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::NotFound => "disco da biblioteca não encontrado",
            Self::Ambiguous => "mais de um disco tem uma biblioteca do My Photos",
        }
    }
}

/// What F11 needs from the machine; tests swap in a fake.
pub(crate) trait Volumes {
    /// Mounted drive roots, `C:\` style, in letter order.
    fn roots(&self) -> Vec<String>;
    fn has_library_db(&self, root: &str) -> bool;
}

pub(crate) struct RealVolumes;

impl Volumes for RealVolumes {
    fn roots(&self) -> Vec<String> {
        mounted_roots()
    }

    fn has_library_db(&self, root: &str) -> bool {
        Path::new(root).join(DB_RELATIVE).is_file()
    }
}

/// The root of the disk that holds the library: `E:\` when it has the database,
/// otherwise the only drive that does.
pub fn locate_library() -> Result<String, LocateError> {
    locate_with(&RealVolumes)
}

pub(crate) fn locate_with(volumes: &dyn Volumes) -> Result<String, LocateError> {
    let candidates: Vec<String> = volumes
        .roots()
        .into_iter()
        .filter(|root| volumes.has_library_db(root))
        .collect();
    if candidates
        .iter()
        .any(|root| root.eq_ignore_ascii_case(PREFERRED_ROOT))
    {
        return Ok(PREFERRED_ROOT.to_string());
    }
    match candidates.as_slice() {
        // First run: no database anywhere yet, so the library starts on `E:\` (AD-007).
        [] if volumes
            .roots()
            .iter()
            .any(|root| root.eq_ignore_ascii_case(PREFERRED_ROOT)) =>
        {
            Ok(PREFERRED_ROOT.to_string())
        }
        [] => Err(LocateError::NotFound),
        [only] => Ok(only.clone()),
        _ => Err(LocateError::Ambiguous),
    }
}

/// Before a run on `root`: if a library already belongs to this volume under another
/// drive letter, point it at `root` so its rows (relative paths) are reused instead of a
/// new library being created. Returns true when it relinked.
pub(crate) fn relink(
    connection: &Connection,
    root: &str,
    volume: Option<&VolumeInfo>,
) -> rusqlite::Result<bool> {
    let Some(volume) = volume else {
        return Ok(false);
    };
    let found: Option<(i64, String)> = connection
        .query_row(
            "SELECT id, root_path FROM libraries WHERE volume_id = ?1 ORDER BY id LIMIT 1",
            [&volume.serial],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match found {
        Some((id, path)) if !path.eq_ignore_ascii_case(root) => {
            // Another library row may already use this root (a stray run before F11);
            // the volume's own row wins and keeps its media.
            let taken: bool = connection.query_row(
                "SELECT EXISTS (SELECT 1 FROM libraries WHERE root_path = ?1 AND id != ?2)",
                params![root, id],
                |row| row.get(0),
            )?;
            if taken {
                return Ok(false);
            }
            connection.execute(
                "UPDATE libraries SET root_path = ?2 WHERE id = ?1",
                params![id, root],
            )?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// After the run has its library row: records which volume it lives on.
pub(crate) fn record_volume(
    connection: &Connection,
    library_id: i64,
    volume: Option<&VolumeInfo>,
) -> rusqlite::Result<()> {
    if let Some(volume) = volume {
        connection.execute(
            "UPDATE libraries SET volume_id = ?2, volume_label = ?3, status = 'active' WHERE id = ?1",
            params![library_id, volume.serial, volume.label],
        )?;
    }
    Ok(())
}

/// Serial and label of the volume that holds `root` (`E:\`). `None` off Windows or
/// when Windows cannot read it.
pub fn volume_info(root: &Path) -> Option<VolumeInfo> {
    #[cfg(windows)]
    {
        volume_info_windows(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        None
    }
}

/// `1A2B3C4D` → `1A2B-3C4D`, the spelling `dir` and `vol` print.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn format_serial(serial: u32) -> String {
    format!("{:04X}-{:04X}", serial >> 16, serial & 0xFFFF)
}

/// Bit 0 = A: … bit 25 = Z: → `["C:\\", "E:\\"]`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn roots_from_mask(mask: u32) -> Vec<String> {
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| format!("{}:\\", (b'A' + bit) as char))
        .collect()
}

fn mounted_roots() -> Vec<String> {
    #[cfg(windows)]
    {
        // SAFETY: no arguments; returns a bit mask.
        let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
        roots_from_mask(mask)
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn volume_info_windows(root: &Path) -> Option<VolumeInfo> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;

    let letter = crate::bench::volume::volume_root_of(root);
    if letter.is_empty() {
        return None;
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(&letter)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut label = [0u16; 261];
    let mut serial = 0u32;
    // SAFETY: every buffer is valid for the length passed; optional outputs are null.
    let ok = unsafe {
        GetVolumeInformationW(
            wide.as_ptr(),
            label.as_mut_ptr(),
            label.len() as u32,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        return None;
    }
    let end = label.iter().position(|&unit| unit == 0).unwrap_or(label.len());
    let label = String::from_utf16_lossy(&label[..end]);
    Some(VolumeInfo {
        serial: format_serial(serial),
        label: (!label.is_empty()).then_some(label),
    })
}
