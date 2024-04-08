mod Dirs;
mod Files;

use Dirs::dirs;
use Files::files;
use ncurses::* ; 
use std::io::{self, Write};
use std::fs;
use std::path::{Path, PathBuf};
fn main() {
    // Start in the current directory
    let mut current_dir = std::env::current_dir().expect("Failed to get current directory"); // get the dir

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
fn copy_file (file : files::File ,  dircetion_dir : dirs::Directory )-> Result<files::File, io::Error>
{
    fs::copy(file.path, dircetion_dir.path)?;
    files::File.create_file(file.name, )

}
