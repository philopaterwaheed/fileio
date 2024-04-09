mod Dirs;
mod Paths;
mod Files;

use Dirs::dirs;
use Files::files;
use Paths::paths;
use ncurses::* ; 
use std::io::{self, Write};
use std::fs;
use std::path::{Path, PathBuf};
fn main() {
    // Start in the current directory
    let mut current_dir = std::env::current_dir().expect("Failed to get current directory"); // get the dir

    // let x = dirs::Directory::new("/home/philosan/Dev/rust/fileio/src/main.rs");
    let y = dirs::Directory::new("/home/philosan/").unwrap();
    let z = files::File::new("main.rs" , paths::str_to_path("/home/philosan/Dev/rust/fileio/src/main.rs")).unwrap();
    copy_file(&z , &y);

    // let dirs = dirs::get_dirs(& current_dir).unwrap();
    // for e in dirs{
    //
    //     if let Ok(e) = e {
    //         println!("{}", e.file_name().to_string_lossy());
    //     }
    // }
    // loop {
    //     print_files(&current_dir);
    //
    //     print!(">> ");
    //     io::stdout().flush().unwrap();
    //     
    //     let mut input = String::new();
    //     io::stdin().read_line(&mut input).expect("Failed to read line");
    //
    //     let input = input.trim();
    //
    //     if input == "quit" {
    //         break;
    //     } else if input == "cd .." {
    //         current_dir.pop();
    //     } else {
    //         let new_dir = current_dir.join(input);
    //         if new_dir.is_dir() {
    //             current_dir = new_dir;
    //         } else {
    //             println!("Not a valid directory or command");
    //         }
    //     }
    // }
}

fn print_files(path: &std::path::PathBuf) { // edit we will just display it on ncurses
}
fn copy_file (file : &files::File ,  dircetion_dir : &dirs::Directory )-> Result<files::File, io::Error>
{
    let new_name = file.name.clone () ; 
    fs::copy(file.path.as_path(), dircetion_dir.path.join(new_name.as_str()).as_path())?;
    let out_file =  files::File::new(new_name.as_str(),dircetion_dir.path.join(new_name.as_str()).as_path());
    out_file
}
