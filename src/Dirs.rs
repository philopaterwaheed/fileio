pub mod dirs {
    use std::fs;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::{env, process::Command};

    use crossterm::terminal::disable_raw_mode;
    #[derive(Debug)]
    pub struct Directory {
        pub path: PathBuf,
        pub name: String,
        pub contains_count: usize,
    }

    impl Directory {
        pub fn new(path: &Path) -> Result<Directory, io::Error> {
            if path == Path::new("/") {
                return Ok(Directory {
                    path: path.to_owned(),
                    name: "/".to_owned(),
                    contains_count: count_contians(&path.to_owned()).unwrap(),
                });
            }
            if !path.exists() {
                fs::create_dir_all(&path)?;
            }
            Ok(Directory {
                path: path.to_owned(),
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                contains_count: count_contians(&path.to_owned())?,
            })
        }
        pub fn get_contains(&self) -> Option<fs::ReadDir> {
            if let Ok(entries) = fs::read_dir(self.path.as_path()) {
                Some(entries)
            } else {
                None
            }
        }
        pub fn remove(&self) -> io::Result<()> {
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
        pub fn up(&mut self) -> io::Result<()> {
            let temp = self.path.clone();
            let new_path = temp.parent();
            match new_path {
                Some(x) => {
                    let temp = Directory::new(x).unwrap();
                    self.name = temp.name;
                    self.path = temp.path;
                    self.contains_count = temp.contains_count;
                }
                None => (),
            }
            Ok(())
        }
        pub fn down(&mut self, index: usize) -> io::Result<()> {
            let dir = &self.vec_of_contains().unwrap().0[index];
            if dir.is_dir() {
                let temp = Directory::new(dir).unwrap();
                self.name = temp.name;
                self.path = temp.path;
                self.contains_count = temp.contains_count;
            }
            Ok(())
        }
        pub fn prev(&self) -> Result<Directory, io::Error> {
            let temp = self.path.clone();
            let new_path = temp.parent();
            match new_path {
                Some(x) => Directory::new(x),
                None => {
                    let error_message = format!("No parentt directory for {}", self.path.display());
                    Err(io::Error::new(io::ErrorKind::NotFound, error_message))
                }
            }
        }
        pub fn find_index(&self) -> usize {
            // find indes of self in parent
            if let Ok(parent) = self.prev() {
                let index_in_parent = parent
                    .vec_of_contains()
                    .unwrap()
                    .1
                    .iter()
                    .position(|s| s.to_owned() == self.name.as_str());
                return index_in_parent.unwrap();
            }
            0
        }
        pub fn vec_of_contains(&self) -> Result<(Vec<PathBuf>, Vec<String>), io::Error> {
            let entries = self.get_contains();

            let (paths, names): (Vec<_>, Vec<_>) = entries
                .unwrap()
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        let path = e.path();
                        let name = path.file_name()?.to_string_lossy().to_string();
                        Some((path, name))
                    })
                })
                .unzip();

            // returns paths and strings
            Ok((paths, names))
        }
        pub fn get_env_dir() -> Result<Directory, io::Error> {
            let current_dir = std::env::current_dir()?;
            Directory::new(current_dir.as_path())
        }
        pub fn start_shell_in_dir(&self) -> io::Result<()> {
            let mut cmd = Command::new("sh"); // Use "cmd" for Windows

            disable_raw_mode()?;
            // Set the working directory to the specified directory
            cmd.arg("-c").arg(format!(
                "clear;cd \"{}\" && exec $SHELL",
                self.path.to_owned().into_os_string().to_str().unwrap()
            )); // Use "/D" for Windows
                // Execute the command
            let status = cmd.status()?;

            if !status.success() {
                return Err(io::Error::new(io::ErrorKind::Other, "Command failed"));
            }

            Ok(())
        }
    }
    pub fn change_env_dir(dir: Directory) {
        if let Err(err) = env::set_current_dir(dir.path) {
            eprintln!("Error: {}", err);
        } else {
            println!("Changed to target_directory");
        }
    }
    pub fn count_contians(path: &PathBuf) -> Result<usize, io::Error> {
        let entries = fs::read_dir(path)?;
        let file_count = entries
            .filter_map(Result::ok) // Filter out Err values and unwrap Ok values
            .count();
        Ok(file_count)
    }
}
