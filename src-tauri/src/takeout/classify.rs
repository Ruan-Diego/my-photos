const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "dng", "cr2", "nef", "arw", "mp4", "mov",
    "mkv", "avi", "3gp", "webm", "m4v",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRole {
    Media { extension: String },
    AlbumMetadata,
    JsonSidecar,
    Ignored,
}

pub fn classify(file_name: &str) -> FileRole {
    if file_name.eq_ignore_ascii_case("archive_browser.html") {
        return FileRole::Ignored;
    }
    if file_name.eq_ignore_ascii_case("metadata.json") {
        return FileRole::AlbumMetadata;
    }
    if let Some((_, extension)) = file_name.rsplit_once('.') {
        let extension = extension.to_ascii_lowercase();
        if MEDIA_EXTENSIONS.contains(&extension.as_str()) {
            return FileRole::Media { extension };
        }
    }
    if has_json_suffix(file_name) {
        return FileRole::JsonSidecar;
    }
    FileRole::Ignored
}

fn has_json_suffix(file_name: &str) -> bool {
    file_name.len() >= 5 && file_name[file_name.len() - 5..].eq_ignore_ascii_case(".json")
}

#[cfg(test)]
mod tests {
    use super::{classify, FileRole};

    #[test]
    fn known_image_is_media_and_every_listed_extension_is_media() {
        assert_eq!(
            classify("IMG_20220810_143700.JPG"),
            FileRole::Media {
                extension: "jpg".to_string()
            }
        );
        for extension in [
            "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "dng", "cr2", "nef", "arw", "mp4",
            "mov", "mkv", "avi", "3gp", "webm", "m4v",
        ] {
            assert_eq!(
                classify(&format!("foto.{extension}")),
                FileRole::Media {
                    extension: extension.to_string()
                }
            );
        }
    }

    #[test]
    fn known_video_is_media() {
        assert_eq!(
            classify("clip.mp4"),
            FileRole::Media {
                extension: "mp4".to_string()
            }
        );
    }

    #[test]
    fn album_metadata_name_ignores_ascii_case() {
        assert_eq!(classify("MeTaDaTa.JsOn"), FileRole::AlbumMetadata);
    }

    #[test]
    fn media_sidecar_json_is_not_media() {
        assert_eq!(classify("foto.jpg.json"), FileRole::JsonSidecar);
    }

    #[test]
    fn archive_browser_html_is_ignored() {
        assert_eq!(classify("archive_browser.html"), FileRole::Ignored);
        assert_eq!(classify("Archive_Browser.HTML"), FileRole::Ignored);
    }

    #[test]
    fn plain_text_is_ignored() {
        assert_eq!(classify("notas.txt"), FileRole::Ignored);
    }
}
