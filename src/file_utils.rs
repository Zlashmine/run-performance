use std::{fs, io, path::Path};

pub fn get_rows_in_file(file: String) -> Result<Vec<String>, io::Error> {
    let path = Path::new(&file);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File does not exist: {}", file),
        ));
    }

    let content = fs::read_to_string(path)?;

    Ok(content.lines().map(String::from).collect())
}
