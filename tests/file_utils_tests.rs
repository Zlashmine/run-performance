#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    use activity_api::file_utils::get_rows_in_file;

    fn create_test_file(contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("test_{}.csv", uuid::Uuid::new_v4()));
        let mut file = File::create(&path).expect("Failed to create test file");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to test file");
        path
    }

    #[test]
    fn test_get_rows_in_file_reads_all_lines() {
        let contents = "line1\nline2\nline3";
        let path = create_test_file(contents);
        let rows = get_rows_in_file(path.to_str().unwrap().to_string()).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], "line1");
        assert_eq!(rows[2], "line3");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_get_rows_in_file_nonexistent_file() {
        let result = get_rows_in_file("nonexistent_file.csv".to_string());
        assert!(result.is_err());
    }
}
