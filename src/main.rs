mod Dirs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;
mod Files;

use Dirs::dirs;
use Files::files;
use ncurses::* ; 
use std::io::{self, Write};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
fn main() {
    // Start in the current directory
    let mut current_dir = std::env::current_dir().expect("Failed to get current directory"); // get the dir

    // let x = dirs::Directory::new("/home/philosan/Dev/rust/fileio/src/main.rs");
    let y = dirs::Directory::new("philosan",Path::new("/home/philosan/")).unwrap();
    let mut z = dirs::Directory::new("c#" , Path::new("/home/philosan/Dev/c#/")).unwrap();
    let mut x = files::File::new("hello" , Path::new("/home/philosan/Dev/c#/hello")).unwrap();
    move_dir(&z,&y);
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
fn copy_file (file : files::File ,  dircetion_dir : &dirs::Directory )-> Result<files::File, io::Error>
{
    let new_name = file.name.clone () ; 
    fs::copy(file.path.as_path(), dircetion_dir.path.join(new_name.as_str()).as_path())?;
    let out_file =  files::File::new(new_name.as_str(),dircetion_dir.path.join(new_name.as_str()).as_path());
    out_file
}
fn move_file (file : files::File ,  dircetion_dir : &dirs::Directory )-> Result<files::File, io::Error>
{
    let new_name = file.name.clone () ; 
    fs::rename(file.path.as_path(), dircetion_dir.path.join(new_name.as_str()).as_path())?;
    let out_file =  files::File::new(new_name.as_str(),dircetion_dir.path.join(new_name.as_str()).as_path());
    out_file
}

fn copy_dir (source_dir : &dirs::Directory ,  dircetion_dir : &dirs::Directory )-> Result<dirs::Directory, io::Error>
{
    let new_name = source_dir.name.clone ();
    let new_path = &dircetion_dir.path.join(&new_name);
    let dir = dirs::Directory::new(new_name.as_str(), new_path);


    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
            println!("{}",entry.file_name().into_string().ok().unwrap());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            copy_dir(&dirs::Directory::new(entry_name.to_owned().as_str(),entry.path().as_path()).unwrap(),&dirs::Directory::new(entry_name.to_owned().as_str(),new_path.as_path()).unwrap())?;
        } else {
            fs::copy(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    dir
}

fn move_dir (source_dir : &dirs::Directory ,  dircetion_dir : &dirs::Directory )-> Result<dirs::Directory, io::Error>
{
    let new_name = source_dir.name.clone ();
    let new_path = &dircetion_dir.path.join(&new_name);
    let dir = dirs::Directory::new(new_name.as_str(), new_path);


    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
            println!("{}",entry.file_name().into_string().ok().unwrap());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            move_dir(&dirs::Directory::new(entry_name.to_owned().as_str(),entry.path().as_path()).unwrap(),&dirs::Directory::new(entry_name.to_owned().as_str(),new_path.as_path()).unwrap())?;
        } else {
            fs::rename(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    dir
}

