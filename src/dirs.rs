use std::io;
use std::fs;
use std::env;
use std::path::{Path,PathBuf};
   pub fn create_dir(path_str: &str) -> Result<String, io::Error> {
        let path = Path::new(path_str);
        fs::create_dir(&path)?;
        path.to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to convert path to string"))
            .map(|s| s.to_owned())
    }
    pub fn get_dirs(path : &std::path::PathBuf)-> Option <fs::ReadDir> {
        if let Ok(entries) = fs::read_dir(path) {
            Some(entries)
        }
        else {
            None
        }
    }
    pub fn exit_to_dir (path : &std::path::PathBuf){
            if let Err(err) = env::set_current_dir(path) {
            eprintln!("Error: {}", err);
        } else {
            println!("Changed to target_directory");
        }

    }
