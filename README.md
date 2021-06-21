[![crates.io](https://img.shields.io/crates/v/unf)](https://crates.io/crates/unf)
[![tests](https://github.com/io12/unf/workflows/tests/badge.svg)](https://github.com/io12/unf/actions?query=workflow%3Atests)
[![cargo-audit](https://github.com/io12/unf/workflows/cargo-audit/badge.svg)](https://github.com/io12/unf/actions?query=workflow%3Acargo-audit)
[![Coverage Status](https://coveralls.io/repos/github/io12/unf/badge.svg?branch=master)](https://coveralls.io/github/io12/unf?branch=master)

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
rename '🤔😀😃😄😁😆😅emojis.txt' -> 'thinking_grinning_smiley_smile_grin_laughing_sweat_smile_emojis.txt'? (y/N): y
```

``` sh
$ unf -f 'Game (Not Pirated 😉).rar'
rename 'Game (Not Pirated 😉).rar' -> 'Game_Not_Pirated_wink.rar'
```

### Recursion

``` sh
$ unf -rf My\ Files/ My\ Folder
rename 'My Files/Passwords :) .txt' -> 'My Files/Passwords.txt'
rename 'My Files/Another Cool Photo.JPG' -> 'My Files/Another_Cool_Photo.JPG'
rename 'My Files/Wow Cool Photo.JPG' -> 'My Files/Wow_Cool_Photo.JPG'
rename 'My Files/Cool Photo.JPG' -> 'My Files/Cool_Photo.JPG'
rename 'My Files/' -> 'My_Files'
rename 'My Folder' -> 'My_Folder'
```

### Collisions

``` sh
$ unf -f -- --fake-flag.txt fake-flag.txt ------fake-flag.txt ' fake-flag.txt' $'\tfake-flag.txt'
rename '--fake-flag.txt' -> 'fake-flag_000.txt'
rename '------fake-flag.txt' -> 'fake-flag_001.txt'
rename ' fake-flag.txt' -> 'fake-flag_002.txt'
rename '	fake-flag.txt' -> 'fake-flag_003.txt'
```

## FAQ

### Is this useful?

Hopefully for some people. There are certain situations in which I believe this tool is useful.

- Downloading files uploaded by non-CLI users, especially large archives with poorly-named files
- The ` (1)` that gets appended to web browser download duplicates
- Unix tools which take advantage of the loose Unix filename restrictions (like `youtube-dl`, which creates filenames from the video title)

### How does this handle collisions?

Since `unf` is an automatic batch rename tool, there may be cases where the path to the unixized filename already exists. `unf` resolves this crisis by appending and incrementing a zero-padded number to the end of the file stem. An example of this is displayed [here](#collisions).

### Why is the collision-resolving number zero-padded?

It has the nice property of being ordered when using tools that sort filenames by ASCII values, such as `ls` and shell completion.

### Why not just use shell completion to access problematic filenames?

Shell completion can automatically insert backslash escapes, but this is sub-optimal. The backslash escapes make the filenames substantially less readable. However, shell completion is great for invoking `unf`.
