# Unjosefizer

Unjosefizer is a Rust application that will load a `.3mf` file saved by Bambu Studio/Orca Slicer, and convert it to a `.3mf` that can be read by Prusaslicer, while maintaining the MMU painting.

Has since become a general purpose `.3mf` manipulation utility.

## Caveats

- **Do not use on files you haven't backed up!**

## Usage

### Paint instancing

Bambu studio/Orca don't have instancing like Prusaslicer, so this aims to mimic that.

- Save a `.3mf` with Bambu/Orca that contains multiple identical objects with different painting
- Load the file in the "Paint Instancing" tab
- Choose an object to copy the paint from (You may want to rename the object in the slicer)
- Choose one or more objects to copy the paint onto
- Click "Apply" and wait for the program to unfreeze.

### Splitting models without losing the painting

Doesn't work with Bambu/Orca `.3mf` files for now.

- To do this, use Prusaslicer to save a 3mf containing two copies of the model:
  - One painted, that isn't split
  - One split, with no painting
- Load the file and process it under the "Splitting" tab
- The newly created file will contain the split model with the paint copied over

### 3mf converting

- Run the program
- Choose an output folder
- Add files with the file picker or by dragging and dropping to the window
- Click "Process"
- The files will be renamed from `name.3mf` to `name_ps.3mf`

## Building from source

To build, you will need [Rust](https://www.rust-lang.org/tools/install) installed.
To run, `git clone` or download this repository, and run
```
cargo build --release
```

## If this is helpful to you, consider buying me a coffee:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I3I1W8O4I)

