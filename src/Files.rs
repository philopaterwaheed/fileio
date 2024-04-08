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
        pub fn create_file(name : &str , path_str: &str) ->  Result<File, io::Error> {
            let path = Path::new(path_str);
            let _f = fs::File::create(&path)?;
            
            Ok(File{
                path : path.to_owned(),
                name : name.to_owned(),
            })
        }
    }
}
