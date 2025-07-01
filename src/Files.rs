pub mod files {
    use std::fs;
    use std::io::{self, BufRead, BufReader};
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    #[derive(Debug, Clone)]
    pub struct File {
        pub path: PathBuf,
        pub name: String,
    }
    impl File {
        pub fn File(path:&Path){}
        pub fn new(path_str: &Path) -> Result<File, io::Error> {
            let path = Path::new(path_str);
            if path.exists() {
                return Ok(File {
                    name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                    path: path.to_owned(),
                });
            }
            let _f = fs::File::create(&path)?;
            Ok(File {
                name: path.file_name().unwrap().to_str().unwrap().to_owned(),
                path: path.to_owned(),
            })
        }
        pub fn perm_ch(&self, permissions: u32) -> io::Result<()> {
            let new_permissions = fs::Permissions::from_mode(permissions);
            fs::set_permissions(&self.path, new_permissions)?;
            Ok(())
        }
        pub fn remove(&self) -> io::Result<()> {
            fs::remove_file(self.path.as_path())?;
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
        pub fn read(&self) -> io::Result<Vec<String>> {
            let mut contents : Vec<String> = vec![];
            let file = std::fs::File::open(self.path.as_path())?;
            let reader = BufReader::new(file);
            for (index, line) in reader.lines().enumerate() {
                if index >= 100 {
                    break;
                }
                let line = line?;
                contents.push(line);
            }
            Ok(contents)
        }
    }
}
