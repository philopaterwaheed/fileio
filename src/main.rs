mod Dirs;
mod Files;

use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};
use ratatui_image::{picker::Picker, StatefulImage, protocol::StatefulProtocol};
use std::{fs};
use std::io::{self, Write, BufReader, BufRead, Read};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::time::UNIX_EPOCH;
use Dirs::dirs;
use Files::files;
use std::process::{Stdio,Command};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone)]
enum Entry {
    // all the operations will be on Entry to detrmine which funtions to use
    file(files::File),
    dir(dirs::Directory),
    None, // for empty dirs they don't have Entries in them
}

#[derive(Debug, Clone)]
struct PinnedEntry {
    entry: Entry,
    category: String,
}

#[derive(Debug, Clone)]
struct CategoryManager {
    pins: Vec<PinnedEntry>,
    
    category_to_entries: HashMap<String, Vec<usize>>,
    entry_path_to_index: HashMap<PathBuf, usize>,
    categories: Vec<String>,
    category_counts: HashMap<String, usize>,
}

impl CategoryManager {
    fn new() -> Self {
        Self {
            pins: Vec::new(),
            category_to_entries: HashMap::new(),
            entry_path_to_index: HashMap::new(),
            categories: vec!["Default".to_string()],
            category_counts: HashMap::new(),
        }
    }
    
    fn from_pins(pins: Vec<PinnedEntry>) -> Self {
        let mut manager = Self::new();
        manager.pins = pins;
        manager.rebuild_indices();
        manager
    }
    
    fn rebuild_indices(&mut self) {
        self.category_to_entries.clear();
        self.entry_path_to_index.clear();
        self.category_counts.clear();
        
        for (index, pin) in self.pins.iter().enumerate() {
            self.category_to_entries
                .entry(pin.category.clone())
                .or_insert_with(Vec::new)
                .push(index);
            
            let path = match &pin.entry {
                Entry::file(f) => f.path.clone(),
                Entry::dir(d) => d.path.clone(),
                Entry::None => continue,
            };
            self.entry_path_to_index.insert(path, index);
            
            *self.category_counts.entry(pin.category.clone()).or_insert(0) += 1;
        }
        
        let mut categories: Vec<String> = self.category_to_entries.keys().cloned().collect();
        categories.sort();
        if categories.is_empty() {
            categories.push("Default".to_string());
        }
        self.categories = categories;
    }
    
    fn add_pin(&mut self, entry: Entry, category: String) {
        let path = match &entry {
            Entry::file(f) => f.path.clone(),
            Entry::dir(d) => d.path.clone(),
            Entry::None => return,
        };
        
        if self.entry_path_to_index.contains_key(&path) {
            return;
        }
        
        let index = self.pins.len();
        let pin = PinnedEntry { entry, category: category.clone() };
        
        self.pins.push(pin);
        
        self.category_to_entries
            .entry(category.clone())
            .or_insert_with(Vec::new)
            .push(index);
        
        self.entry_path_to_index.insert(path, index);
        
        *self.category_counts.entry(category.clone()).or_insert(0) += 1;
        
        if !self.categories.contains(&category) {
            self.categories.push(category);
            self.categories.sort();
        }
    }
    
    fn remove_pin_by_path(&mut self, path: &PathBuf) -> bool {
        if let Some(&index) = self.entry_path_to_index.get(path) {
            let pin = &self.pins[index];
            let category = pin.category.clone();
            
            self.pins.remove(index);
            
            self.rebuild_indices();
            
            if !self.category_counts.contains_key(&category) || self.category_counts[&category] == 0 {
                self.categories.retain(|c| c != &category);
            }
            
            true
        } else {
            false
        }
    }
    
    fn change_category(&mut self, pin_index: usize, new_category: String) {
        if pin_index >= self.pins.len() {
            return;
        }
        
        self.pins[pin_index].category = new_category.clone();
        
        self.rebuild_indices();
    }
    
    fn get_categories(&self) -> &Vec<String> {
        &self.categories
    }
    
    fn get_category_count(&self, category: &str) -> usize {
        self.category_counts.get(category).copied().unwrap_or(0)
    }
    
    fn get_entries_in_category(&self, category: &str) -> Vec<PinnedEntry> {
        if let Some(indices) = self.category_to_entries.get(category) {
            indices.iter()
                .filter_map(|&i| self.pins.get(i))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    fn search_entries_in_category(&self, category: &str, query: &str) -> Vec<PinnedEntry> {
        if let Some(indices) = self.category_to_entries.get(category) {
            indices.iter()
                .filter_map(|&i| self.pins.get(i))
                .filter(|pin| entry_contains_query(&pin.entry, query))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    fn pin_exists(&self, entry: &Entry) -> bool {
        let path = match entry {
            Entry::file(f) => &f.path,
            Entry::dir(d) => &d.path,
            Entry::None => return false,
        };
        self.entry_path_to_index.contains_key(path)
    }
    
    fn find_pin_index_by_entry(&self, entry: &Entry) -> Option<usize> {
        let path = match entry {
            Entry::file(f) => &f.path,
            Entry::dir(d) => &d.path,
            Entry::None => return None,
        };
        self.entry_path_to_index.get(path).copied()
    }
    
    fn get_pins(&self) -> &Vec<PinnedEntry> {
        &self.pins
    }
    
    fn get_pins_mut(&mut self) -> &mut Vec<PinnedEntry> {
        &mut self.pins
    }
}

#[derive(Debug, Clone)]
struct CategoryState {
    selected_category: usize,
    entries_in_category: Vec<PinnedEntry>,
    selected_entry: usize,
    show_categories: bool,
    search_mode: bool,
    search_query: String,
    category_input_mode: bool,
    category_input: String,
    entry_to_change: Option<usize>,
}

#[derive(Debug, Clone)]
struct PasteProgress {
    current_file: String,
    files_done: usize,
    total_files: usize,
    bytes_done: u64,
    total_bytes: u64,
    is_active: bool,
}

struct ImagePreview {
    image: Option<StatefulProtocol>,
    image_path: Option<PathBuf>,
    picker: Picker,
}

impl ImagePreview {
    fn new() -> Self {
        // Try to use proper picker that detects font size and protocol
        let picker = match Picker::from_query_stdio() {
            Ok(picker) => picker,
            Err(_) => {
                // Fall back to fixed font size if query fails
                Picker::from_fontsize((8, 12))
            }
        };
        
        Self {
            image: None,
            picker,
            image_path: None,
        }
    }

    fn load_image(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if !is_image_file(path) {
            return Ok(());
        }

        match image::ImageReader::open(path) {
            Ok(reader) => {
                match reader.decode() {
                    Ok(dyn_img) => {
                        let protocol = self.picker.new_resize_protocol(dyn_img);
                        self.image = Some(protocol);
                        self.image_path = Some(path.to_path_buf());
                    }
                    Err(_) => {
                    }
                }
            }
            Err(_) => {
            }
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.image = None;
        self.image_path = None;
    }
}

impl Default for PasteProgress {
    fn default() -> Self {
        Self {
            current_file: String::new(),
            files_done: 0,
            total_files: 0,
            bytes_done: 0,
            total_bytes: 0,
            is_active: false,
        }
    }
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
fn save_pins(category_manager: &CategoryManager) -> io::Result<()> {
    let pins_file = get_pins_file_path();
    // clears if it exists
    let mut file = fs::File::create(&pins_file)?;
    
    for pin in category_manager.get_pins() {
        match &pin.entry {
            Entry::dir(d) => {
                writeln!(file, "dir:{}:{}", pin.category, d.path.to_string_lossy())?;
            }
            Entry::file(f) => {
                writeln!(file, "file:{}:{}", pin.category, f.path.to_string_lossy())?;
            }
            Entry::None => {}
        }
    }
    Ok(())
}

// Function to load pins from file
fn load_pins() -> io::Result<CategoryManager> {
    let pins_file = get_pins_file_path();
    let mut pins = Vec::new();
    
    if !pins_file.exists() {
        return Ok(CategoryManager::new());
    }
    
    let file = fs::File::open(&pins_file)?;
    let reader = BufReader::new(file);
    
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() == 3 {
            let entry_type = parts[0];
            let category = parts[1].to_string();
            let path_str = parts[2];
            let path = PathBuf::from(path_str);
            
            // Only add pins that still exist
            if path.exists() {
                let entry = match entry_type {
                    "dir" => {
                        if let Ok(dir) = dirs::Directory::new(&path) {
                            Some(Entry::dir(dir))
                        } else { None }
                    }
                    "file" => {
                        if let Ok(file) = files::File::new(&path) {
                            Some(Entry::file(file))
                        } else { None }
                    }
                    _ => None
                };
                
                if let Some(entry) = entry {
                    pins.push(PinnedEntry { entry, category });
                }
            }
        }
    }
    
    Ok(CategoryManager::from_pins(pins))
}

fn entry_contains_query(entry: &Entry, query: &str) -> bool {
    match entry {
        Entry::dir(d) => d.name.to_lowercase().contains(&query.to_lowercase()),
        Entry::file(f) => f.name.to_lowercase().contains(&query.to_lowercase()),
        Entry::None => false,
    }
}

impl Default for CategoryState {
    fn default() -> Self {
        Self {
            selected_category: 0,
            entries_in_category: Vec::new(),
            selected_entry: 0,
            show_categories: true,
            search_mode: false,
            search_query: String::new(),
            category_input_mode: false,
            category_input: String::new(),
            entry_to_change: None,
        }
    }
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

    let mut category_manager = load_pins().unwrap_or_else(|_| CategoryManager::new()); // load pins or create empty manager
    let mut category_state = CategoryState::default();
    let mut show_pins_popup: bool = false; // flag to show pins popup
    let mut show_progress_popup: bool = false; // flag to show progress popup manually
    let mut show_file_info_popup: bool = false; // flag to show file info popup

    let paste_progress = Arc::new(Mutex::new(PasteProgress::default()));
    let mut image_preview = ImagePreview::new(); // Add image preview state

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut should_quit = false;
    update(selections, contains, &mut image_preview);
    while !should_quit {
        terminal.draw(|f| ui(f, selections, contains, &mut input_state, &mut buffer_state, &mut category_manager, &mut category_state, &mut show_pins_popup, &paste_progress, &mut show_progress_popup, &mut show_file_info_popup, &mut image_preview))?;
        should_quit = handle_events(
            selections,
            contains,
            &mut input_state,
            &mut buffer_state,
            &mut search,
            &mut category_manager,
            &mut category_state,
            &mut show_pins_popup,
            &paste_progress,
            &mut show_progress_popup,
            &mut show_file_info_popup,
            &mut image_preview,
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

    let _ = save_pins(&category_manager);

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
    category_manager: &mut CategoryManager,
    category_state: &mut CategoryState,
    show_pins_popup: &mut bool,
    paste_progress: &Arc<Mutex<PasteProgress>>,
    show_progress_popup: &mut bool,
    show_file_info_popup: &mut bool,
    image_preview: &mut ImagePreview,
) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if category_state.category_input_mode {
                match key.code {
                    KeyCode::Char(c) => {
                        category_state.category_input.push(c);
                    }
                    KeyCode::Enter => {
                        if let Some(entry_idx) = category_state.entry_to_change {
                            if entry_idx < category_manager.get_pins().len() {
                                category_manager.change_category(entry_idx, category_state.category_input.clone());
                                let _ = save_pins(category_manager);
                                
                                let categories = category_manager.get_categories();
                                if let Some(cat_idx) = categories.iter().position(|c| c == &category_state.category_input) {
                                    category_state.selected_category = cat_idx;
                                }
                                category_state.entries_in_category = category_manager.get_entries_in_category(&categories[category_state.selected_category]);
                            }
                        }
                        category_state.category_input_mode = false;
                        category_state.category_input.clear();
                        category_state.entry_to_change = None;
                    }
                    KeyCode::Backspace => {
                        category_state.category_input.pop();
                    }
                    KeyCode::Esc => {
                        category_state.category_input_mode = false;
                        category_state.category_input.clear();
                        category_state.entry_to_change = None;
                    }
                    _ => {}
                }
                return Ok(false);
            }
            
            // Handle search mode in pins popup
            if *show_pins_popup && category_state.search_mode {
                match key.code {
                    KeyCode::Char(c) => {
                        category_state.search_query.push(c);
                        if !category_state.search_query.is_empty() {
                            let categories = category_manager.get_categories();
                            category_state.entries_in_category = category_manager.search_entries_in_category(
                                &categories[category_state.selected_category], 
                                &category_state.search_query
                            );
                        } else {
                            let categories = category_manager.get_categories();
                            category_state.entries_in_category = category_manager.get_entries_in_category(
                                &categories[category_state.selected_category]
                            );
                        }
                        category_state.selected_entry = 0;
                    }
                    KeyCode::Enter => {
                        category_state.search_mode = false;
                        category_state.search_query.clear();
                    }
                    KeyCode::Backspace => {
                        category_state.search_query.pop();
                        if !category_state.search_query.is_empty() {
                            let categories = category_manager.get_categories();
                            category_state.entries_in_category = category_manager.search_entries_in_category(
                                &categories[category_state.selected_category], 
                                &category_state.search_query
                            );
                        } else {
                            let categories = category_manager.get_categories();
                            category_state.entries_in_category = category_manager.get_entries_in_category(
                                &categories[category_state.selected_category]
                            );
                        }
                        category_state.selected_entry = 0;
                    }
                    KeyCode::Esc => {
                        category_state.search_mode = false;
                        category_state.search_query.clear();
                        let categories = category_manager.get_categories();
                        category_state.entries_in_category = category_manager.get_entries_in_category(
                            &categories[category_state.selected_category]
                        );
                    }
                    _ => {}
                }
                return Ok(false);
            }
            
            if !*(input_state.0/* mode of input */) {
                //if the user not trying to input text
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    //quit
                    if !paste_progress.lock().unwrap().is_active {
                        return Ok(true);
                    }
                    else if !(*show_progress_popup) {
                        // if progress popup is open, close it
                        *show_progress_popup = true;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Down
                    || key.code == KeyCode::Char('j')
                {
                    if *show_pins_popup {
                        if category_state.show_categories {
                            // navigate in categories
                            let categories = category_manager.get_categories();
                            if category_state.selected_category + 1 < categories.len() {
                                category_state.selected_category += 1;
                                category_state.entries_in_category = category_manager.get_entries_in_category(&categories[category_state.selected_category]);
                                category_state.selected_entry = 0;
                            }
                        } else {
                            // navigate in entries
                            if category_state.selected_entry + 1 < category_state.entries_in_category.len() {
                                category_state.selected_entry += 1;
                            }
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
                        if category_state.show_categories {
                            // navigate in categories
                            if category_state.selected_category > 0 {
                                category_state.selected_category -= 1;
                                let categories = category_manager.get_categories();
                                category_state.entries_in_category = category_manager.get_entries_in_category(&categories[category_state.selected_category]);
                                category_state.selected_entry = 0;
                            }
                        } else {
                            // navigate in entries
                            if category_state.selected_entry > 0 {
                                category_state.selected_entry -= 1;
                            }
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
                    if *show_pins_popup {
                        // Switch to categories side
                        category_state.show_categories = true;
                    } else {
                        let _ = selections.1.up();
                        selections.2 = selections.0;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Right
                    || key.code == KeyCode::Char('l')
                {
                    if *show_pins_popup {
                        // Switch to entries side
                        category_state.show_categories = false;
                    } else {
                        match &selections.3 {
                            Entry::dir(_d) => {
                                // if the selected is a dir enter it
                                    let _ = selections.1.down(selections.2);
                                    selections.0 = selections.2;
                                    selections.2 = 0;
                            }
                            Entry::file(f) => {
                               let _ =  open_in_default(f.path.as_path());
                            }
                            Entry::None => {}
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Tab {
                    if *show_pins_popup {
                        // Switch between categories and entries
                        category_state.show_categories = !category_state.show_categories;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter
                    || key.code == KeyCode::Char('S')
                {
                    if *show_pins_popup {
                        if category_state.show_categories {
                            // Switch to entries side when Enter pressed on category
                            category_state.show_categories = false;
                        } else {
                            // Navigate to selected pin and close popup
                            if !category_state.entries_in_category.is_empty() && category_state.selected_entry < category_state.entries_in_category.len() {
                                match &category_state.entries_in_category[category_state.selected_entry].entry {
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
                        }
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
                    if !buffer_state.1.is_empty() {
                        *show_progress_popup = true; // show progress popup
                        start_background_paste(buffer_state.1.clone(), selections.1.path.clone(), paste_progress.clone());
                        buffer_state.1.clear(); //clear the buffer
                    }
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
                        if !category_manager.pin_exists(&pin_entry) {
                            category_manager.add_pin(pin_entry, "Default".to_string());
                            let _ = save_pins(category_manager);
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('u') {
                    // unpin current selection
                    if !matches!(selections.3, Entry::None) {
                        let path = match &selections.3 {
                            Entry::dir(d) => &d.path,
                            Entry::file(f) => &f.path,
                            Entry::None => return Ok(false),
                        };
                        if category_manager.remove_pin_by_path(path) {
                            let _ = save_pins(category_manager);
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('o') {
                    // toggle pins popup
                    *show_pins_popup = !*show_pins_popup;
                    if *show_pins_popup {
                        let categories = category_manager.get_categories();
                        if !categories.is_empty() {
                            category_state.selected_category = 0;
                            category_state.entries_in_category = category_manager.get_entries_in_category(&categories[0]);
                            category_state.selected_entry = 0;
                            category_state.show_categories = true;
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('/') {
                    if *show_pins_popup {
                        // Start search in pins popup
                        category_state.search_mode = true;
                        category_state.search_query.clear();
                    } else {
                        //search current dir
                        *input_state.0 = true;
                        *input_state.2 = 4;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('c') && *show_pins_popup {
                    if !category_state.show_categories && !category_state.entries_in_category.is_empty() && category_state.selected_entry < category_state.entries_in_category.len() {
                        let selected_entry = &category_state.entries_in_category[category_state.selected_entry];
                        if let Some(pin_idx) = category_manager.find_pin_index_by_entry(&selected_entry.entry) {
                            category_state.entry_to_change = Some(pin_idx);
                            category_state.category_input_mode = true;
                            category_state.category_input = category_manager.get_pins()[pin_idx].category.clone();
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('i') {
                    // toggle file info popup for selected entry
                    if !matches!(selections.3, Entry::None) {
                        *show_file_info_popup = !*show_file_info_popup;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Esc {
                    // close pins popup or progress popup or file info popup
                    if *show_pins_popup {
                        *show_pins_popup = false;
                        category_state.search_mode = false;
                        category_state.search_query.clear();
                    } else if *show_progress_popup {
                        *show_progress_popup = false;
                    } else if *show_file_info_popup {
                        *show_file_info_popup = false;
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('U') {
                    // remove selected pin from pins list (when in popup)
                    if *show_pins_popup && !category_state.show_categories && !category_state.entries_in_category.is_empty() && category_state.selected_entry < category_state.entries_in_category.len() {
                        let selected_entry = &category_state.entries_in_category[category_state.selected_entry];
                        let path = match &selected_entry.entry {
                            Entry::dir(d) => &d.path,
                            Entry::file(f) => &f.path,
                            Entry::None => return Ok(false),
                        };
                        
                        if category_manager.remove_pin_by_path(path) {
                            let _ = save_pins(category_manager);
                            
                            let categories = category_manager.get_categories();
                            if category_state.selected_category < categories.len() {
                                category_state.entries_in_category = category_manager.get_entries_in_category(&categories[category_state.selected_category]);
                            } else if !categories.is_empty() {
                                category_state.selected_category = 0;
                                category_state.entries_in_category = category_manager.get_entries_in_category(&categories[0]);
                            }
                            
                            if category_state.selected_entry >= category_state.entries_in_category.len() && !category_state.entries_in_category.is_empty() {
                                category_state.selected_entry = category_state.entries_in_category.len() - 1;
                            } else if category_state.entries_in_category.is_empty() {
                                category_state.selected_entry = 0;
                            }
                        }
                    }
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('m') {
                    // change permissions
                    *input_state.0 = true;
                    *input_state.2 = 5;
                }
                if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('T') {
                    // toggle progress popup
                    *show_progress_popup = !*show_progress_popup;
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
        update(selections, contains, image_preview);
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
        5 => {
            // change permissions
            if let Ok(permissions) = u32::from_str_radix(&input_state.1, 8) {
                match &selections.3 {
                    Entry::dir(d) => {
                        let _ = d.perm_ch(permissions);
                    }
                    Entry::file(f) => {
                        let _ = f.perm_ch(permissions);
                    }
                    Entry::None => {}
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn get_file_info(entry: &Entry) -> Vec<String> {
    let mut info = Vec::new();
    
    match entry {
        Entry::file(f) => {
            let path = &f.path;
            info.push(format!("File: {}", f.name));
            info.push(format!("Path: {}", path.display()));
            
            if let Ok(metadata) = fs::metadata(path) {
                // File size
                info.push(format!("Size: {} ({})", format_bytes(metadata.len()), metadata.len()));
                
                // File type and permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    info.push(format!("Permissions: {:o}", mode & 0o777));
                    info.push(format!("Mode: {:o}", mode));
                }
                
                // Timestamps
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Modified: {}", format_timestamp(timestamp)));
                    }
                }
                
                if let Ok(accessed) = metadata.accessed() {
                    if let Ok(duration) = accessed.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Accessed: {}", format_timestamp(timestamp)));
                    }
                }
                
                if let Ok(created) = metadata.created() {
                    if let Ok(duration) = created.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Created: {}", format_timestamp(timestamp)));
                    }
                }
                
                // Additional Unix-specific metadata
                #[cfg(unix)]
                {
                    info.push(format!("Inode: {}", metadata.ino()));
                    info.push(format!("Device: {}", metadata.dev()));
                    info.push(format!("Hard links: {}", metadata.nlink()));
                    info.push(format!("UID: {}", metadata.uid()));
                    info.push(format!("GID: {}", metadata.gid()));
                    info.push(format!("Blocks: {}", metadata.blocks()));
                    info.push(format!("Block size: {}", metadata.blksize()));
                }
                
                // File type
                info.push(format!("Is symlink: {}", metadata.is_symlink()));
                info.push(format!("Read only: {}", metadata.permissions().readonly()));
            } else {
                info.push("Could not read file metadata".to_string());
            }
        }
        Entry::dir(d) => {
            let path = &d.path;
            info.push(format!("Directory: {}", d.name));
            info.push(format!("Path: {}", path.display()));
            
            if let Ok(metadata) = fs::metadata(path) {
                // Directory permissions
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode();
                    info.push(format!("Permissions: {:o}", mode & 0o777));
                    info.push(format!("Mode: {:o}", mode));
                }
                
                // Timestamps
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Modified: {}", format_timestamp(timestamp)));
                    }
                }
                
                if let Ok(accessed) = metadata.accessed() {
                    if let Ok(duration) = accessed.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Accessed: {}", format_timestamp(timestamp)));
                    }
                }
                
                if let Ok(created) = metadata.created() {
                    if let Ok(duration) = created.duration_since(UNIX_EPOCH) {
                        let timestamp = duration.as_secs();
                        info.push(format!("Created: {}", format_timestamp(timestamp)));
                    }
                }
                
                // Additional Unix-specific metadata
                #[cfg(unix)]
                {
                    info.push(format!("Inode: {}", metadata.ino()));
                    info.push(format!("Device: {}", metadata.dev()));
                    info.push(format!("Hard links: {}", metadata.nlink()));
                    info.push(format!("UID: {}", metadata.uid()));
                    info.push(format!("GID: {}", metadata.gid()));
                }
                
                // Directory contents count
                if let Ok(entries) = fs::read_dir(path) {
                    let count = entries.count();
                    info.push(format!("Items: {}", count));
                }
                
                info.push(format!("Read only: {}", metadata.permissions().readonly()));
            } else {
                info.push("Could not read directory metadata".to_string());
            }
        }
        Entry::None => {
            info.push("No item selected".to_string());
        }
    }
    
    info
}

fn format_timestamp(timestamp: u64) -> String {
    use std::process::Command;
    
    // Try to use the `date` command to format the timestamp
    if let Ok(output) = Command::new("date")
        .arg("-d")
        .arg(&format!("@{}", timestamp))
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
    {
        if output.status.success() {
            if let Ok(formatted) = String::from_utf8(output.stdout) {
                return formatted.trim().to_string();
            }
        }
    }
    
    // Fallback to basic timestamp if date command fails
    format!("{} (Unix timestamp)", timestamp)
}

fn ui(
    frame: &mut Frame,
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    constants: &mut (Vec<PathBuf>, Vec<String>),
    input_state: &mut (&mut bool, &mut String, &mut usize),
    buffer_state: &mut (usize, &mut Vec<(Entry, bool)>),
    category_manager: &mut CategoryManager,
    category_state: &mut CategoryState,
    show_pins_popup: &mut bool,
    paste_progress: &Arc<Mutex<PasteProgress>>,
    show_progress_popup: &mut bool,
    show_file_info_popup: &mut bool,
    image_preview: &mut ImagePreview,
) {
    let commands = vec![
        Row::new([
            "('D' , Delete  : remove)",
            "('y'   :  copy )",
            "('x'   :  remove from buffer )",
            "('S'   :  open shell in dir )",
            "('P'   :  pin )",
            "('i'   :  file info )"

        ]),
        Row::new([
            "('a'   : add file)",
            "('d'   :  cut )",
            "('w'   :  buffer up )",
            "('q'   :  quit )",
            "('u'   :  unpin )",
            "('T'   :  toggle progress popup)"
        ]),
        Row::new([
            "('A'   : add dir)",
            "('p'   :  paste )",
            "('s'   :  buffer down )",
            "('Arrows'   :  movments )",
            "('o'   :  pins popup )",
        ]),
        Row::new(["('r'   :  rename )", "","", "('/'  :  search )" , "('U'   :  remove pin )"]),
        Row::new(["('m'   :  change permissions )" , "" , "" ,  "('N'   :  prev search)" , "('c'   :  change category in pins)"]),
        Row::new(["" , "" , "", "('n'   :  next search)"]),
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
    .split(frame.area());
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
    sel.select(Some(selections.2)); // Select the first item initially
    prev_sel.select(Some(selections.0)); // Select the index of curr in prev
    buffer_sel.select(Some(buffer_state.0)); // Selection of buffer
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
            // Check if the file is an image and we have a preview
            if let  Some(image_path) = image_preview.image_path.clone(){
                if f.path == image_path && image_preview.image.is_some() {
                    // Render image preview
                    render_image_preview(frame, inner_layout[2], image_preview);
                }
            }
            else if let Ok(contents) = f.read() {
                // Render text content for non-image files
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

    let input_title = match *input_state.2 {
        1 => "Rename",
        2 => "Add File",
        3 => "Add Directory",
        4 => "Search",
        5 => "Change Permissions (octal, e.g., 755)",
        _ => "Input",
    };

    frame.render_widget(
        // the text input box
        Paragraph::new(input_state.1.to_owned()).block(
            Block::default().title(input_title).borders(Borders::ALL).style(
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
        render_pins_popup(frame, category_manager, category_state);
    }

    // Render file info popup if enabled
    if *show_file_info_popup {
        render_file_info_popup(frame, selections);
    }

    // Render paste progress if toggled
    if let Ok(progress) = paste_progress.lock() {
        if *show_progress_popup {
            render_paste_progress(frame, &progress);
        }
    }
}

fn update(
    selections: &mut (usize, &mut dirs::Directory, usize, Entry),
    contains: &mut (Vec<PathBuf>, Vec<String>),
    image_preview: &mut ImagePreview,
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
                image_preview.clear(); // Clear image preview for directories
            } else if path.is_file() {
                if let Ok(new) = files::File::new(path) {
                    selections.3 = Entry::file(new); // setting it to the file
                    // Try to load image preview if it's an image file
                    if let Some(image_path) = &image_preview.image_path {
                        if image_path != path {
                            let _ = image_preview.load_image(path);
                        }
                        
                    }
                    else {
                        let _ = image_preview.load_image(path);
                    }
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
    let widths: &[Constraint; 6] = &[
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
        Constraint::Percentage(16),
    ];

    // Render the table with the provided data
    frame.render_stateful_widget(
        Table::new(data.to_vec(), widths) // Convert array to Vec
            .style(Style::new().blue().fg(Color::White).bg(Color::Black))
            .block(Block::default().title("commands").borders(Borders::ALL))
            .header(
                Row::new(vec!["alter", "ccp", "buffer", "controls" , "pin" , "info"]).style(
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

fn render_pins_popup(frame: &mut Frame, category_manager: &CategoryManager, category_state: &mut CategoryState) {
    let popup_area = centered_rect(80, 60, frame.area());
    
    // Clear the area first
    frame.render_widget(Clear, popup_area);
    
    // Create layout for categories and entries
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(popup_area);
    
    // Categories side
    let mut categories_display: Vec<String> = Vec::new();
    let categories = category_manager.get_categories();
    if categories.is_empty() {
        categories_display.push("No categories".to_string());
    } else {
        for category in categories {
            let count = category_manager.get_category_count(category);
            categories_display.push(format!("{} ({})", category, count));
        }
    }

    let mut categories_list_state = ListState::default();
    if !categories.is_empty() && category_state.selected_category < categories.len() {
        categories_list_state.select(Some(category_state.selected_category));
    }

    // Render categories list
    frame.render_stateful_widget(
        List::new(categories_display)
            .block(
                Block::default()
                    .title("Categories")
                    .borders(Borders::ALL)
                    .border_style(if category_state.show_categories {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    })
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Black)
                    .bg(if category_state.show_categories { Color::Cyan } else { Color::DarkGray })
            )
            .highlight_symbol("→ ")
            .repeat_highlight_symbol(true),
        main_layout[0],
        &mut categories_list_state,
    );

    // Entries side
    let mut entries_display: Vec<String> = Vec::new();
    if category_state.entries_in_category.is_empty() {
        if category_state.search_mode && !category_state.search_query.is_empty() {
            entries_display.push(format!("No entries found for '{}'", category_state.search_query));
        } else {
            entries_display.push("No entries in this category".to_string());
        }
    } else {
        for entry in &category_state.entries_in_category {
            let entry_name = match &entry.entry {
                Entry::dir(d) => format!("📁 {} => {}", d.name, d.path.display()),
                Entry::file(f) => format!("📄 {} => {}", f.name, f.path.display()),
                Entry::None => "".to_string(),
            };
            entries_display.push(entry_name);
        }
    }

    let mut entries_list_state = ListState::default();
    if !category_state.entries_in_category.is_empty() && category_state.selected_entry < category_state.entries_in_category.len() {
        entries_list_state.select(Some(category_state.selected_entry));
    }

    // Create the title for entries with search info and input
    let entries_title = if category_state.search_mode {
        format!("Entries - Search: {}", category_state.search_query)
    } else if category_state.category_input_mode {
        format!("Entries - Change Category: {}", category_state.category_input)
    } else {
        "Entries".to_string()
    };

    // Render entries list
    frame.render_stateful_widget(
        List::new(entries_display)
            .block(
                Block::default()
                    .title(entries_title)
                    .borders(Borders::ALL)
                    .border_style(if !category_state.show_categories || category_state.search_mode || category_state.category_input_mode {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    })
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Black)
                    .bg(if !category_state.show_categories || category_state.search_mode || category_state.category_input_mode { 
                        Color::Cyan 
                    } else { 
                        Color::DarkGray 
                    })
            )
            .highlight_symbol("→ ")
            .repeat_highlight_symbol(true),
        main_layout[1],
        &mut entries_list_state,
    );

    // Add help text at the bottom
    let help_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(popup_area);
    
    let help_text = if category_state.category_input_mode {
        "Enter: Confirm category change | Esc: Cancel"
    } else if category_state.search_mode {
        "Type to search | Enter: Exit search | Esc: Cancel search"
    } else {
        "Tab/h/l: Switch sides | /: Search | c: Change category | Enter: Navigate | U: Remove | Esc: Close"
    };
    
    frame.render_widget(
        Paragraph::new(help_text)
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Yellow)),
        help_area[1],
    );
}

fn render_file_info_popup(frame: &mut Frame, selections: &mut (usize, &mut dirs::Directory, usize, Entry)) {
    let popup_area = centered_rect(70, 80, frame.area());
    
    // Clear the area first
    frame.render_widget(Clear, popup_area);
    
    // Get file information
    let file_info = get_file_info(&selections.3);
    
    // Create list items from file info
    let info_items: Vec<ListItem> = file_info
        .iter()
        .map(|info| ListItem::new(Text::from(info.as_str())))
        .collect();
    
    // Create the main list
    let info_list = List::new(info_items)
        .block(
            Block::default()
                .title("File Information")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan))
        )
        .style(Style::default().fg(Color::White).bg(Color::Black));
    
    // Create help text
    let help_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(popup_area);
    
    // Render the file info list
    frame.render_widget(info_list, help_layout[0]);
    
    // Render help text
    frame.render_widget(
        Paragraph::new("Press 'i' or 'Esc' to close")
            .block(Block::default().borders(Borders::TOP))
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center),
        help_layout[1],
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

fn start_background_paste(
    buffer: Vec<(Entry, bool)>,
    dest_dir: PathBuf,
    progress: Arc<Mutex<PasteProgress>>,
) {
    thread::spawn(move || {
        // Calculate total work
        let mut total_files = 0;
        let mut total_bytes = 0;
        
        for (entry, _) in &buffer {
            match entry {
                Entry::file(f) => {
                    total_files += 1;
                    if let Ok(metadata) = f.path.metadata() {
                        total_bytes += metadata.len();
                    }
                }
                Entry::dir(d) => {
                    if let Ok((files, bytes)) = count_dir_contents(&d.path) {
                        total_files += files;
                        total_bytes += bytes;
                    }
                }
                Entry::None => {}
            }
        }

        // Initialize progress
        {
            let mut prog = progress.lock().unwrap();
            prog.total_files = total_files;
            prog.total_bytes = total_bytes;
            prog.files_done = 0;
            prog.bytes_done = 0;
            prog.is_active = true;
        }

        // Process each entry
        for (entry, operation) in buffer {
            match entry {
                Entry::dir(d) => {
                    if operation {
                        // Move directory
                        let _ = background_move_dir(&d, &dest_dir, &progress);
                    } else {
                        // Copy directory
                        let _ = background_copy_dir(&d, &dest_dir, &progress);
                    }
                }
                Entry::file(f) => {
                    if operation {
                        // Move file
                        let _ = background_move_file(&f, &dest_dir, &progress);
                    } else {
                        // Copy file
                        let _ = background_copy_file(&f, &dest_dir, &progress);
                    }
                }
                Entry::None => {}
            }
        }

        // Mark as complete
        {
            let mut prog = progress.lock().unwrap();
            prog.is_active = false;
            prog.current_file.clear();
        }
    });
}

fn count_dir_contents(path: &Path) -> io::Result<(usize, u64)> {
    let mut file_count = 0;
    let mut total_bytes = 0;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            file_count += 1;
            if let Ok(metadata) = path.metadata() {
                total_bytes += metadata.len();
            }
        } else if path.is_dir() {
            let (sub_files, sub_bytes) = count_dir_contents(&path)?;
            file_count += sub_files;
            total_bytes += sub_bytes;
        }
    }

    Ok((file_count, total_bytes))
}

fn background_copy_file(
    file: &files::File,
    dest_dir: &PathBuf,
    progress: &Arc<Mutex<PasteProgress>>,
) -> Result<(), io::Error> {
    let dest_path = dest_dir.join(&file.name);
    
    // Update progress with current file
    {
        let mut prog = progress.lock().unwrap();
        prog.current_file = file.name.clone();
    }

    // Copy file with progress tracking
    let mut source = fs::File::open(&file.path)?;
    let mut destination = fs::File::create(&dest_path)?;
    
    let mut buffer = vec![0; 8192]; // 8KB buffer
    
    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        
        destination.write_all(&buffer[..bytes_read])?;
        
        // Update progress
        {
            let mut prog = progress.lock().unwrap();
            prog.bytes_done += bytes_read as u64;
        }
    }

    // Update file completion
    {
        let mut prog = progress.lock().unwrap();
        prog.files_done += 1;
    }

    Ok(())
}

fn background_move_file(
    file: &files::File,
    dest_dir: &PathBuf,
    progress: &Arc<Mutex<PasteProgress>>,
) -> Result<(), io::Error> {
    let dest_path = dest_dir.join(&file.name);
    
    // Update progress with current file
    {
        let mut prog = progress.lock().unwrap();
        prog.current_file = file.name.clone();
    }

    // Try to rename first (fast move within same filesystem)
    match fs::rename(&file.path, &dest_path) {
        Ok(_) => {
            // Update progress
            let mut prog = progress.lock().unwrap();
            prog.files_done += 1;
            if let Ok(metadata) = dest_path.metadata() {
                prog.bytes_done += metadata.len();
            }
            Ok(())
        }
        Err(_) => {
            // Fall back to copy + delete
            background_copy_file(file, dest_dir, progress)?;
            fs::remove_file(&file.path)?;
            Ok(())
        }
    }
}

fn background_copy_dir(
    source_dir: &dirs::Directory,
    dest_dir: &PathBuf,
    progress: &Arc<Mutex<PasteProgress>>,
) -> Result<(), io::Error> {
    let new_dir_path = dest_dir.join(&source_dir.name);
    fs::create_dir_all(&new_dir_path)?;

    for entry in fs::read_dir(&source_dir.path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Ok(file) = files::File::new(&path) {
                background_copy_file(&file, &new_dir_path, progress)?;
            }
        } else if path.is_dir() {
            if let Ok(dir) = dirs::Directory::new(&path) {
                background_copy_dir(&dir, &new_dir_path, progress)?;
            }
        }
    }

    Ok(())
}

fn background_move_dir(
    source_dir: &dirs::Directory,
    dest_dir: &PathBuf,
    progress: &Arc<Mutex<PasteProgress>>,
) -> Result<(), io::Error> {
    let new_dir_path = dest_dir.join(&source_dir.name);
    
    // Check if we're trying to move into self
    if dest_dir.starts_with(&source_dir.path) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Cannot move directory into itself"));
    }

    // Try to rename first (fast move within same filesystem)
    match fs::rename(&source_dir.path, &new_dir_path) {
        Ok(_) => {
            // Update progress for all files in the moved directory
            if let Ok((files, bytes)) = count_dir_contents(&new_dir_path) {
                let mut prog = progress.lock().unwrap();
                prog.files_done += files;
                prog.bytes_done += bytes;
            }
            Ok(())
        }
        Err(_) => {
            // Fall back to copy + delete
            background_copy_dir(source_dir, dest_dir, progress)?;
            fs::remove_dir_all(&source_dir.path)?;
            Ok(())
        }
    }
}

fn render_paste_progress(frame: &mut Frame, progress: &PasteProgress) {
    // Create a progress popup area
    let progress_area = centered_rect(70, 25, frame.area());
    
    // Clear the area first
    frame.render_widget(Clear, progress_area);
    
    // Check if operation is active
    if !progress.is_active && progress.total_files == 0 {
        // Show "no operation" message when manually opened
        frame.render_widget(
            Paragraph::new("No paste operation in progress.\n\nPress 'p' to start pasting files from buffer.\nPress 'T' to close this window.")
                .block(
                    Block::default()
                        .title("Paste Progress (No Operation)")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow))
                )
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            progress_area,
        );
        return;
    }
    
    // Calculate progress percentages
    let file_progress = if progress.total_files > 0 {
        (progress.files_done as f64 / progress.total_files as f64 * 100.0) as u16
    } else {
        0
    };
    
    let byte_progress = if progress.total_bytes > 0 {
        (progress.bytes_done as f64 / progress.total_bytes as f64 * 100.0) as u16
    } else {
        0
    };

    // Create layout for progress bars
    let progress_layout = Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3), // Current file
            Constraint::Length(3), // File progress bar
            Constraint::Length(3), // Byte progress bar
            Constraint::Min(0), // Status/instructions
        ],
    )
    .split(progress_area);

    // Current file info
    let current_file_text = if progress.current_file.is_empty() {
        if progress.is_active {
            "Preparing...".to_string()
        } else {
            "Operation completed".to_string()
        }
    } else {
        format!("Processing: {}", progress.current_file)
    };
    
    let title = if progress.is_active {
        "Paste Operation (In Progress)"
    } else {
        "Paste Operation (Completed)"
    };
    
    frame.render_widget(
        Paragraph::new(current_file_text)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if progress.is_active { Color::Green } else { Color::Blue }))
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center),
        progress_layout[0],
    );

    // File progress bar
    frame.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .title(format!("Files: {}/{}", progress.files_done, progress.total_files))
                    .borders(Borders::ALL)
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(file_progress)
            .label(format!("{}%", file_progress)),
        progress_layout[1],
    );

    // Byte progress bar
    let bytes_text = format!(
        "Data: {}/{}",
        format_bytes(progress.bytes_done),
        format_bytes(progress.total_bytes)
    );
    
    frame.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .title(bytes_text)
                    .borders(Borders::ALL)
            )
            .gauge_style(Style::default().fg(Color::Green))
            .percent(byte_progress)
            .label(format!("{}%", byte_progress)),
        progress_layout[2],
    );

    // Instructions
    let instructions = if progress.is_active {
        "Press 'Esc' or 'T' to close this window"
    } else {
        "Operation completed! Press 'Esc' or 'T' to close"
    };
    
    frame.render_widget(
        Paragraph::new(instructions)
            .block(
                Block::default()
                    .borders(Borders::ALL)
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center),
        progress_layout[3],
    );
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn is_image_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            let ext_lower = ext_str.to_lowercase();
            matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif" | "ico" | "svg")
        } else {
            false
        }
    } else {
        false
    }
}

fn render_image_preview(frame: &mut Frame, rect: Rect, image_preview: &mut ImagePreview) {
    if let Some(ref mut protocol) = image_preview.image {
        // Create the image widget
        let image_widget = StatefulImage::default();
        
        // Render the image with a border and title
        let image_block = Block::default()
            .title("Image Preview")
            .borders(Borders::ALL)
            .style(Style::new().blue().fg(Color::White).bg(Color::Black));
        
        let inner_rect = image_block.inner(rect);
        frame.render_widget(image_block, rect);
        
        // Render the image within the bordered area
        frame.render_stateful_widget(image_widget, inner_rect, protocol);
        
        // Handle encoding result to avoid issues
        if let Some(Err(_)) = protocol.last_encoding_result() {
            // If encoding failed, show error message
            let error_text = Paragraph::new("Failed to render image")
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            frame.render_widget(error_text, inner_rect);
        }
    }
}
