mod Dirs;
mod Files;

use::ncurses::*;

use crossterm::{
    event::{self, Event, KeyCode},
    cursor::Show,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::io::{self};
use std::path::{Path};
use Dirs::dirs;
use Files::files;
 
static mut CLEAR :bool = false ; 
fn main() -> io::Result<()>{

    let curr_dir : &mut dirs::Directory = &mut dirs::Directory::get_env_dir().unwrap();
    let prev_sel :usize = 0;
    let sel:usize  = 0 ; 

    let tup = &mut (prev_sel,curr_dir,sel);

    // let tup: &mut (usize , &mut dirs::Directory ,usize) = (prev_sel,curr_dir,sel);

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f|ui(f,tup))?;
        should_quit = handle_events(tup)?;
        unsafe{
            if CLEAR{
                enable_raw_mode();
                terminal.clear();
                CLEAR=false;
            }
        }
    }

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    std::io::stdout().execute(Show)?;
    Ok(())
}

fn handle_events(selections : &mut (usize , &mut dirs::Directory ,usize)) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Down ||key.code == KeyCode::Char('j'){
                if selections.2 + 1 < selections.1.contains_count{
                    selections.2 = selections.2 + 1 ;
                }
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Up ||key.code == KeyCode::Char('k'){
                if selections.2 > 0 {
                    selections.2 = selections.2 - 1 ;
                }
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Left ||key.code == KeyCode::Char('h'){
                let _ = selections.1.up();
                selections.2 = selections.0;
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Right ||key.code == KeyCode::Char('l'){
                if selections.1. contains_count != 0 {
                    let _ = selections.1.down(selections.2);
                    selections.0 = selections.2;
                    selections.2 = 0;
                }
                
            }
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter ||key.code == KeyCode::Char('S'){
                let _ = selections.1.start_shell_in_dir();
                unsafe{
                    CLEAR = true;
                }
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame, selections : &mut (usize , &mut dirs::Directory ,usize)) {
 let curr = &selections.1;
 let prev = curr.prev();
 selections.0 = curr.find_index();

 let (current_contains_pathes,current_contains_strings)= curr.vec_of_contains().unwrap();
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
    let mut sel = ListState::default();
    let mut next_sel = ListState::default();
    let mut prev_sel = ListState::default();
    sel.select(Some(selections.2)); // Select the first item initially
    prev_sel.select(Some(selections.0)); // Select the first item initially
    if let Ok(d) = prev  {
        let prev_c = d.vec_of_contains().unwrap().1;

        render_list(frame, inner_layout[0],prev_c , &mut prev_sel,"Prev");
    }
    render_list(frame, inner_layout[1],current_contains_strings , &mut sel,format!("Curr").as_str());
    if  !current_contains_pathes.len() == 0 {
        if current_contains_pathes[selections.2].is_dir(){
            let next_c = dirs::Directory::new(current_contains_pathes[selections.2].as_path()).unwrap().vec_of_contains().unwrap().1;
            render_list(frame, inner_layout[2],next_c , &mut next_sel,"next");

        }
    }
}

fn render_list (frame : &mut Frame ,rect : Rect ,contains : Vec<String>,state:&mut ListState, title : &str) { 
    frame.render_stateful_widget(
        List::new(contains).block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::new().blue())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom)
        .highlight_spacing(HighlightSpacing::Always),
        rect,state
     );
}
fn copy_file(file: files::File, dircetion_dir: &dirs::Directory) -> Result<files::File, io::Error> {
    let new_name = file.name.clone();
    fs::copy(
        file.path.as_path(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    )?;
    let out_file = files::File::new(
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
    let dir = dirs::Directory::new( new_path);

    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            copy_dir(
                &dirs::Directory::new(entry.path().as_path())
                    .unwrap(),
                &dirs::Directory::new(new_path.as_path()).unwrap(),
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
    let dir = dirs::Directory::new(new_path);

    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            move_dir(
                 dirs::Directory::new(entry.path().as_path())
                    .unwrap(),
                &dirs::Directory::new(new_path.as_path()).unwrap(),
            )?;
        } else {
            fs::rename(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    let _remove_pross = source_dir.remove();
    dir
}
