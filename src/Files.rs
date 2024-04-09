pub mod files {
    use std::io::{self, Write};
    use std::fs;
    use std::path::{Path, PathBuf};
    #[derive(Debug)]
    pub struct File {
        pub path: PathBuf,
        pub name: String,
    }
    impl File {
        pub fn new(name : &str , path_str: &Path) ->  Result<File, io::Error> {
            let path = Path::new(path_str);
        if path.exists() {
          return  Ok(File{
                name : name.to_owned(),
                path : path.to_owned(),
            })
        } 
            let _f = fs::File::create(&path)?;
            Ok(File{
                name : name.to_owned(),
                path : path.to_owned(),
            })
        }
    }
}
