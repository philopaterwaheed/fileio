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


## Get started 
### dependencies
make sure that you have 
`xdp-open` to open files in their defulat app
### simple way
you can simply download the executable from the releases and then move it to /bin or any other folders that are in the path
```
sudo mv fileio /bin
```
