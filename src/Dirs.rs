pub mod dirs {
use std::io;
use std::fs;
use std::{env,process::Command};
use std::path::{Path,PathBuf};
    pub fn str_to_path (str : &str)-> &Path {
        let path = Path::new(str);
        path
    }
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
    pub fn change_to_dir (path : &Path){
            if let Err(err) = env::set_current_dir(path) {
            eprintln!("Error: {}", err);
        } else {
            println!("Changed to target_directory");
        }

    }
    pub fn start_shell_in_dir(dir: &str) -> io::Result<()> {
        let mut cmd = Command::new("sh"); // Use "cmd" for Windows

        // Set the working directory to the specified directory
        cmd.arg("-c").arg(format!("cd \"{}\" && exec $SHELL", dir)); // Use "/D" for Windows

        // Execute the command
        let status = cmd.status()?;

        if !status.success() {
            eprintln!("Failed to start shell in {}", dir);
            return Err(io::Error::new(io::ErrorKind::Other, "Command failed"));
        }

        Ok(())
    }
}
