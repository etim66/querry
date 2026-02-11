use std::{
    error::Error,
    fs::{self, File},
    path::PathBuf,
};

use directories::ProjectDirs;

/// Get the database path based on whether it's a test or not.
pub fn get_db_path(is_test: Option<bool>) -> Result<String, Box<dyn Error>> {
    let is_test = is_test.unwrap_or(false);
    let file_path: PathBuf = if is_test {
        // If it's a test, use a temporary directory with a unique filename
        let filename = format!("querry_test_{}.db", uuid::Uuid::new_v4());
        std::env::temp_dir().join(filename)
    } else {
        // For non-test cases, use the user data directory
        let project_dirs = ProjectDirs::from("org", "etim", "querry")
            .ok_or("Unable to get project directories")?;
        project_dirs.data_dir().join("querry.db")
    };

    // Ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create the file if it doesn't exist
    if !file_path.exists() {
        File::create(&file_path)?; // This creates an empty file
    }

    let path_str = file_path
        .to_str()
        .ok_or("Invalid Unicode in path")?
        .to_string();
    Ok(path_str)
}

#[cfg(test)]
mod tests {
    use super::get_db_path;
    use std::path::PathBuf;

    #[test]
    fn test_get_db_path_for_tests_creates_file() {
        let path = get_db_path(Some(true)).expect("Expected test DB path");
        let path_buf = PathBuf::from(&path);

        assert!(path_buf.exists());
        assert!(path_buf.is_file());
    }

    #[test]
    fn test_get_db_path_for_tests_uses_temp_dir() {
        let path = get_db_path(Some(true)).expect("Expected test DB path");
        let path_buf = PathBuf::from(&path);

        assert!(path_buf.starts_with(std::env::temp_dir()));
    }
}
