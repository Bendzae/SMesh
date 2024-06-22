# SMesh

SMesh is a polygon mesh manipulation library based on the
[Surface Mesh](https://link.springer.com/chapter/10.1007/978-3-642-24734-7_29)
data structure and the [pmp](https://github.com/pmp-library/pmp-library)
library.

For now it is a rough port of the amazing
[pmp](https://github.com/pmp-library/pmp-library) library with a "rusty" api on
top, but I plan to add additional functionality. The libary uses a slotmap based
implementation of the Surface Mesh which takes heavy inspiration from
[blackjacks](https://github.com/setzer22/blackjack) halfedge-mesh
implementation.

### Examples

![screenshot](visualizer_screenshot.png)
_use the visualizer example app to interactively explore library features_

`cargo run --example visualizer`

There is also an example for the bevy mesh integration:

`cargo run --example bevy_mesh`

### Usage

Coming soon...
(Check examples for now)

### Goals

I aim to provide a flexible rust implementation of the Surface Mesh with a focus
on usefulness for procedural mesh generation. Other goals are:

- Ergonomic and easy-to-use api
- Port most operations from the pmp library
- Support most operations that are possible in modern 3D modeling software like
  blender
- Integration with the bevy game engine
- Target manifold tri & quad meshes for now
