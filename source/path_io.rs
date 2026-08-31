use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 1_000_000;

pub(crate) fn read_nul_paths(reader: impl Read) -> io::Result<Vec<PathBuf>> {
    let mut reader = reader;
    let mut buffer = [0_u8; 8192];
    let mut record = Vec::new();
    let mut paths = Vec::new();
    let mut total = 0_usize;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            total = total.saturating_add(1);
            if total > MAX_TOTAL_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "NUL path input exceeds the 64 MiB limit",
                ));
            }
            if *byte == 0 {
                if record.is_empty() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "NUL path input contains an empty record",
                    ));
                }
                if paths.len() == MAX_RECORDS {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "NUL path input exceeds 1000000 records",
                    ));
                }
                paths.push(PathBuf::from(os_string_from_bytes(std::mem::take(
                    &mut record,
                ))?));
            } else {
                if record.len() == MAX_RECORD_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "NUL path record exceeds the 1 MiB limit",
                    ));
                }
                record.push(*byte);
            }
        }
    }
    if !record.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "NUL path input ends without a NUL delimiter",
        ));
    }
    Ok(paths)
}

pub(crate) fn write_paths(
    writer: &mut impl Write,
    paths: &[PathBuf],
    nul_delimited: bool,
) -> io::Result<()> {
    let delimiter = if nul_delimited { 0 } else { b'\n' };
    for path in paths {
        writer.write_all(path_bytes(path))?;
        writer.write_all(&[delimiter])?;
    }
    writer.flush()
}

#[cfg(unix)]
fn os_string_from_bytes(bytes: Vec<u8>) -> io::Result<OsString> {
    use std::os::unix::ffi::OsStringExt;
    Ok(OsString::from_vec(bytes))
}

#[cfg(not(unix))]
fn os_string_from_bytes(bytes: Vec<u8>) -> io::Result<OsString> {
    String::from_utf8(bytes)
        .map(OsString::from)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "path is not valid UTF-8"))
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> &[u8] {
    path.as_os_str().as_encoded_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_complete_nul_records_and_rejects_malformed_input() {
        assert_eq!(
            read_nul_paths(&b"one.jpg\0nested/two.png\0"[..]).unwrap(),
            vec![PathBuf::from("one.jpg"), PathBuf::from("nested/two.png")]
        );
        assert!(read_nul_paths(&b"unterminated"[..]).is_err());
        assert!(read_nul_paths(&b"one\0\0"[..]).is_err());
        assert!(
            read_nul_paths(
                std::iter::repeat_n(b'x', MAX_RECORD_BYTES + 1)
                    .collect::<Vec<_>>()
                    .as_slice()
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn nul_output_preserves_non_utf8_and_embedded_newline_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let paths = vec![PathBuf::from(OsString::from_vec(vec![
            b'/', b't', b'm', b'p', b'/', 0xff, b'\n', b'x',
        ]))];
        let mut output = Vec::new();
        write_paths(&mut output, &paths, true).unwrap();
        assert_eq!(output, b"/tmp/\xff\nx\0");
    }
}
