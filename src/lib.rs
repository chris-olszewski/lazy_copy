use std::{
    fs::OpenOptions,
    io::{self, Read, Seek, Write},
    path::Path,
};

const BUFFER_SIZE: usize = 8 * 1024;

/// Copy contents of wanted into actual.
pub fn copy<R: io::Read, P: AsRef<Path>>(mut wanted: R, actual: P) -> io::Result<u64> {
    let mut actual = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(actual)?;
    let mut wanted_buffer = [0; BUFFER_SIZE];
    let mut actual_buffer = [0; BUFFER_SIZE];
    let mut bytes_copied = 0;
    loop {
        let wanted_read = wanted.read(&mut wanted_buffer)?;
        if wanted_read == 0 {
            break;
        }
        let actual_read = actual.read(&mut actual_buffer)?;
        match actual_read.cmp(&wanted_read) {
            std::cmp::Ordering::Equal
                if wanted_buffer[..wanted_read] == actual_buffer[..actual_read] =>
            {
                bytes_copied += wanted_read as u64;
            }
            std::cmp::Ordering::Equal => {
                // We rewind so that we're back to before the write
                actual.seek(io::SeekFrom::Current(-(actual_read as i64)))?;
                actual.write_all(&wanted_buffer[..wanted_read])?;
                bytes_copied += wanted_read as u64;
                let copied = io::copy(&mut wanted, &mut actual)?;
                bytes_copied += copied;
                break;
            }
            std::cmp::Ordering::Greater => {
                // actual > wanted => trim actual down to desired size -> have to assume Ok(0) will be next wanted.read return
                actual.seek(io::SeekFrom::Current(-(actual_read as i64)))?;
                actual.write_all(&wanted_buffer[..wanted_read])?;
                bytes_copied += wanted_read as u64;
                break;
            }
            std::cmp::Ordering::Less => {
                // actual < wanted => actual is shorter than wanted, append in remaining bytes

                // move back cursor in file to before the read
                actual.seek(io::SeekFrom::Current(-(actual_read as i64)))?;
                actual.write_all(&wanted_buffer[..wanted_read])?;
                bytes_copied += wanted_read as u64;
                let copied = io::copy(&mut wanted, &mut actual)?;
                bytes_copied += copied;
                // trim shouldn't be needed so we return here
                return Ok(bytes_copied);
            }
        }
    }

    actual.set_len(bytes_copied)?;

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
