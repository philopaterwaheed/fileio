pub mod dirs {
    use std::io;
    use std::fs;
    use std::{env,process::Command};
    use std::path::{Path,PathBuf};
    use std::os::unix::fs::PermissionsExt;
    #[derive(Debug)]
    pub struct Directory {
       pub path: PathBuf,
       pub name: String,
    }
    
    impl Directory {
        pub fn new(name : &str , path_str: &Path) -> Result<Directory, io::Error> {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                fs::create_dir_all(&path)?;
                
            }
            Ok(Directory { path ,name : name.to_owned()})
        }        
        pub fn get_contains(&self)-> Option <fs::ReadDir> {
            if let Ok(entries) = fs::read_dir(self.path.as_path()) {
                Some(entries)
            }
            else {
                None
            }
        }  
        pub fn remove(self) -> io::Result<()> {
            fs::remove_dir_all(self.path.as_path())?;
            Ok(())
        }
        pub fn perm_ch(&self, permissions: u32) -> io::Result<()> {
            let new_permissions = fs::Permissions::from_mode(permissions);
            fs::set_permissions(&self.path, new_permissions)?;
            Ok(())
        }
        pub fn rename(&mut self, str: &str) -> io::Result<()> {
            let new_path = self.path.parent().unwrap().join(str);
            fs::rename(self.path.as_path(), new_path.clone())?;
            // self = &mut File::new(str, new_path.as_path()).unwrap() ;
            //
            self.name = str.to_owned();
            self.path = new_path.as_path().to_owned();
            Ok(())
        }
    }
    pub fn change_to_dir (dir : Directory){
            if let Err(err) = env::set_current_dir(dir.path) {
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
