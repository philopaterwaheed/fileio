mod Dirs;
mod Files;

use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use std::fs;
use std::io::{self, Write, BufReader, BufRead};
use std::path::{Path, PathBuf};
use Dirs::dirs;
use Files::files;
use std::process::{Stdio,Command};

#[derive(Debug, Clone)]
enum Entry {
    // all the operations will be on Entry to detrmine which funtions to use
    file(files::File),
    dir(dirs::Directory),
    None, // for empty dirs they don't have Entries in them
}

static mut CLEAR: bool = false;

// Function to get the pins file path
fn get_pins_file_path() -> PathBuf {
    if let Some(home_dir) = std::env::var_os("HOME") {
        PathBuf::from(home_dir).join(".fileio_pins")
    } else {
        PathBuf::from(".fileio_pins")
    }
}

// Function to save pins to file
fn save_pins(pins: &Vec<Entry>) -> io::Result<()> {
    let pins_file = get_pins_file_path();
    // clears if it exists
    let mut file = fs::File::create(&pins_file)?;
    
    for pin in pins {
        match pin {
            Entry::dir(d) => {
                writeln!(file, "dir:{}", d.path.to_string_lossy())?;
            }
            Entry::file(f) => {
                writeln!(file, "file:{}", f.path.to_string_lossy())?;
            }
            Entry::None => {}
        }
    }
    Ok(())
}

// Function to load pins from file
fn load_pins() -> io::Result<Vec<Entry>> {
    let pins_file = get_pins_file_path();
    let mut pins = Vec::new();
    
    if !pins_file.exists() {
        return Ok(pins);
    }
    
    let file = fs::File::open(&pins_file)?;
    let reader = BufReader::new(file);
    
    for line in reader.lines() {
        let line = line?;
        if let Some((entry_type, path_str)) = line.split_once(':') {
            let path = PathBuf::from(path_str);
            
            // Only add pins that still exist
            if path.exists() {
                match entry_type {
                    "dir" => {
                        if let Ok(dir) = dirs::Directory::new(&path) {
                            pins.push(Entry::dir(dir));
                        }
                    }
                    "file" => {
                        if let Ok(file) = files::File::new(&path) {
                            pins.push(Entry::file(file));
                        }
                    }
                    _ => {} // Ignore invalid entries
                }
            }
        }
    }
    
    Ok(pins)
}

fn main() -> io::Result<()> {
    let curr_dir: &mut dirs::Directory = &mut dirs::Directory::get_env_dir().unwrap();
    let prev_sel: usize = 0;
    let sel: usize = 0; // the cursor posison
    let selected: Entry = Entry::None; // the selected entry that cursor points to

    let selections = &mut (prev_sel, curr_dir, sel, selected);
    let contains = &mut selections.1.vec_of_contains().unwrap(); // the contains of the opening dir

    let mut input_mode: bool = false; //to now if we are inputing
    let mut input_string = &mut String::new(); // the string the user will enter
    let mut opera_code: usize = 0; //the code of the operation the user will do
    let mut input_state = (&mut input_mode, input_string, &mut opera_code);

    let mut buffer_vec: &mut Vec<(Entry, bool)> = &mut Vec::new(); // the copy and paste buffer
    let mut buffer_state = (0 as usize, buffer_vec); // the buffer vector and its selection

    let mut search: (&mut Vec<usize>, usize, &mut String) =
        (&mut Vec::new(), 0, &mut String::new()); // the search results indexs and the selected index

    let mut pins_vec = load_pins().unwrap_or_else(|_| Vec::new()); // load pins or create empty vec
    let mut pins: &mut Vec<Entry> = &mut pins_vec; // pinned entries
    let mut pin_selection: usize = 0; // current selection in pins
    let mut show_pins_popup: bool = false; // flag to show pins popup

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut should_quit = false;
    update(selections, contains);
    while !should_quit {
        terminal.draw(|f| ui(f, selections, contains, &mut input_state, &mut buffer_state, pins, &mut pin_selection, &mut show_pins_popup))?;
        should_quit = handle_events(
            selections,
            contains,
            &mut input_state,
            &mut buffer_state,
            &mut search,
            pins,
            &mut pin_selection,
            &mut show_pins_popup,
        )?;
        unsafe {
            if CLEAR {
                // clear before entering sub terminal
                let _ = enable_raw_mode();
                let _ = terminal.clear();
                CLEAR = false;
            }
        }
    }

    // eartave pins before exiting
    let _ = save_pins(pins);

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    std::io::stdout().execute(Show)?;
    Ok(())
}

fn handle_events(
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    contains: &mut (Vec<PathBuf>, Vec<String>),
    input_state: &mut (&mut bool, &mut String, &mut usize),
    buffer_state: &mut (usize, &mut Vec<(Entry, bool)>),
    search: &mut (&mut Vec<usize>, usize, &mut String),
    pins: &mut Vec<Entry>,
    pin_selection: &mut usize,
    show_pins_popup: &mut bool,
) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if !*(input_state.0/* mode of input */) {
                //if the user not trying to input text
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    //quit
                    return Ok(true);
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Down
                    || key.code == KeyCode::Char('j')
                {
                    if *show_pins_popup {
                        // navigate in pins popup
                        if !pins.is_empty() && *pin_selection + 1 < pins.len() {
                            *pin_selection += 1;
                        }
                    } else {
                        // normal file navigation
                        if selections.2 + 1 < contains.1.len() {
                            selections.2 = selections.2 + 1;
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Up
                    || key.code == KeyCode::Char('k')
                {
                    if *show_pins_popup {
                        // navigate in pins popup
                        if !pins.is_empty() && *pin_selection > 0 {
                            *pin_selection -= 1;
                        }
                    } else {
                        // normal file navigation
                        if selections.2 > 0 {
                            selections.2 = selections.2 - 1;
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Left
                    || key.code == KeyCode::Char('h')
                {
                    let _ = selections.1.up();
                    selections.2 = selections.0;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Right
                    || key.code == KeyCode::Char('l')
                {
                    match &selections.3 {
                        Entry::dir(_d) => {
                            // if the selected is a dir enter it
                                let _ = selections.1.down(selections.2);
                                selections.0 = selections.2;
                                selections.2 = 0;
                        }
                        Entry::file(f) => {
                            // todo!() /* open the file with it's defualt app*/
                           let _ =  open_in_default(f.path.as_path());
                        }
                        Entry::None => {}
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter
                    || key.code == KeyCode::Char('S')
                {
                    if *show_pins_popup {
                        // navigate to selected pin and close popup
                        if !pins.is_empty() && *pin_selection < pins.len() {
                            match &pins[*pin_selection] {
                                Entry::dir(d) => {
                                    if let Ok(new_dir) = dirs::Directory::new(&d.path) {
                                        *selections.1 = new_dir;
                                        selections.2 = 0;
                                    }
                                }
                                Entry::file(f) => {
                                    if let Some(parent) = f.path.parent() {
                                        if let Ok(new_dir) = dirs::Directory::new(parent) {
                                            *selections.1 = new_dir;
                                            // Find the file in the directory and select it
                                            if let Ok(contains_result) = selections.1.vec_of_contains() {
                                                if let Some(pos) = contains_result.0.iter().position(|p| p == &f.path) {
                                                    selections.2 = pos;
                                                }
                                            }
                                        }
                                    }
                                }
                                Entry::None => {}
                            }
                        }
                        *show_pins_popup = false;
                    } else {
                        // normal shell opening
                        let _ = selections.1.start_shell_in_dir();
                        unsafe {
                            CLEAR = true;
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Delete
                    || key.code == KeyCode::Char('D')
                {
                    // removes the Entry
                    match &selections.3 {
                        // checks the type of slection typee
                        Entry::dir(d) => {
                            let _ = d.remove();
                        }
                        Entry::file(f) => {
                            let _ = f.remove();
                        }
                        Entry::None => {}
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('r') {
                    // renames the Entry
                    *input_state.0 = true;
                    *input_state.2 = 1;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('a') {
                    // add a file Entry
                    *input_state.0 = true;
                    *input_state.2 = 2;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('A') {
                    // removes the Entry
                    *input_state.0 = true;
                    *input_state.2 = 3;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('/') {
                    //search current dir
                    *input_state.0 = true;
                    *input_state.2 = 4;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('n') {
                    //select next search result
                    if (search.0).len() > 0 && contains.1.len() > 0 {
                        if (search.1 as i32 + 1) < search.0.len() as i32
                            && search.0[search.1] < contains.1.len()
                            && contains.1[search.0[search.1]].contains(search.2.as_str())
                        {
                            search.1 = search.1 + 1;
                            selections.2 = (search.0)[search.1];
                        } else {
                            if search.0[search.1] <= contains.1.len() - 1 {
                                if contains.1[search.0[search.1]].contains(search.2.as_str()) {
                                    search.1 = 0;
                                    selections.2 = (search.0)[search.1];
                                } else {
                                    search.1 = 0;
                                    search.0.clear();
                                    search.2.clear();
                                }
                            }
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('N') {
                    //select prev search result
                    if (search.0).len() > 0 && contains.1.len() > 0 {
                        if (search.1 as i32 - 1) >= 0
                            && search.0[search.1] < contains.1.len()
                            && contains.1[search.0[search.1]].contains(search.2.as_str())
                        {
                            search.1 = search.1 - 1;
                            selections.2 = (search.0)[search.1];
                        } else {
                            if search.0[search.1] <= contains.1.len() - 1 {
                                if contains.1[search.0[search.1]].contains(search.2.as_str()) {
                                    search.1 = search.0.len() - 1;
                                    selections.2 = (search.0)[search.1];
                                } else {
                                    search.1 = 0;
                                    search.0.clear();
                                    search.2.clear();
                                }
                            }
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('y') {
                    // copy the Entry
                    let copy_entry: Entry;
                    match &selections.3 {
                        // chciking the type of selected entry
                        Entry::dir(d) => {
                            copy_entry = Entry::dir(dirs::Directory::new(d.path.as_path()).unwrap())
                        }
                        Entry::file(f) => {
                            copy_entry = Entry::file(files::File::new(f.path.as_path()).unwrap())
                        }
                        Entry::None => copy_entry = Entry::None,
                    }
                    if let Entry::None = copy_entry {
                    } else {
                        buffer_state.1.push((copy_entry, false));
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('d') {
                    // removes the Entry
                    let cut_entry: Entry;
                    match &selections.3 {
                        // chciking the type of selected entry
                        Entry::dir(d) => {
                            cut_entry = Entry::dir(dirs::Directory::new(d.path.as_path()).unwrap())
                        }
                        Entry::file(f) => {
                            cut_entry = Entry::file(files::File::new(f.path.as_path()).unwrap())
                        }
                        Entry::None => cut_entry = Entry::None,
                    }
                    if let Entry::None = cut_entry {
                    } else {
                        buffer_state.1.push((cut_entry, true));
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('p') {
                    // pastes buffer the Entry
                    for (entry, operation) in buffer_state.1.iter_mut() {
                        match &entry {
                            // chciking the type of element
                            Entry::dir(d) => {
                                if *operation {
                                    // if we are cutting a dir
                                    if let Ok(_x) = move_dir(&d, selections.1) {
                                        // if we are not moving into self ;
                                    } else {
                                    }
                                } else {
                                    // if we are copying
                                    let _ = copy_dir(&d, selections.1);
                                }
                            }
                            Entry::file(f) => {
                                if *operation {
                                    // if we are cutting a dir
                                    let _ = move_file(&f, selections.1);
                                } else {
                                    // if we are copying
                                    let _ = copy_file(&f, selections.1);
                                }
                            }
                            Entry::None => {}
                        }
                    }
                    buffer_state.1.clear(); //clear the buffer
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('w') {
                    // decreese the buffer selection
                    if buffer_state.0 > 0 {
                        buffer_state.0 = buffer_state.0 - 1;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('s') {
                    // increese the buffer selection
                    if buffer_state.0 < (buffer_state.1.len() as i8 - 1) as usize {
                        // if we are less than the buffer
                        // size
                        buffer_state.0 = buffer_state.0 + 1;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('x') {
                    // delete selection from buffer
                    if buffer_state.1.len() != 0 && buffer_state.0 < buffer_state.1.len() {
                        buffer_state.1.remove(buffer_state.0);
                    }
                    buffer_state.0 = 0;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('P') {
                    // pin current selection
                    if !matches!(selections.3, Entry::None) {
                        let pin_entry = selections.3.clone();
                        if !pins.iter().any(|p| match (p, &pin_entry) {
                            (Entry::dir(d1), Entry::dir(d2)) => d1.path == d2.path,
                            (Entry::file(f1), Entry::file(f2)) => f1.path == f2.path,
                            _ => false,
                        }) {
                            pins.push(pin_entry);
                            let _ = save_pins(pins); // Save pins after adding
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('u') {
                    // unpin current selection
                    if !matches!(selections.3, Entry::None) {
                        let original_len = pins.len();
                        pins.retain(|p| match (p, &selections.3) {
                            (Entry::dir(d1), Entry::dir(d2)) => d1.path != d2.path,
                            (Entry::file(f1), Entry::file(f2)) => f1.path != f2.path,
                            _ => true,
                        });
                        // Save pins if any were removed
                        if pins.len() != original_len {
                            let _ = save_pins(pins);
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('o') {
                    // toggle pins popup
                    *show_pins_popup = !*show_pins_popup;
                    if *show_pins_popup && !pins.is_empty() {
                        *pin_selection = 0; // reset selection when opening popup
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Esc {
                    // close pins popup
                    if *show_pins_popup {
                        *show_pins_popup = false;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('U') {
                    // remove selected pin from pins list (when in popup)
                    if *show_pins_popup && !pins.is_empty() && *pin_selection < pins.len() {
                        pins.remove(*pin_selection);
                        let _ = save_pins(pins); // Save pins after removing
                        
                        // Adjust pin selection if needed
                        if *pin_selection >= pins.len() && pins.len() > 0 {
                            *pin_selection = pins.len() - 1;
                        } else if pins.is_empty() {
                            *pin_selection = 0;
                        }
                    }
                }
            } else {
                //if we are in input mode
                match key.code {
                    KeyCode::Char(c) => {
                        input_state.1.push(c);
                    }
                    KeyCode::Enter => {
                        let _ = input_operation_excute(selections, input_state, contains, search);
                        *input_state.0 = false;
                        input_state.1.clear();
                        *input_state.2 = 0;
                    }
                    KeyCode::Backspace => {
                        input_state.1.pop();
                    }
                    KeyCode::Esc => {
                        // exiting out of input mode
                        *input_state.0 = false;
                        input_state.1.clear();
                        *input_state.2 = 0;
                    }
                    _ => {}
                }
            }
        }
        update(selections, contains);
    }
    Ok(false)
}
fn input_operation_excute(
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    input_state: &mut (&mut bool, &mut String, &mut usize),
    contains: &mut (Vec<PathBuf>, Vec<String>),
    search: &mut (&mut Vec<usize>, usize, &mut String),
) -> io::Result<()> {
    match input_state.2 {
        1 => {
            // the rename operation
            match &mut selections.3 {
                // checks the Entry type
                Entry::dir(d) => {
                    let _ = d.rename(input_state.1);
                }
                Entry::file(f) => {
                    let _ = f.rename(input_state.1);
                }
                Entry::None => {}
            }
        }
        2 => {
            // adding a file
            let _new_file = files::File::new(selections.1.path.join(&input_state.1).as_path());
        }
        3 => {
            // adding a dir
            let _new_dir = dirs::Directory::new(selections.1.path.join(&input_state.1).as_path());
        }
        4 => {
            // search
            if let Ok(searched) = search_dir(&input_state.1, contains) {
                *(search.0) = searched.clone();
                if searched.len() > 0 {
                    selections.2 = searched[0];
                    *search.2 = input_state.1.clone();
                    search.1 = 0;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn ui(
    frame: &mut Frame,
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    constants: &mut (Vec<PathBuf>, Vec<String>),
    input_state: &mut (&mut bool, &mut String, &mut usize),
    buffer_state: &mut (usize, &mut Vec<(Entry, bool)>),
    pins: &mut Vec<Entry>,
    pin_selection: &mut usize,
    show_pins_popup: &mut bool,
) {
    let commands = vec![
        Row::new([
            "('D' , Delete  : remove)",
            "('y'   :  copy )",
            "('x'   :  remove from buffer )",
            "('S'   :  open shell in dir )",
            "('P'   :  pin )"
        ]),
        Row::new([
            "('a'   : add file)",
            "('d'   :  cut )",
            "('w'   :  buffer up )",
            "('q'   :  quit )",
            "('u'   :  unpin )",
        ]),
        Row::new([
            "('A'   : add dir)",
            "('p'   :  paste )",
            "('s'   :  buffer down )",
            "('Arrows'   :  movments )",
            "('o'   :  pins popup )",
        ]),
        Row::new(["('r'   :  rename )", "", "", "('/'  :  search )" , "('U'   :  remove pin )"]),
        Row::new(["" , "" , "" ,  "('N'   :  prev search)"  ]),
        Row::new(["" , "" , "", "('n'   :  next search)",  ]),
    ];
    let mut buffer: Vec<String> = Vec::new();
    let curr = &selections.1; // the curr dir
    let prev = curr.prev(); // gets the prev dir
    selections.0 = curr.find_index(); // finds the index of curr in prev
    for (entry, _operation) in buffer_state.1.iter_mut() {
        // getting the buffer as a vec of string
        match &entry {
            // chciking the type of element
            Entry::dir(d) => {
                buffer.push(d.name.to_owned());
            }
            Entry::file(f) => {
                buffer.push(f.name.to_owned());
            }
            Entry::None => {}
        }
    }
    let (_current_contains_pathes, current_contains_strings) = constants; // the contains of curr as vec
                                                                          // of string and PathBuf
    let main_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Ratio(30, 100),
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
        [
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ],
    )
    .split(main_layout[1]);
    let mut sel = ListState::default(); // selection state of curr dir
    let mut no_sel = ListState::default(); // selection state of next dir just set to nothing
    let mut prev_sel = ListState::default(); // selection state of prev dir
    let mut buffer_sel = ListState::default(); // selection state of buffer
    let mut pins_sel = ListState::default(); // selection state of pins
    sel.select(Some(selections.2)); // Select the first item initially
    prev_sel.select(Some(selections.0)); // Select the index of curr in prev
    buffer_sel.select(Some(buffer_state.0)); // Selection of buffer
    pins_sel.select(Some(*pin_selection)); // Selection of pins
    if let Ok(d) = prev {
        // checks if the prev dir exists in case we are in /
        let prev_c = d.vec_of_contains().unwrap().1;
        render_list(frame, inner_layout[0], prev_c, &mut prev_sel, "Prev"); // renders it
    }
    render_list(
        frame,
        inner_layout[1],
        current_contains_strings.to_owned(),
        &mut sel,
        format!("Curr").as_str(),
    );
    match &selections.3 {
        // the content of selected
        Entry::file(f) => {
            if let Ok(contents) = f.read() {
                render_list(
                    frame,
                    inner_layout[2],
                    contents,
                    &mut no_sel,
                    "file content",
                );
            }
        }
        Entry::dir(d) => {
            if let Ok(contains) = d.vec_of_contains() {
                let next_c = contains.1; // the paths  if ok
                render_list(frame, inner_layout[2], next_c, &mut no_sel, "next");
            } else {
            }
        }
        Entry::None => {}
    }

    // the operations area
    let down_layout = Layout::new(
        // the down square
        Direction::Vertical,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .split(main_layout[2]);

    frame.render_widget(
        // the text input box
        Paragraph::new(input_state.1.to_owned()).block(
            Block::default().title("input").borders(Borders::ALL).style(
                Style::new()
                    .blue()
                    .fg(if !*input_state.0 {
                        Color::White
                    } else {
                        Color::Black
                    })
                    .bg(if !*input_state.0 {
                        Color::Black
                    } else {
                        Color::White
                    }),
            ),
        ),
        down_layout[0],
    );
    // the command area
    let operation_layout = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .split(down_layout[1]);
    render_table(frame, operation_layout[0], &commands); // the commands
    render_list(
        frame,
        operation_layout[1],
        buffer,
        &mut buffer_sel,
        "buffer",
    ); // the buffer

    // Render pins popup if enabled
    if *show_pins_popup {
        render_pins_popup(frame, pins, pin_selection);
    }
}

fn update(
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    contains: &mut (Vec<PathBuf>, Vec<String>),
) {
    // updates the selected entry
    if let Ok(temp) = selections.1.vec_of_contains() {
        // restets the contains
        contains.0 = temp.0;
        contains.1 = temp.1;
        // checks the path of the selected Entry
        if selections.2 >= contains.0.len() {
            // if we are now selecting byond the dir size
            // becuase of delete or something like that
            selections.2 = std::cmp::max(contains.0.len() as i32 - 1, 0) as usize;
            // the compare for if the dir empty
            // the selection would be -1
        }
        if contains.0.len() > 0 {
            // to avoid 0 indexing if the dir is empty
            let path = contains.0[selections.2].as_path(); // the path of the entry selected
            if path.is_dir() {
                if let Ok(new) = dirs::Directory::new(path) {
                    // if its ok to select it
                    selections.3 = Entry::dir(new); // setting it to the dir
                } else {
                    selections.3 = Entry::None; // setting it to a None just as a place holder
                }
            } else if path.is_file() {
                if let Ok(new) = files::File::new(path) {
                    selections.3 = Entry::file(new); // setting it to the file
                } else {
                    selections.3 = Entry::None;
                }
            } else {
                // if the dir is empty set the Entry to None
                selections.3 = Entry::None;
            }
        } else {
            // if the dir is empty set the Entry to None
            selections.3 = Entry::None;
        }
    }
}

fn render_list(
    frame: &mut Frame,
    rect: Rect,
    contains: Vec<String>,
    state: &mut ListState,
    title: &str,
) {
    // a function to render lists
    frame.render_stateful_widget(
        List::new(contains)
            .block(Block::default().title(title).borders(Borders::ALL)) // sets the
            // title
            .style(Style::new().blue().fg(Color::White).bg(Color::Black))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            )
            .highlight_symbol(">>")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom)
            .highlight_spacing(HighlightSpacing::Always),
        rect,
        state,
    );
}

fn render_table(frame: &mut Frame, rect: Rect, data: &Vec<Row>) {
    let mut sel = TableState::default(); // selection state of curr dir

    // Define the widths of the columns
    let widths: &[Constraint; 5] = &[
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ];

    // Render the table with the provided data
    frame.render_stateful_widget(
        Table::new(data.to_vec(), widths) // Convert array to Vec
            .style(Style::new().blue().fg(Color::White).bg(Color::Black))
            .block(Block::default().title("commands").borders(Borders::ALL))
            .header(
                Row::new(vec!["atler", "ccp", "buffer", "controls" , "pin"]).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .widths(widths),
        rect,
        &mut sel,
    );
}

fn render_pins_popup(frame: &mut Frame, pins: &mut Vec<Entry>, pin_selection: &mut usize) {
    // Create a centered popup area
    let popup_area = centered_rect(60, 40, frame.size());
    
    // Clear the area first
    frame.render_widget(Clear, popup_area);
    
    // Create pins display
    let mut pins_display: Vec<String> = Vec::new();
    if pins.is_empty() {
        pins_display.push("No pins available".to_string());
        pins_display.push("Press 'P' to pin files/folders".to_string());
    } else {
        for (_i, pin) in pins.iter().enumerate() {
            let pin_name = match pin {
                Entry::dir(d) => format!("📁 {}", d.name),
                Entry::file(f) => format!("📄 {}", f.name),
                Entry::None => "".to_string(),
            };
            pins_display.push(pin_name);
        }
    }

    // Create list state and select current pin
    let mut list_state = ListState::default();
    if !pins.is_empty() && *pin_selection < pins.len() {
        list_state.select(Some(*pin_selection));
    }

    // Render the popup
    frame.render_stateful_widget(
        List::new(pins_display)
            .block(
                Block::default()
                    .title("Pins (Enter: Go to, Esc: Close, U: Remove)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Black)
                    .bg(Color::Cyan)
            )
            .highlight_symbol("→ ")
            .repeat_highlight_symbol(true),
        popup_area,
        &mut list_state,
    );
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn copy_file(
    file: &files::File,
    dircetion_dir: &dirs::Directory,
) -> Result<files::File, io::Error> {
    let new_name = file.name.clone();
    fs::copy(
        file.path.as_path(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    )?;
    let out_file = files::File::new(dircetion_dir.path.join(new_name.as_str()).as_path());
    out_file
}
fn move_file(
    file: &files::File,
    dircetion_dir: &dirs::Directory,
) -> Result<files::File, io::Error> {
    let new_name = file.name.clone();
    fs::rename(
        file.path.as_path(),
        dircetion_dir.path.join(new_name.as_str()).as_path(),
    )?;
    let out_file = files::File::new(dircetion_dir.path.join(new_name.as_str()).as_path());
    out_file
}

fn copy_dir(
    source_dir: &dirs::Directory,
    dircetion_dir: &dirs::Directory,
) -> Result<dirs::Directory, io::Error> {
    let new_name = source_dir.name.clone();
    let new_path = &dircetion_dir.path.join(&new_name);
    let dir = dirs::Directory::new(new_path);

    for entry in fs::read_dir(source_dir.path.to_owned())? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            let entry_name = entry.file_name().into_string().ok().unwrap();
            copy_dir(
                &dirs::Directory::new(entry.path().as_path()).unwrap(),
                &dirs::Directory::new(new_path.as_path()).unwrap(),
            )?;
        } else {
            fs::copy(entry.path(), new_path.join(entry.file_name()))?;
        }
    }
    dir
}

fn move_dir(
    source_dir: &dirs::Directory,
    dircetion_dir: &dirs::Directory,
) -> Result<dirs::Directory, io::Error> {
    let new_name = source_dir.name.clone();
    let new_path = &dircetion_dir.path.join(&new_name);
    if dircetion_dir.path.as_path() != source_dir.path.as_path() {
        let dir = dirs::Directory::new(new_path);

        for entry in fs::read_dir(source_dir.path.to_owned())? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                move_dir(
                    &dirs::Directory::new(entry.path().as_path()).unwrap(),
                    &dirs::Directory::new(new_path.as_path()).unwrap(),
                )?;
            } else {
                fs::rename(entry.path(), new_path.join(entry.file_name()))?;
            }
        }
        let _remove_pross = source_dir.remove();
        dir
    } else {
        let error_message = format!("move into self");
        Err(io::Error::new(io::ErrorKind::NotFound, error_message))
    }
}
fn search_dir(
    query: &str,
    contains: &mut (Vec<PathBuf>, Vec<String>),
) -> Result<Vec<usize>, io::Error> {
    let contains = contains.1.to_owned();
    // Find the indices of strings that contain the substring
    let indices: Vec<usize> = contains
        .iter()
        .enumerate()
        .filter(|(_, s)| s.contains(query))
        .map(|(i, _)| i)
        .collect();

    Ok(indices)
}
fn open_in_default(path : &Path) -> Result<(), Box<dyn std::error::Error>>{
    Command::new("xdg-open".to_string()   )
        .arg(path.to_str().unwrap())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
