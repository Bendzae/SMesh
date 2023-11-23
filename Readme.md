# SMesh

SMesh is a polygon mesh manipulation library based
on the [Surface Mesh](https://link.springer.com/chapter/10.1007/978-3-642-24734-7_29) data structure and
the [pmp](https://github.com/pmp-library/pmp-library) library.

For now it is basically a rust port of the amazing [pmp](https://github.com/pmp-library/pmp-library) library, but I hope to adapt
implementations and api to be more rust idiomatic as well as adding additional operations.
It uses a slotmap based implementation of the Surface Mesh which takes heavy inspiration from 
[blackjacks](https://github.com/setzer22/blackjack) halfedge-mesh implementation.


### Usage

Coming soon...

### Goals

I aim to provide a flexible rust implementation of the Surface Mesh with a focus
on usefulness for procedural mesh generation. Other goals are:

- Ergonomic and easy-to-use api
- Support most operations that are possible in modern 3D modeling software like blender
- Integration with the bevy game engine

### Non-Goals

To manage scope I won't focus on the following:

- Performance: For now I will prefer ease of use over performance where possible
- Support for unusual meshes: Will mainly target manifold tri & quad meshes for now

