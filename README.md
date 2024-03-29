# Unjosefizer

Unjosefizer is a Rust application that will load a `.3mf` file saved by Bambu Studio/Orca Slicer, and convert it to a `.3mf` that can be read by Prusaslicer, while maintaining the MMU painting.

## Caveats

- **Do not use on files you haven't backed up!**
- Essentially everything except meshes and MMU painting will be discarded

## Use

- Run the program
- Choose an output folder
- Add files with the file picker or by dragging and dropping to the window
- Click "Process"
- The files will be renamed from `name.3mf` to `name_ps.3mf`

## Running

To build, you will need [Rust](https://www.rust-lang.org/tools/install) installed.
To run, `git clone` or download this repository, and run
```
cargo build --release
```

## If this is helpful, consider buying me a coffee

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I3I1W8O4I)
