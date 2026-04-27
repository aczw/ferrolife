# ferrolife

## Build and Run

### Desktop

Run in debug mode:

```bash
cargo run
```

Build debug binaries:

```bash
cargo build
```

Build optimized release binaries:

```bash
cargo build --release
```

### WebAssembly (optional)

Build for `wasm32-unknown-unknown`:

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown
```

If you use `wasm-pack`, you can also run:

```bash
wasm-pack build --target web
```

## Keyboard Controls

These controls are currently implemented in the app.

| Key(s)                  | Action                                   |
| ----------------------- | ---------------------------------------- |
| `W` / `Arrow Up`        | Pan camera up                            |
| `A` / `Arrow Left`      | Pan camera left                          |
| `S` / `Arrow Down`      | Pan camera down                          |
| `D` / `Arrow Right`     | Pan camera right                         |
| `Shift` + movement keys | Increase camera pan speed while held     |
| `E`                     | Zoom in                                  |
| `Q`                     | Zoom out                                 |
| `Space`                 | Pause or resume simulation               |
| `Esc`                   | Exit application                         |
| `U` (desktop only)      | Open file picker and load an image board |

## UI Controls

Desktop shows an ImGui controls panel in the top-left corner.
Web builds show a minimal browser control bar in the top-left corner.

| Button                   | Action                                                          |
| ------------------------ | --------------------------------------------------------------- |
| `Pause/Resume`           | Toggle simulation playback                                      |
| `Upload Image`           | Open file picker and load image as initial board (desktop only) |
| `Alive Threshold` slider | Set alive detection threshold (`0.0` to `1.0`, default `0.3`)   |

## Notes

- Movement and zoom are continuous while keys are held.
- On desktop, you can also drag and drop an image file into the window to load a board.
- On web builds, image upload is not supported yet.
