use std::fs;
use std::io;
use std::path::Path;

pub fn read_text(path: &Path) -> io::Result<String> {
    let text = fs::read_to_string(path)?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_text() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        fs::write(path, "\r,\n,\r\n").unwrap();

        let output = read_text(path).unwrap();
        assert_eq!(output, "\n,\n,\n");
    }
}
