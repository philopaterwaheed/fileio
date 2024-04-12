mod Dirs;
mod Files;

use::ncurses::*;

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::io::{self};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use Dirs::dirs;
use Files::files;

fn main() -> io::Result<()>{

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui)?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
fn ui(frame: &mut Frame) {
 let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Ratio(30,100),
        ],
    )
    .split(frame.size());
    frame.render_widget(
        Block::new().borders(Borders::TOP).title("Fileio"),
        main_layout[0],
    );
    frame.render_widget(
        Block::new().borders(Borders::ALL).title("Commands"),
        main_layout[2],
    );

    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50),Constraint::Percentage(50)],
    )
    .split(main_layout[1]);
    frame.render_widget(
        Block::default().borders(Borders::ALL).title("Prev"),
        inner_layout[0],
    );
    frame.render_widget(
        Block::default().borders(Borders::ALL).title("Curr"),
        inner_layout[1],
    );
    frame.render_widget(
        Block::default().borders(Borders::ALL).title("Next"),
        inner_layout[2],
    );
}

fn print_files(path: &std::path::PathBuf) { // edit we will just display it on ncurses
}
fn copy_file(file: files::File, dircetion_dir: &dirs::Directory) -> Result<files::File, io::Error> {
    let new_name = file.name.clone();
    fs::copy(
        file.path.as_path(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    )?;
    let out_file = files::File::new(
        new_name.as_str(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    );
    out_file
}
fn move_file(file: files::File, dircetion_dir: &dirs::Directory) -> Result<files::File, io::Error> {
    let new_name = file.name.clone();
    fs::rename(
        file.path.as_path(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    )?;
    let out_file = files::File::new(
        new_name.as_str(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    );
    out_file
}

fn copy_dir(
    source_dir: &dirs::Directory,
    dircetion_dir: &dirs::Directory,
) -> Result<dirs::Directory, io::Error> {
    let new_name = source_dir.name.clone();
    let new_path = &dircetion_dir.path.join(&new_name);
    let dir = dirs::Directory::new(new_name.as_str(), new_path);

    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            copy_dir(
                &dirs::Directory::new(entry_name.to_owned().as_str(), entry.path().as_path())
                    .unwrap(),
                &dirs::Directory::new(entry_name.to_owned().as_str(), new_path.as_path()).unwrap(),
            )?;
        } else {
            fs::copy(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    dir
}

fn move_dir(source_dir: dirs::Directory, dircetion_dir: &dirs::Directory)-> Result<dirs::Directory, io::Error> {
    let new_name = source_dir.name.clone();
    let new_path = &dircetion_dir.path.join(&new_name);
    let dir = dirs::Directory::new(new_name.as_str(), new_path);

    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            move_dir(
                 dirs::Directory::new(entry_name.to_owned().as_str(), entry.path().as_path())
                    .unwrap(),
                &dirs::Directory::new(entry_name.to_owned().as_str(), new_path.as_path()).unwrap(),
            )?;
        } else {
            fs::rename(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    let _remove_pross = source_dir.remove();
    dir
}
