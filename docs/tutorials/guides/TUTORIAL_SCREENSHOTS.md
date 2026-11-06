# Take pictures of the windows with the picture mode of phoenix

`phoenix --shot FILE.png` opens the window, waits a few frames, saves a picture of it as a PNG file and closes by itself. It is how the pictures of the README are made. In this mode the window reads and writes no preferences and no list of recent files, so a run leaves your settings as they were. `phoenix` is the program of the window, beside `phoenix` in the release archive; it is built with the `gui` feature.

Run the commands from the root of the repository; see [how to run the guides](README.md).

1. Take the start screen, with a size, a theme and a number of frames to wait. The size is `WIDTHxHEIGHT` in pixels, each side from 320 to 8192; the theme is `dark`, `light` or `contrast`.

```console
phoenix --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Take the window of a board: `--open` gives the file and the window shown is `document` by default. A board needs `--open`; the start screen cannot have one.

```console
phoenix --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Take another window with `--window`: `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` or `help` (`3d`, `panel` and `gerber` need `--open`). `--lang` sets the language of the window.

```console
phoenix --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Take the whole set of pictures of a version with the tool of the repository. `--dry-run` prints the command lines and runs nothing; without it the tool starts the program once for each window, checks every PNG (it must not be empty or of one colour, and must weigh less than 400 KB) and writes the files `phoenix-pcb-NN-WINDOW.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix --out build/screens --dry-run
```

## How to read the result

* The picture is what the window drew, in physical pixels, so on a screen with scaling it can be larger than `--size`. The exit code is 0 when the picture was written, 1 when it was not and 2 for a mistake in the arguments. A file that exists is not written over unless `--replace` is given; without it the run stops before the window opens.
* Each run of the tool has its own settings folder inside a temporary folder, so your settings and recent files are not read or written. The taking of the picture itself has not been run on a screen yet, and no picture here is claimed to look right until you have seen it.

## See also

* [docs/VIEWER.md](../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../tools/make_shots.py)
