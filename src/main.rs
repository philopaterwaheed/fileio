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
use std::path::{Path,PathBuf};
use Dirs::dirs;
use Files::files;

#[derive(Debug)]
enum Entry { // all the operations will be on Entry to detrmine which funtions to use 
    file (files::File),
    dir (dirs::Directory),
    None // for empty dirs they don't have Entries in them 
}
 
static mut CLEAR :bool = false ; 


fn main() -> io::Result<()>{

    let curr_dir : &mut dirs::Directory = &mut dirs::Directory::get_env_dir().unwrap();
    let prev_sel :usize = 0;
    let sel:usize  = 0 ; 
    let selected :  Entry  = Entry::None;

    let selections = &mut (prev_sel,curr_dir,sel,selected);
    let contains  = &mut selections.1.vec_of_contains().unwrap();

    let mut input_mode : bool = false; //to now if we are inputing 
    let mut input_string = &mut String ::new();// the string the user will enter 
    let mut opera_code : usize = 0 ; //the code of the operation the user will do
    let mut input_state = (&mut input_mode, input_string , &mut opera_code);

    let mut buffer_vec : &mut Vec<(Entry, bool)> = &mut Vec::new() ; // the copy and paste buffer
    let mut buffer_state = (0 as usize, buffer_vec); // hte buffer vector and its selection

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut should_quit = false;
    update(selections ,contains);
    while !should_quit {
        terminal.draw(|f|ui(f,selections , contains,&input_state.1.to_owned(),&mut buffer_state))?;
        should_quit = handle_events(selections, contains,  &mut input_state, &mut buffer_state)?;
        unsafe{
            if CLEAR{ // clear before entering sub terminal
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

fn handle_events(selections : &mut (usize , &mut dirs::Directory ,usize ,  Entry), contants  : &mut (Vec<PathBuf>,Vec<String>),input_state: &mut (&mut bool,&mut String , &mut usize) , buffer_state: &mut (usize,&mut Vec<(Entry, bool)>)) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if !*(input_state.0 /* mode of input */ ){ //if the user not trying to input text
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') { //quit 
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
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Delete||key.code == KeyCode::Char('D'){ // removes the Entry
                    match &selections.3{// checks the type of slection typee
                        Entry::dir(d)=>{let _ = d.remove();},
                        Entry::file(f)=>{let _ = f.remove();},
                        Entry::None=>{},
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('r'){ // removes the Entry
                    *input_state.0= true; 
                    *input_state.2= 1; 
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('y'){ // copy the Entry
                    let copy_entry : Entry ; 
                    match & selections.3 { // chciking the type of selected entry
                        Entry::dir(d)=>{copy_entry = Entry::dir(dirs::Directory::new(d.path.as_path()).unwrap()) },
                        Entry::file(f)=>{copy_entry = Entry::file(files::File::new(f.path.as_path()).unwrap()) },
                        Entry::None=>{copy_entry = Entry::None},
                    }
                    if let Entry::None = copy_entry{}
                    else {buffer_state.1.push ((copy_entry,false));}
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('d'){ // removes the Entry
                    let cut_entry : Entry ; 
                    match & selections.3 { // chciking the type of selected entry
                        Entry::dir(d)=>{cut_entry = Entry::dir(dirs::Directory::new(d.path.as_path()).unwrap()) },
                        Entry::file(f)=>{cut_entry = Entry::file(files::File::new(f.path.as_path()).unwrap()) },
                        Entry::None=>{cut_entry = Entry::None},
                    }
                    if let Entry::None = cut_entry{}
                    else {buffer_state.1.push ((cut_entry,true));}
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('p'){ // pastes buffer the Entry
                    for  (entry , operation )in  buffer_state.1.iter_mut(){
                        match &entry { // chciking the type of element
                                Entry::dir(d)=>{

                                    if *operation{// if we are cutting a dir
                                        if let Ok(x) =  move_dir(&d, selections.1){
                                            // if we are not moving into self ; 
                                        }
                                        else{

                                        }
                                    }
                                    else { // if we are copying
                                       let _= copy_dir(&d, selections.1);
                                    }
                                },
                                Entry::file(f)=>{
                                    if *operation {// if we are cutting a dir
                                        let _= move_file(&f, selections.1);
                                    }
                                    else { // if we are copying
                                       let _= copy_file(&f, selections.1);
                                    }
                                },
                                Entry::None=>{},
                            }
                    }
                    buffer_state.1.clear();//clear the buffer 
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('w'){ // decreese the buffer selection
                    if buffer_state.0 > 0{
                        buffer_state.0 = buffer_state.0 - 1 ;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('s'){ // increese the buffer selection
                    if buffer_state.0 <  buffer_state.1.len()-1 { // if we are less than the buffer
                                                               // size
                        buffer_state.0 = buffer_state.0 + 1 ;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('x'){ // delete
                                                                                             // selection
                                                                                             // from
                                                                                             // buffer
                    if buffer_state.1.len() !=0 && buffer_state.0 < buffer_state.1.len(){
                        buffer_state.1.remove(buffer_state.0);
                    }
                    buffer_state.0 = 0 ; 
                }
                                                                                             
            }
            else{ //if we are in input mode
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                }
                    match key.code{
                        KeyCode::Char(c)=>{
                            input_state.1.push(c);

                        }
                        KeyCode::Enter=>{
                            let _ = input_operation_excute(selections,input_state);
                            *input_state.0= false;
                            input_state.1.clear() ;
                            *input_state.2= 0;
                        },
                        KeyCode::Backspace =>{
                            input_state.1.pop();
                        },
                        _=>{}
                    }

                }
        }
        update(selections,contants);
    }
    Ok(false)
}
fn input_operation_excute(selections : &mut (usize , &mut dirs::Directory ,usize ,  Entry),input_state: &mut (&mut bool,&mut String , &mut usize)) -> io::Result<()> {
    match input_state.2 {

        1=>{ // the rename operation
            match &mut selections.3 { // checks the Entry type 
                    Entry::dir(d)=>{let _ = d.rename(input_state.1);},
                    Entry::file(f)=>{let _ = f.rename(input_state.1);},
                    Entry::None=>{},
                }

        }, 
        _=>{},
    }
    Ok(())
}

fn ui(frame: &mut Frame, selections : &mut (usize , &mut dirs::Directory ,usize ,  Entry) , constants  : &mut (Vec<PathBuf>,Vec<String>), input_string: &str, buffer_state: &mut (usize,&mut Vec<(Entry, bool)>)) {
    let commands: Vec<String> = vec![
    "('D' , Delete  : remove)".to_string(),
    "('r'   :  rename )".to_string(),
];
 let mut buffer : Vec <String> = Vec::new();
 let curr = &selections.1; // the curr dir
 let prev = curr.prev(); // gets the prev dir
 selections.0 = curr.find_index(); // finds the index of curr in prev 
for  (entry , _operation )in  buffer_state.1.iter_mut(){ // getting the buffer as a vec of string
    match &entry { // chciking the type of element
            Entry::dir(d)=>{
                buffer.push(d.name.to_owned());
            },
            Entry::file(f)=>{
                buffer.push(f.name.to_owned());
            },
            Entry::None=>{},
        }
}
 let (current_contains_pathes,current_contains_strings)= constants; // the contains of curr as vec
                                                                    // of string and PathBuf
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
        Block::new().borders(Borders::TOP).title("Fileio by philo"),
        main_layout[0],
    );


    // the expplorer layout 
    let inner_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50),Constraint::Percentage(50)],
    )
    .split(main_layout[1]);
    let mut sel = ListState::default(); // selection state of curr dir
    let mut no_sel = ListState::default();// selection state of next dir just set to nothing 
    let mut prev_sel = ListState::default();// selection state of prev dir
    let mut buffer_sel = ListState::default();// selection state of buffer
    sel.select(Some(selections.2)); // Select the first item initially
    prev_sel.select(Some(selections.0)); // Select the index of curr in prev
    buffer_sel.select(Some(buffer_state.0)); // Selection of buffer
    if let Ok(d) = prev  { // checks if the prev dir exists in case we are in / 
        let prev_c = d.vec_of_contains().unwrap().1;
        render_list(frame, inner_layout[0],prev_c , &mut prev_sel,"Prev"); // renders it 
    }
    render_list(frame, inner_layout[1],current_contains_strings.to_owned() , &mut sel,format!("Curr").as_str());

     if let Entry::dir(d) = &selections.3 { // if the selection is on a dir 
             let next_c = d.vec_of_contains().unwrap().1;
             render_list(frame, inner_layout[2],next_c , &mut no_sel,"next");
     }

    // the operations area
    let down_layout = Layout::new( // the down square
        Direction::Vertical,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .split(main_layout[2]);

    frame.render_widget(// the texxt input box
        Paragraph::new(input_string)
            .block(Block::default().title("input").borders(Borders::ALL)),
        down_layout[0],
    );
    // the command area
    let operation_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(down_layout[1]);
    render_list(frame, operation_layout[0] ,commands , &mut no_sel,"command");
    render_list(frame, operation_layout[1] ,buffer, &mut buffer_sel,"buffer");// hte buffer

}

fn update (selections : &mut (usize , &mut dirs::Directory ,usize ,Entry) , contains  : &mut (Vec<PathBuf>,Vec<String>)){ // updates the selected entry 
    let temp = selections.1.vec_of_contains().unwrap(); // gets what current dir contains
    contains.0  = temp.0; 
    contains.1 = temp.1; 
    // checks the path of the selected Entry
    if selections.2  >= contains.0.len() {
        // if we are now selecting byond the dir size 
        // becuase of delete or something like that 
        selections.2 = std::cmp::max (contains.0.len() as i32 - 1 ,0)as usize; // the compare for if the dir empty
                                                             // the selection would be -1 
    }
    if contains.0.len() > 0 { // to avoid 0 indexing if the dir is empty 
        let path  = contains.0[selections.2].as_path(); // the path of the entry selected
        if path.is_dir(){
            selections.3 =  Entry::dir(dirs::Directory::new(path).unwrap()); // setting it to the dir
                                                                             // variant 
        }
        else{
            selections.3 =  Entry::file(files::File::new(path).unwrap()); // setting it to the file
                                                                             // variant 
        }
    }
    else { // if the dir is empty set the Entry to None 
        selections.3 = Entry::None;
    }

}

fn render_list (frame : &mut Frame ,rect : Rect ,contains : Vec<String>,state:&mut ListState, title : &str) { // a function to render lists 
    frame.render_stateful_widget(
        List::new(contains).block(Block::default().title(title).borders(Borders::ALL)) // sets the
                                                                                       // title 
        .style(Style::new().blue())
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .direction(ListDirection::TopToBottom)
        .highlight_spacing(HighlightSpacing::Always),
        rect,state
     );
}
fn copy_file(file: &files::File, dircetion_dir: &dirs::Directory) -> Result<files::File, io::Error> {
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
fn move_file(file: &files::File, dircetion_dir: &dirs::Directory) -> Result<files::File, io::Error> {
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

fn move_dir(source_dir: &dirs::Directory, dircetion_dir: &dirs::Directory)-> Result<dirs::Directory, io::Error> {
    let new_name = source_dir.name.clone();
    let new_path = &dircetion_dir.path.join(&new_name);
    if dircetion_dir.path.as_path() != source_dir.path.as_path(){ 
        let dir = dirs::Directory::new(new_path);

        for entry in fs::read_dir(source_dir.path.to_owned())? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                move_dir(
                     &dirs::Directory::new(entry.path().as_path())
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
    else {
        let error_message = format!("move into self" );
        Err(io::Error::new(io::ErrorKind::NotFound, error_message))
    }
}
