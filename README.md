# `unf`

UNixize Filename -- replace annoying anti-unix characters in filenames

## About

Certain characters in filenames are problematic for command-line users. For example, `' '`, `'*'`, and `'|'` are treated specially by the shell, and `':'` is used as a path separator in environment variables. `unf` renames these files, so you no longer have to be annoyed when your Windows-using friend sends you an irritatingly-named zip file.

## Installing

### Using `cargo`

``` sh
cargo install unf
```

This installs to `~/.cargo/bin`, so make sure that's in your `PATH`.

## Usage

```
unf [FLAGS] <PATH>...
```

`<PATH>...`: The paths of filenames to unixize

`-r` `--recursive`: Recursively unixize filenames in directories. If some of the specified paths are directories, unf will operate recursively on their contents

`-d` `--dryrun`: Do not rename any files, but print all the renames that would happen

`-s` `--follow-symlinks`: Follow symbolic links

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
