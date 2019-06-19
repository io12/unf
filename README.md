# `unf`

UNixize Filename -- replace annoying anti-unix characters in filenames

## About

Certain characters in filenames are problematic for command-line users. For example, spaces and parentheses are treated specially by the shell. `unf` renames these files, so you no longer have to be annoyed when your Windows-using friend sends you an irritatingly-named zip file.

## Installing

### Using `cargo`

``` sh
cargo install unf
```

This installs to `~/.cargo/bin`, so make sure that's in your `PATH`.

### Arch Linux

Install `unf` from the AUR.

## Usage

```
unf [FLAGS] <PATH>...
```

`<PATH>...`: The paths of filenames to unixize

`-r` `--recursive`: Recursively unixize filenames in directories. If some of the specified paths are directories, unf will operate recursively on their contents

`-f` `--force` Do not interactively prompt to rename each file

## Examples

``` sh
$ unf 🤔😀😃😄😁😆😅emojis.txt
rename '🤔😀😃😄😁😆😅emojis.txt' -> 'emojis.txt'? (y/N): y
```

``` sh
$ unf -f 'Game (Not Pirated 😉).rar'
rename 'Game (Not Pirated 😉).rar' -> 'Game_Not_Pirated.rar'
```

``` sh
$ unf -rf My\ Files/ My\ Folder
rename 'My Files/Passwords :) .txt' -> 'My Files/Passwords.txt'
rename 'My Files/Another Cool Photo.JPG' -> 'My Files/Another_Cool_Photo.JPG'
rename 'My Files/Wow Cool Photo.JPG' -> 'My Files/Wow_Cool_Photo.JPG'
rename 'My Files/Cool Photo.JPG' -> 'My Files/Cool_Photo.JPG'
rename 'My Files/' -> 'My_Files'
rename 'My Folder' -> 'My_Folder'
```

``` sh
$ unf -f -- --fake-flag.txt fake-flag.txt ------fake-flag.txt ' fake-flag.txt' $'\tfake-flag.txt'
rename '--fake-flag.txt' -> 'fake-flag_000.txt'
rename '------fake-flag.txt' -> 'fake-flag_001.txt'
rename ' fake-flag.txt' -> 'fake-flag_002.txt'
rename '	fake-flag.txt' -> 'fake-flag_003.txt'
```
