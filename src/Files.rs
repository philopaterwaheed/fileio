pub mod files {
    use std::fs;
    use std::io::{self, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    #[derive(Debug)]
    pub struct File {
        pub path: PathBuf,
        pub name: String,
    }
    impl File {
        pub fn new(name: &str, path_str: &Path) -> Result<File, io::Error> {
            let path = Path::new(path_str);
            if path.exists() {
                return Ok(File {
                    name: name.to_owned(),
                    path: path.to_owned(),
                });
            }
            let _f = fs::File::create(&path)?;
            Ok(File {
                name: name.to_owned(),
                path: path.to_owned(),
            })
        }
        pub fn perm_ch(&self, permissions: u32) -> io::Result<()> {
            let new_permissions = fs::Permissions::from_mode(permissions);
            fs::set_permissions(&self.path, new_permissions)?;
            Ok(())
        }
        pub fn remove(self) -> io::Result<()> {
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
    }
}
