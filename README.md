[![CI](https://github.com/Bendzae/SMesh/actions/workflows/rust.yml/badge.svg)](https://github.com/Bendzae/SMesh/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/smesh.svg)](https://crates.io/crates/smesh)

# SMesh

> [!CAUTION]
> Library is still work in progress

SMesh is a polygon mesh manipulation library based on the
[Surface Mesh](https://link.springer.com/chapter/10.1007/978-3-642-24734-7_29)
data structure and the [pmp](https://github.com/pmp-library/pmp-library)
library and the halfedge-mesh implementation of the [blackjack](https://github.com/setzer22/blackjack)
project.

The libary uses a slotmap based implementation of the Surface Mesh which takes heavy inspiration from
[blackjacks](https://github.com/setzer22/blackjack) halfedge-mesh implementation.

The goal of this library is to provide a flexible mesh abstraction and
set of operations to be used for procedural modeling and procedural generation
of 3D meshes.

### Examples

![tree_example](smesh_demo.gif)
_parameterized procedural mesh generation_

`cargo run --example tree`

![extrude_example](extrude_demo.gif)
_mesh extrusion and manipulation example with the visual debug tools enabled_

`cargo run --example extrude`

### Usage

_Preface_: Mesh elements in SMesh are identified by a unique typesafe id, which can be of type:
`VertexId`, `HalfedgeId` and `FaceId`.

#### Mesh creation

SMesh has a simple api to add vertices to your mesh and connect them to faces:
_Add vertices_

```rust
    let mut smesh = SMesh::new();
    let v0 = smesh.add_vertex(vec3(-1.0, -1.0, 0.0)); // Returns a unique VertexId
    let v1 = smesh.add_vertex(vec3(1.0, -1.0, 0.0));
    let v2 = smesh.add_vertex(vec3(1.0, 1.0, 0.0));
    let v3 = smesh.add_vertex(vec3(-1.0, 1.0, 0.0));
```

_Build face_

```rust
    smesh.add_face(vec![v0, v1, v2, v3])?;
```

#### Mesh queries

SMesh provides a chainable api to query mesh elements using the typical halfedge-mesh relationships:

_get outgoing halfedge of a vertex_

```rust
let outgoing_halfedge_query = v0.halfedge(); // returns a MeshQueryBuilder<HalfedgeId>
```

_you can execute the query on a smesh instance by using `.run(&smesh)`_

```rust
let outgoing_halfedge = v0.halfedge().run(&smesh)?;  // returns a HalfedgeId
```

_chaining queries_

```rust
let vertex = v0.halfedge_to(v1).cw_rotated_neighbour().dst_vert().run(&smesh)?;  // returns a VertexId
```

#### Mesh operations

Coming soon...

Please check the examples for more :)

#### UV Unwrapping

SMesh supports automatic UV unwrapping via the xatlas library. Enable the `xatlas` feature:

```toml
[dependencies]
smesh = { version = "0.2", features = ["xatlas"] }
```

Then generate UVs for your mesh:

```rust
mesh.generate_uv_atlas()?;
```

Or with custom options:

```rust
use smesh::smesh::xatlas_integration::{XatlasOptions, ChartOptions, PackOptions};

let options = XatlasOptions {
    chart: ChartOptions::default(),
    pack: PackOptions::default(),
};
mesh.generate_uv_atlas_with_options(options)?;
```

### Goals

I aim to provide a flexible rust implementation of the Surface Mesh with a focus
on usefulness for procedural mesh generation. Other goals are:

- Ergonomic and easy-to-use api
- Port most operations from the pmp library and blackjack
- Support most operations that are possible in modern 3D modeling software like
  blender
- Integration with the bevy game engine
- Target manifold tri & quad meshes for now
