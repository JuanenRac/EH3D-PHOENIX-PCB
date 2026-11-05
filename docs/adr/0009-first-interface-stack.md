# 0009: the first interface stack is egui on eframe with a canvas of our own (provisional)

Status: proposed. It becomes accepted, or is replaced, after the measurements listed below have been made on a real screen.

## Context

The roadmap (phase F6) asks for a short interface prototype and a final decision of the stack before the viewer and the editor are built. The candidates written in `docs/STATUS.md` were egui on wgpu with a custom canvas, iced, Slint, or everything on wgpu. The decision depends on how fluid a window stays with small, medium and large boards, which cannot be known from the data side alone.

Four programs of the suite (ISOCAM, STENCILCAM, GERBERVIEWER and PHOENIX-PNP) already use egui 0.37 on eframe, with the theme Forge, the icon of the program, system fonts for Japanese and Chinese and eight languages, so that part of the work is already known and shared.

## Decision (provisional)

The prototype uses **egui 0.37 on eframe (glow backend)** with a **canvas drawn by our own code** through the `Painter` of egui, and keeps everything that is not drawing outside egui:

* the primitives (line, arc, polygon, circle, text), the layers, the view, the selection by position with a spatial index, the highlight of a net and the measure live in the crate `eh3d-phoenix-viewer` without any dependency on egui, and are tested without a screen;
* the window (`gui` feature, off by default) is one file that draws that state and passes on the pointer, the wheel, the clicks, the drags and the keys; the editing of a board (tools, drags, shortcuts, undo, the findings of the rules, the ratsnest, the synchronisation with a schematic and the saving under another name) is state of the same crate, tested without a screen, and it works only through the reversible operations of the board crates;
* the benchmark of the data side (`phoenix view --bench`) measures the preparation of the primitives and the selection by position on boards of 1 000, 20 000 and 200 000 primitives.

## Reasons

* It is the stack of the sibling programs, so the theme, the icon, the fonts, the languages and the habits are shared and the prototype is cheap.
* An immediate-mode window fits a viewer whose picture changes with every wheel turn and every drag.
* A canvas of our own gives control of the order of the layers, the colours, the picking by our own index and the highlight of a net, which no ready-made widget gives.
* Because the logic does not know egui, changing the stack later costs one file and not the viewer.

## Risks

* The `Painter` of egui tessellates on the processor at every frame: with 200 000 primitives on the screen the frame may not stay fluid. What reduces it is already there (only the primitives that touch the window are listed) or is easy to add (skip what is smaller than a pixel, merge the lines of a layer into one mesh, keep the triangles of a polygon instead of cutting it at every frame; the window cuts it at every frame today).
* Text is drawn by egui; many designators and pin numbers at once may be slow, and the font for Japanese and Chinese comes from the system.
* No instancing and no custom shaders on the glow backend. If measurement shows a limit, the canvas can move to a callback on wgpu (`egui_wgpu`) fed by the same primitives, or the stack can change; the cost of the second is the window file.
* Accessibility and docking of panels are not tested; the window has fixed side panels. The editing that exists is small (one item at a time, a grid snap, two kinds of drag), and the complexity of a fuller editor (selection of many items, handles, copy and placing) is still ahead.
* An edit rebuilds the scene and the spatial index from the board. That is simple and always right, but on a large board it costs the whole preparation of the primitives at every step; if it misses the target below, the scene has to change in place (only the primitives of the items that changed).
* Linux needs the X11 and Wayland libraries at build time; the CI installs them for the lints of the window.

## What has to be measured on a real screen before this is accepted

On a laptop with integrated graphics and on a computer with a separate card, on Windows and Linux, at full screen on a 1080p and a 4K display:

* the frame time (median, 95th and 99th percentile) while the wheel zooms and while the pointer drags, with the small, medium and large boards of the benchmark, with every layer on and with one layer;
* the delay from a click to the highlighted net;
* the delay from the release of a drag, or a click of a tool, to the redrawn board (the scene is rebuilt), on the small, medium and large boards, and the delay of the answer of the rule check after an edit;
* the memory, the time to open and the time to the first frame;
* the same with the text of the designators on and off.

A proposal for the gate, to be confirmed with the first numbers: a 95th percentile frame time under 17 ms on the medium board and under 33 ms on the large board on a laptop with integrated graphics, and a click answered in under 100 ms. If the window misses it and cutting work does not fix it, this record is replaced by one that chooses the wgpu canvas or another stack.

## Consequences

Until then the viewer and the editing in its window are a prototype (PPCB-014 and PPCB-015 at `prototype`), nothing is promised about a fuller editor, and no other part of the repository depends on egui: the command line works without the `gui` feature. The geometry library for offsets and boolean operations is a separate decision.
