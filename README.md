# fileio
a command line file manager written in rust
for easier command line navigation 
-----

![Screenshot from 2024-05-15 06-02-03](https://github.com/philopaterwaheed/fileio/assets/61416026/2bccdae1-33ba-489c-90b5-ad679b0231f9)


## what makes us different 
- ### a shared buffer betwwen copy and cut
    you can copy some files and cut some files and paste them at the same place


https://github.com/philopaterwaheed/fileio/assets/61416026/4d70028c-463f-4f9d-9697-00beac15a45f



- ### a flexable buffer
  you can delete and add as much as you want from the paste buffer <br>
  which gives you the abslute control on what you will paste now if you made a mistake
  
https://github.com/philopaterwaheed/fileio/assets/61416026/c6ef11d2-6742-44ac-b7f8-5e6d7bacca84

## Controls

| Shortcut | Description             |
|----------|-------------------------|
| `q` | Quit|
| `y` | Copy selected text|
| `d` | Cut selected text|
| `p` | Paste copied/cut text|
| `w` | Buffer up|
| `s` | Buffer Down|
| `x` | Delete from buffer|
|  `→` | enter if dir open in defulat app if file|
| `←` | up in dirs tree|
| `↑` | Selection Up|
| `↓` | Selection down|
| `S` | Open dir in shell|
| `a` | Touch a file|
| `A` | Crate a  dir|
| `r` | Rename|
| `D` | Delete|
| `/` | Search|
| `n` | next search|
| `N` | prev search|
| `P` | Pin selected file/directory|
| `u` | Unpin selected file/directory|
| `o` | Open pins popup|
| `U` | Remove selected pin (when in popup)|

## Pin Feature

The pin feature allows you to bookmark important directories and files for quick navigation using a popup window. **Pins are automatically saved and persist between application runs.**

- **Pin an item**: Press `P` when a file or directory is selected to add it to your pins
- **Open pins popup**: Press `o` to open the pins popup window
- **Navigate in popup**: Use arrow keys (↑/↓) to navigate through pinned items
- **Go to pin**: Press `Enter` while in the popup to navigate to the selected pin
- **Close popup**: Press `Esc` to close the pins popup
- **Unpin item**: Press `u` to remove the currently selected item from pins (if it's pinned)
- **Remove pin**: Press `U` while in the popup to remove the selected pin from the list

The popup displays directories with 📁 and files with 📄 icons, making it easy to identify pin types at a glance.

### Persistence
Pins are automatically saved to `~/.fileio_pins` (or `.fileio_pins` in the current directory if `$HOME` is not available). The file stores the full paths of pinned items, and upon startup, the application automatically loads saved pins. If a pinned path no longer exists, it will be silently ignored during loading.

## Get started 
### dependencies
make sure that you have 
`xdp-open` to open files in their defulat app
### simple way
you can simply download the executable from the releases and then move it to /bin or any other folders that are in the path
```
sudo mv fileio /bin
```
