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

#[cfg(test)]
mod tests {
    use super::{file_bytes, relative_path};

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
}
