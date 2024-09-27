use crate::prelude::*;
use glam::vec3;

pub fn vertex_onering() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    let v0 = mesh.add_vertex(vec3(0.4499998093, 0.5196152329, 0.0000000000));
    let v1 = mesh.add_vertex(vec3(0.2999998033, 0.5196152329, 0.0000000000));
    let v2 = mesh.add_vertex(vec3(0.5249998569, 0.3897114396, 0.0000000000));
    let v3 = mesh.add_vertex(vec3(0.3749998510, 0.3897114396, 0.0000000000));
    let v4 = mesh.add_vertex(vec3(0.2249998450, 0.3897114396, 0.0000000000));
    let v5 = mesh.add_vertex(vec3(0.4499999285, 0.2598076165, 0.0000000000));
    let v6 = mesh.add_vertex(vec3(0.2999999225, 0.2598076165, 0.0000000000));

    mesh.add_triangle(v3, v0, v1)?;
    mesh.add_triangle(v3, v2, v0)?;
    mesh.add_triangle(v4, v3, v1)?;
    mesh.add_triangle(v5, v2, v3)?;
    mesh.add_triangle(v6, v5, v3)?;
    mesh.add_triangle(v6, v3, v4)?;

    Ok(mesh)
}

pub fn edge_onering() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    let v0 = mesh.add_vertex(vec3(0.5999997854, 0.5196152329, 0.0000000000));
    let v1 = mesh.add_vertex(vec3(0.4499998093, 0.5196152329, 0.0000000000));
    let v2 = mesh.add_vertex(vec3(0.2999998033, 0.5196152329, 0.0000000000));
    let v3 = mesh.add_vertex(vec3(0.6749998331, 0.3897114396, 0.0000000000));
    let v4 = mesh.add_vertex(vec3(0.5249998569, 0.3897114396, 0.0000000000));
    let v5 = mesh.add_vertex(vec3(0.3749998510, 0.3897114396, 0.0000000000));
    let v6 = mesh.add_vertex(vec3(0.2249998450, 0.3897114396, 0.0000000000));
    let v7 = mesh.add_vertex(vec3(0.5999999046, 0.2598076165, 0.0000000000));
    let v8 = mesh.add_vertex(vec3(0.4499999285, 0.2598076165, 0.0000000000));
    let v9 = mesh.add_vertex(vec3(0.2999999225, 0.2598076165, 0.0000000000));

    mesh.add_triangle(v4, v0, v1)?;
    mesh.add_triangle(v4, v3, v0)?;
    mesh.add_triangle(v5, v1, v2)?;
    mesh.add_triangle(v5, v4, v1)?;
    mesh.add_triangle(v6, v5, v2)?;
    mesh.add_triangle(v7, v3, v4)?;
    mesh.add_triangle(v8, v7, v4)?;
    mesh.add_triangle(v8, v4, v5)?;
    mesh.add_triangle(v9, v8, v5)?;
    mesh.add_triangle(v9, v5, v6)?;

    Ok(mesh)
}

pub fn subdivided_icosahedron() {
    todo!()
    // if (icosahedron_mesh.is_empty())
    // {
    // // use ref for brevity
    // let& mesh = icosahedron_mesh;
    // mesh = icosahedron();
    //
    // // select all edges as features
    // detect_features(mesh, 25);
    //
    // // feature-preserving subdivision
    // loop_subdivision(mesh);
    // loop_subdivision(mesh);
    // loop_subdivision(mesh);
    // }
    // return icosahedron_mesh;
    // }
    //
    // SurfaceMesh l_shape()
    // {
    // let mesh = &mut SMesh::new();
    //
    // std::vector<Vertex> vertices;
    //
    // vertices.push_back(mesh.add_vertex(vec3(0.0, 0.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.5, 0.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(1.0, 0.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(1.0, 0.5, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.5, 0.5, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.5, 1.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.5, 1.5, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.5, 2.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.0, 2.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.0, 1.5, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.0, 1.0, 0.0)));
    // vertices.push_back(mesh.add_vertex(vec3(0.0, 0.5, 0.0)));
    //
    // mesh.add_face(vertices);
    //
    // return mesh;
}

pub fn open_cone() {
    todo!()
    // let mesh = cone(8, 1, 1.5);
    // for (let f : mesh.faces())
    // if (mesh.valence(f) > 3)
    // {
    // mesh.delete_face(f);
    // mesh.garbage_collection();
    // break;
    // }
    // return mesh;
}

pub fn texture_seams_mesh() {
    todo!()
    // let mesh = &mut SMesh::new();
    // let v0 = mesh.add_vertex(vec3(0.5999997854, 0.5196152329, 0.0000000000));
    // let v1 = mesh.add_vertex(vec3(0.4499998093, 0.5196152329, -0.001000000));
    // let v2 = mesh.add_vertex(vec3(0.2999998033, 0.5196152329, 0.0000000000));
    // let v3 = mesh.add_vertex(vec3(0.6749998331, 0.3897114396, -0.001000000));
    // let v4 = mesh.add_vertex(vec3(0.5249998569, 0.3897114396, 0.0000000000));
    // let v5 = mesh.add_vertex(vec3(0.3749998510, 0.3897114396, 0.0000000000));
    // let v6 = mesh.add_vertex(vec3(0.2249998450, 0.3897114396, 0.0000000000));
    // let v7 = mesh.add_vertex(vec3(0.5999999046, 0.2598076165, 0.0000000000));
    // let v8 = mesh.add_vertex(vec3(0.4499999285, 0.2598076165, 0.0000000000));
    // let v9 = mesh.add_vertex(vec3(0.2999999225, 0.2598076165, 0.0000000000));
    // let v10 = mesh.add_vertex(vec3(0.749999285, 0.2598076165, 0.0000000000));
    // let v11 = mesh.add_vertex(vec3(0.8249998331, 0.3897114396, 0.0000000000));
    // let v12 = mesh.add_vertex(vec3(0.749999285, 0.5196152329, 0.0000000000));
    // let v13 = mesh.add_vertex(vec3(0.6749998331, 0.6496152329, 0.0000000000));
    // let v14 = mesh.add_vertex(vec3(0.5249998569, 0.6496152329, 0.0000000000));
    // let v15 = mesh.add_vertex(vec3(0.3749998510, 0.6496152329, 0.0000000000));
    //
    // mesh.add_triangle(v4, v0, v1);
    // mesh.add_triangle(v4, v3, v0);
    // mesh.add_triangle(v15, v4, v1);
    // mesh.add_triangle(v2, v5, v4);
    // mesh.add_triangle(v6, v5, v2);
    // mesh.add_triangle(v7, v11, v4);
    // mesh.add_triangle(v8, v7, v4);
    // mesh.add_triangle(v8, v4, v5);
    // mesh.add_triangle(v9, v8, v5);
    // mesh.add_triangle(v9, v5, v6);
    // mesh.add_triangle(v7, v10, v11);
    // mesh.add_triangle(v4, v11, v3);
    // mesh.add_triangle(v3, v11, v12);
    // mesh.add_triangle(v3, v12, v0);
    // mesh.add_triangle(v0, v12, v13);
    // mesh.add_triangle(v0, v13, v14);
    // mesh.add_triangle(v0, v14, v1);
    // mesh.add_triangle(v1, v14, v15);
    // mesh.add_triangle(v2, v4, v15);
    //
    // // add test texcoords
    // let texcoords = mesh.halfedge_property<Vector<Scalar, 2>>("h:tex");
    //
    // for (let v : mesh.vertices())
    // {
    // Point p = mesh.position(v);
    // for (let h : mesh.halfedges(v))
    // {
    // if (mesh.is_boundary(mesh.opposite_halfedge(h)))
    // {
    // continue;
    // }
    // texcoords[mesh.opposite_halfedge(h)] = TexCoord(p[0], p[1]);
    // }
    // }
    //
    // // change texcoords to create a texture seam
    // std::vector<Face> faces = {Face(0),  Face(1),  Face(12), Face(13),
    // Face(14), Face(15), Face(16), Face(17)};
    // for (let f : faces)
    // {
    // for (let h : mesh.halfedges(f))
    // {
    // texcoords[h] += TexCoord(0.1, 0.1);
    // }
    // }
    //
    // return mesh;
}
