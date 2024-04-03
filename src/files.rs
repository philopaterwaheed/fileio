use std::io::{self, Write};
use std::fs;
use std::path::{Path, PathBuf};
    pub fn create_file(path_str: &str) -> Result<String, io::Error> {
        let path = Path::new(path_str);
        fs::File::create(&path)?;
        
        // Convert the path to a String and return it
        // This involves handling potential failure in conversion, thus the map_err
        path.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to convert path to string"))
            .map(|s| s.to_owned())
    }
