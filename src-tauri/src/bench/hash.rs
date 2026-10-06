use std::io::{self, Read};

use sha2::{Digest, Sha256};

const CHUNK: usize = 65_536;

// Sector-aligned so the same reader works with FILE_FLAG_NO_BUFFERING.
#[repr(align(4096))]
struct Aligned([u8; CHUNK]);

pub(crate) fn hash_reader(mut reader: impl Read) -> io::Result<[u8; 32]> {
    let mut hasher = Sha256::new();
    let mut aligned = Aligned([0u8; CHUNK]);
    let buf = &mut aligned.0;
    loop {
        let read = reader.read(&mut buf[..])?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hasher.finalize().into())
}

pub(crate) fn content_digest(digests_in_order: &[Option<[u8; 32]>]) -> String {
    let mut hasher = Sha256::new();
    let mut hashed = false;
    for digest in digests_in_order {
        if let Some(bytes) = digest {
            hasher.update(bytes);
            hashed = true;
        }
    }
    if !hashed {
        return String::new();
    }
    to_hex(&hasher.finalize())
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read};

    use super::super::corpus::file_bytes;
    use super::{content_digest, hash_reader};

    #[test]
    fn known_file_digests_match_the_spec() {
        assert_eq!(
            hex_of(hash_reader(Limited::new(file_bytes(0))).unwrap()),
            "d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca"
        );
        assert_eq!(
            hex_of(hash_reader(Cursor::new(file_bytes(1))).unwrap()),
            "29fa02d3381a0b093ff2d6d783c1c78176fd1aafce643c8d205631dc85df5e95"
        );
        assert_eq!(
            hex_of(hash_reader(Cursor::new(file_bytes(63))).unwrap()),
            "b21fe85ea1fba0db5690a657db55547f0cf8cb4a9ea81ac332e67de8de84be06"
        );
    }

    #[test]
    fn content_digest_of_indexes_0_through_63_matches_the_spec() {
        let digests: Vec<Option<[u8; 32]>> = (0..64)
            .map(|index| Some(hash_reader(Cursor::new(file_bytes(index))).unwrap()))
            .collect();
        assert_eq!(
            content_digest(&digests),
            "c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415"
        );
    }

    #[test]
    fn content_digest_is_empty_when_nothing_succeeds() {
        assert_eq!(content_digest(&[]), "");
        assert_eq!(content_digest(&[None, None]), "");
    }

    #[test]
    fn chunked_reader_smaller_than_4096_still_matches_index_0() {
        let bytes = file_bytes(0);
        let digest = hash_reader(Chunked {
            data: &bytes,
            piece: 3,
        })
        .unwrap();
        assert_eq!(
            hex_of(digest),
            "d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca"
        );
    }

    #[test]
    fn reader_error_is_returned() {
        let error = hash_reader(Failing).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    fn hex_of(bytes: [u8; 32]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    struct Limited {
        data: Cursor<[u8; 4096]>,
    }

    impl Limited {
        fn new(bytes: [u8; 4096]) -> Self {
            Self {
                data: Cursor::new(bytes),
            }
        }
    }

    impl Read for Limited {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            assert!(buf.len() <= 65_536);
            self.data.read(buf)
        }
    }

    struct Chunked<'a> {
        data: &'a [u8],
        piece: usize,
    }

    impl Read for Chunked<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.data.is_empty() {
                return Ok(0);
            }
            let size = self.piece.min(buf.len()).min(self.data.len());
            buf[..size].copy_from_slice(&self.data[..size]);
            self.data = &self.data[size..];
            Ok(size)
        }
    }

    struct Failing;

    impl Read for Failing {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("unreadable"))
        }
    }
}
