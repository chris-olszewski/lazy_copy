use std::{
    fs::OpenOptions,
    io::{self, Read, Seek, Write},
    path::Path,
};

const BUFFER_SIZE: usize = 8 * 1024;

/// Copy contents of source to dest, only writing to dest once differing bytes are encountered.
///
/// Will result in source and est having identical bytes while trying to avoid unnecessary writes.
/// If differing bytes are encountered, then `io::copy` will be used to write the remaining bytes to dest.
pub fn copy<R: io::Read, P: AsRef<Path>>(mut source: R, dest: P) -> io::Result<u64> {
    let mut dest = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(dest)?;
    let mut source_buffer = [0; BUFFER_SIZE];
    let mut dest_buffer = [0; BUFFER_SIZE];
    let mut bytes_copied = 0;
    loop {
        let source_bytes_read = source.read(&mut source_buffer)?;
        if source_bytes_read == 0 {
            break;
        }
        let dest_bytes_read = dest.read(&mut dest_buffer)?;
        match dest_bytes_read.cmp(&source_bytes_read) {
            std::cmp::Ordering::Equal
                if source_buffer[..source_bytes_read] == dest_buffer[..dest_bytes_read] =>
            {
                bytes_copied += source_bytes_read as u64;
            }
            // Content differs
            std::cmp::Ordering::Equal => {
                // Move backwards and write the latest read
                dest.seek(io::SeekFrom::Current(-(dest_bytes_read as i64)))?;
                dest.write_all(&source_buffer[..source_bytes_read])?;
                bytes_copied += source_bytes_read as u64;
                // Use `io::copy` to write rest of bytes to file
                let copied = io::copy(&mut source, &mut dest)?;
                bytes_copied += copied;
                break;
            }
            // dest has more bytes than source
            std::cmp::Ordering::Greater => {
                // Move backward and write the latest read
                dest.seek(io::SeekFrom::Current(-(dest_bytes_read as i64)))?;
                dest.write_all(&source_buffer[..source_bytes_read])?;
                bytes_copied += source_bytes_read as u64;
                break;
            }
            // source has more bytes than dest
            std::cmp::Ordering::Less => {
                // Move backward and write the latest read
                dest.seek(io::SeekFrom::Current(-(dest_bytes_read as i64)))?;
                dest.write_all(&source_buffer[..source_bytes_read])?;
                bytes_copied += source_bytes_read as u64;
                // Use `io::copy` to write rest of bytes to file
                let copied = io::copy(&mut source, &mut dest)?;
                bytes_copied += copied;
                break;
            }
        }
    }

    // Possibly truncate dest to be the same size as source
    dest.set_len(bytes_copied)?;

    Ok(bytes_copied)
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use super::*;

    use tempdir::TempDir;

    #[test]
    fn full_match() -> io::Result<()> {
        let tmp_dir = TempDir::new("match")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = b"foo\nbar\n";
        File::create(&output)?.write_all(wanted)?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn partial_match() -> io::Result<()> {
        let tmp_dir = TempDir::new("match")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = b"foo\nbar\n";
        File::create(&output)?.write_all(b"foo\nbaz\n")?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn no_match() -> io::Result<()> {
        let tmp_dir = TempDir::new("match")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = b"foo\nbar\n";
        File::create(&output)?.write_all(b"yes\nope\n")?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn empty_file() -> io::Result<()> {
        let tmp_dir = TempDir::new("match")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = b"foo\nbar\n";
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn actual_needs_trim() -> io::Result<()> {
        let tmp_dir = TempDir::new("match")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = b"foo\nbar\n";
        File::create(&output)?.write_all(b"foo\nbar\nbaz\n")?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn large_match() -> io::Result<()> {
        let tmp_dir = TempDir::new("large")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = include_bytes!("../fixtures/random_50mb.bin");
        File::create(&output)?.write_all(&wanted[..])?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn large_partial_match() -> io::Result<()> {
        let tmp_dir = TempDir::new("large-partial")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = include_bytes!("../fixtures/test1.bin");
        let actual_start = include_bytes!("../fixtures/test2.bin");
        File::create(&output)?.write_all(&actual_start[..])?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }

    #[test]
    fn large_complete_mismatch() -> io::Result<()> {
        let tmp_dir = TempDir::new("mismatch")?;
        let output = tmp_dir.path().join("bar.txt");
        let wanted = include_bytes!("../fixtures/end1.bin");
        let actual_start = include_bytes!("../fixtures/end2.bin");
        File::create(&output)?.write_all(&actual_start[..])?;
        let bytes_copied = copy(&wanted[..], &output)?;
        assert_eq!(bytes_copied, wanted.len() as u64);
        let on_disk = std::fs::read(&output)?;
        assert_eq!(&wanted[..], &on_disk);
        Ok(())
    }
}
