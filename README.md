# ferrolife

![Thumbnail](assets/thumbnail.png)

Live site: [ferrolife.czw.sh](https://ferrolife.czw.sh/)

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

| Key(s)                  | Action                                         |
| ----------------------- | ---------------------------------------------- |
| `W` / `Arrow Up`        | Pan camera up                                  |
| `A` / `Arrow Left`      | Pan camera left                                |
| `S` / `Arrow Down`      | Pan camera down                                |
| `D` / `Arrow Right`     | Pan camera right                               |
| `Shift` + movement keys | Increase camera pan speed while held           |
| `E`                     | Zoom in                                        |
| `Q`                     | Zoom out                                       |
| `Space`                 | Pause or resume simulation                     |
| `Esc`                   | Exit application                               |
| `U`                     | Open file picker and load an image board       |
| `B`                     | Save current board as PNG image (desktop only) |

## UI Controls

![UI Controls](assets/controls.png)

Open the control panel in the top-left corner.

| Control                  | Action                                                            |
| ------------------------ | ----------------------------------------------------------------- |
| `Pause/Resume`           | Toggle simulation playback                                        |
| `Upload Image`           | Open file picker and load image as initial board                  |
| `Save Board`             | Save the current board as a PNG image (desktop only)              |
| `Clear Board`            | Set all cells to black                                            |
| `Alive Threshold` slider | Set alive detection threshold (`0.0` to `1.0`, default `0.3`)     |
| `Cell Color` picker      | Choose the paint color used when activating cells (default white) |

### Mouse Interaction

| Input      | Action                                                          |
| ---------- | --------------------------------------------------------------- |
| Left click | Set the clicked cell alive using the selected `Live Cell Color` |

## Gallery

Circuit:

![](assets/circuit.png)

Football:

![](assets/football.png)

Logos:

![](assets/logos.png)

Matisse:

![](assets/matisse.png)

Metro:

![](assets/metro.png)

Gosper:

![](assets/gosper.png)

To reproduce these simulations, upload the corresponding image in `test/`
