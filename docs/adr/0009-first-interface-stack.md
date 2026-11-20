# 0009: the interface stack is egui with wgpu inside the window frame of the suite

Status: accepted. It replaces the provisional choice of egui on eframe with the glow backend that this record held before; the file name keeps the number.

## Context

The roadmap (phase F6) asked for a short prototype and a final decision of the stack before the viewer and the editor grew. The candidates were egui on wgpu with a canvas of our own, iced, Slint, or everything on wgpu. Four programs of the suite (ISOCAM, STENCILCAM, GERBERVIEWER and PHOENIX-PNP) already use egui 0.37 on eframe, with the theme Forge, the icon of the program, system fonts for Japanese and Chinese and eight languages. EH3D-COMMON now also holds a window frame, `eh3d-dock`, with panels that can be docked, tabbed, split, floated, folded and locked, named workspaces saved in the settings, menus declared as data with shortcuts and recent files, and all of its words in the eight languages. A professional editor needs exactly that, and it is drawn with egui.

## Decision

The interface of the design tool is **egui 0.37 with the wgpu backend** (`eframe` with `wgpu`), with the panels, workspaces and menus of **`eh3d-dock`**, and a **canvas drawn by our own code**: the primitives of the board are sent to the graphics card through a callback of `egui_wgpu`, fed by the same list of primitives that the viewer prepares today, instead of being tessellated on the processor by the `Painter`.

Everything that is not drawing stays outside egui, as it is now:

* the primitives (line, arc, polygon, circle, text), the layers, the view, the selection by position with a spatial index, the highlight of a net and the measure live in the crate `eh3d-phoenix-viewer` without any dependency on egui, and are tested without a screen;
* the editing (tools, drags, shortcuts, undo, the findings of the rules, the ratsnest, the synchronisation with a schematic and the saving) is state of the same crate and works only through the reversible operations of the board crates;
* the window (`gui` feature, off by default) draws that state and passes on the pointer, the wheel, the clicks and the keys.

## Reasons

* It is the stack of the sibling programs and of the frame of the suite, so the theme, the icon, the fonts, the languages and the habits are shared and the window frame is not written again.
* An immediate-mode window fits a viewer whose picture changes with every wheel turn and every drag.
* The glow backend has no instancing and no custom shaders, and the `Painter` tessellates on the processor at every frame; with hundreds of thousands of primitives the frame would not stay fluid. A callback on wgpu removes both limits and is fed by the same primitives, so the cost of the change is the window file.
* A canvas of our own gives control of the order of the layers, the colours, the picking by our own index and the highlight of a net, which no ready-made widget gives.
* Because the logic does not know egui, changing the stack later would cost one file and not the viewer.

## Risks

* Text is drawn by egui; many designators and pin numbers at once may be slow, and the font for Japanese and Chinese comes from the system.
* wgpu needs a graphics driver that supports it; on a machine without one the window falls back to the software path of the platform or does not open, and the command line works without the `gui` feature.
* Accessibility (keyboard navigation, no signalling by colour alone) is a requirement of the frame and has to be tested in the window.
* An edit rebuilds the scene and the spatial index from the board. That is simple and always right, but on a large board it costs the whole preparation of the primitives at every step; if it misses the target below, the scene has to change in place.
* Linux needs the X11 and Wayland libraries at build time; the CI installs them for the lints of the window.

## What is still to be done, and how the choice is checked

The window is now laid out with `eh3d-dock`: ten panels (the board, the layers, the properties, the components, the nets, the problems of the rules, the library, the console, the report and the tasks), three workspaces by task (Edit, Review and Compact), the menu bar and the arrangement kept in the file of preferences (`docs/VIEWER.md`). It still draws with the painter of egui on the glow backend, behind one interface (`CanvasEngine`) chosen by the setting `canvas_engine`, so the callback of wgpu is another implementation of it. Moving the drawing to wgpu is open work, and until it is done and measured the viewer and the editing in its window stay a prototype (PPCB-014 and PPCB-015). The report panel keeps the last frame times (`frame_log_frames`) and shows the median and the 95th and 99th percentile, which are the numbers of the first measurement below; the verdict against the gate is `FrameStats::within` in `canvas_engine`. The choice is checked, not decided, by these measurements on a real screen, on a laptop with integrated graphics and on a computer with a separate card, on Windows and Linux, at 1080p and 4K:

* the frame time (median, 95th and 99th percentile) while the wheel zooms and the pointer drags, with the small, medium and large boards of the benchmark (`phoenix view --bench` measures the data side), with every layer on and with one layer;
* the delay from a click to the highlighted net, and from the release of a drag or the click of a tool to the redrawn board and the answer of the rule check;
* the memory, the time to open and the time to the first frame, with the text of the designators on and off.

The gate, to be confirmed with the first numbers: a 95th percentile frame time under 17 ms on the medium board and under 33 ms on the large board on integrated graphics, and a click answered in under 100 ms. If the window misses it and cutting work does not fix it, this record is replaced by one that names another stack.

## Consequences

No other part of the repository depends on egui: the command line works without the `gui` feature. The geometry library for offsets and boolean operations is a separate decision (`docs/adr/0007-geometry-library-and-degenerate-cases.md`).
