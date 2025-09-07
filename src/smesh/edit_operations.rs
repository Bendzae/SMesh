use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use slotmap::SecondaryMap;

use crate::{bail, prelude::*};

/// Edit operations
impl SMesh {
    pub fn extrude(&mut self, face: FaceId) -> SMeshResult<FaceId> {
        let vertices = face.vertices(self).collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        self.delete_only_face(face)?;
        // Make faces
        for ((old_0, new_0), (old_1, new_1)) in
            vertex_pairs.iter().copied().circular_tuple_windows()
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
        }
        let top_face = self.make_face(vertex_pairs.iter().map(|(_old, new)| *new).collect_vec())?;
        Ok(top_face)
    }

    pub fn extrude_faces(&mut self, faces: Vec<FaceId>) -> SMeshResult<Vec<FaceId>> {
        // self.add_attribute_map::<HalfedgeId>("debug").unwrap();
        // Step 1: Collect all unique vertices and create mapping to new vertices
        let mut vertex_map = HashMap::new();
        for &face in faces.iter() {
            for vertex in face.vertices(self).collect_vec() {
                if !vertex_map.contains_key(&vertex) {
                    let position = vertex.position(self)?;
                    let new_vertex = self.add_vertex(position);
                    vertex_map.insert(vertex, new_vertex);
                }
            }
        }

        // Step 2: Collect the vertices of each face before deleting
        let mut face_vertex_map = HashMap::new();
        for &face in faces.iter() {
            let vertices = face.vertices(self).collect_vec();
            face_vertex_map.insert(face, vertices);
        }

        // Step 3: Collect boundary half-edges
        let selected_faces: HashSet<FaceId> = faces.iter().cloned().collect();
        let mut boundary_half_edges = Vec::new();
        let mut inner_half_edges = Vec::new();
        let mut boundary_vertices = Vec::new();
        for &face in faces.iter() {
            for half_edge in face.halfedges(self) {
                let opp = half_edge.opposite().run(self)?;
                let adjacent_face = opp.face().run(self).ok();
                if adjacent_face.is_none() || !selected_faces.contains(&adjacent_face.unwrap()) {
                    boundary_half_edges.push(half_edge);
                    boundary_vertices.push(half_edge.src_vert().run(self)?);
                    boundary_vertices.push(half_edge.dst_vert().run(self)?);
                } else {
                    let face = opp.face().run(self).ok();
                    if face.is_some() && selected_faces.contains(&face.unwrap()) {
                        inner_half_edges.push(half_edge);
                    }
                }
            }
        }
        //
        // for &he in &boundary_half_edges {
        //     self.attribute_mut("debug")
        //         .unwrap()
        //         .insert(he, "red".to_string());
        // }

        // Step 4: Delete the old vertices/faces
        if faces.len() == 1 {
            self.delete_only_face(*faces.first().unwrap())?;
        }
        for vertex in faces
            .iter()
            .flat_map(|f| f.vertices(self))
            .filter(|v| !boundary_vertices.contains(v))
            .collect_vec()
        {
            self.delete_vertex(vertex)?;
        }
        for he in inner_half_edges {
            self.delete_only_edge(he)?;
        }

        // Step 5: Create side faces along boundary edges
        for edge in boundary_half_edges.iter() {
            let src_old = edge.src_vert().run(self)?;
            let dst_old = edge.dst_vert().run(self)?;
            let src_new = vertex_map[&src_old];
            let dst_new = vertex_map[&dst_old];
            self.make_quad(src_old, dst_old, dst_new, src_new)?;
        }

        // Step 6: Create new faces on top
        let mut new_faces = Vec::new();
        for &face in faces.iter() {
            let old_vertices = &face_vertex_map[&face];
            let new_vertices = old_vertices.iter().map(|&v| vertex_map[&v]).collect_vec();
            let new_face = self.make_face(new_vertices)?;
            new_faces.push(new_face);
        }

        Ok(new_faces)
    }
    pub fn extrude_edge(&mut self, e0: HalfedgeId) -> SMeshResult<HalfedgeId> {
        // Find boundary halfedge
        let e0 = match e0.is_boundary(self) {
            true => e0,
            false => {
                let opposite = e0.opposite().run(self)?;
                if !opposite.is_boundary(self) {
                    bail!("Can only extrude boundary edges");
                }
                opposite
            }
        };

        let v0 = e0.src_vert().run(self)?;
        let v1 = e0.dst_vert().run(self)?;

        let pos0 = v0.position(self)?;
        let pos1 = v1.position(self)?;
        let v0_new = self.add_vertex(pos0);
        let v1_new = self.add_vertex(pos1);

        // TODO: maybe check vertex normals (if exist) to determine order?
        self.make_face(vec![v0, v1, v1_new, v0_new])?;
        let new_edge = v0_new.halfedge_to(v1_new).run(self)?;
        Ok(new_edge)
    }

    pub fn extrude_edge_chain(&mut self, edges: Vec<HalfedgeId>) -> SMeshResult<Vec<HalfedgeId>> {
        let mut boundary_edges = Vec::new();
        for e in edges {
            let eb = match e.is_boundary(self) {
                true => e,
                false => {
                    let opposite = e.opposite().run(self)?;
                    if !opposite.is_boundary(self) {
                        bail!("Can only extrude boundary edges");
                    }
                    opposite
                }
            };
            boundary_edges.push(eb);
        }

        // Assert all are connected in sequence and check if they form a loop
        let mut is_loop = false;
        for (i, (current, next)) in boundary_edges.iter().circular_tuple_windows().enumerate() {
            let is_last_iteration = i == boundary_edges.len() - 1;
            match current.next().run(self) {
                Ok(he) => {
                    if he == *next {
                        if is_last_iteration {
                            is_loop = true;
                        }
                    } else {
                        bail!("Not an edge chain");
                    }
                }
                Err(_) => {
                    if !is_last_iteration {
                        bail!("Halfedge has no next")
                    }
                }
            }
        }

        let vertices = boundary_edges
            .iter()
            .flat_map(|e| {
                vec![
                    e.src_vert().run(self).unwrap(),
                    e.dst_vert().run(self).unwrap(),
                ]
            })
            .collect_vec();
        // Duplicate verts
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let position = v.position(self)?;
            vertex_pairs.push((*v, self.add_vertex(position)));
        }

        assert_eq!(vertices.len(), vertex_pairs.len());

        // Make faces
        let mut new_edges = Vec::new();
        for ((old_0, new_0), (old_1, new_1)) in vertex_pairs
            .iter()
            .copied()
            .circular_tuple_windows()
            .take(vertex_pairs.len() - (if is_loop { 0 } else { 1 }))
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
            new_edges.push((new_0).halfedge_to(new_1).run(self)?);
        }
        Ok(new_edges)
    }

    pub fn subdivide<T: Into<MeshSelection>>(
        &mut self,
        selection: T,
    ) -> SMeshResult<MeshSelection> {
        let s: MeshSelection = selection.into();
        let faces = s.clone().resolve_to_faces(self)?;
        let halfedges = s.clone().resolve_to_halfedges(self)?;

        let face_corners = self
            .faces()
            .map(|f| (f, f.halfedge().src_vert().run(self).unwrap()))
            .collect::<HashMap<FaceId, VertexId>>();

        let mut he_cache = HashSet::new();
        // Returned selection
        let mut selection = MeshSelection::new();
        for he in halfedges {
            if he_cache.contains(&he) {
                continue;
            }

            let he_opposite = he.opposite().run(self)?;

            let p0 = he.src_vert().position(self)?;
            let p1 = he.dst_vert().position(self)?;
            let v = self.add_vertex(0.5 * (p0 + p1));
            let new_he = self.insert_vertex(he, v)?;

            he_cache.insert(he);
            he_cache.insert(he_opposite);

            selection.insert(he);
            selection.insert(he_opposite);
            selection.insert(new_he);
            selection.insert(new_he.opposite().run(self)?);
        }

        for f in faces {
            let valence = f.valence(self) / 2;
            let corner = face_corners[&f];
            let corner_edge = f
                .halfedges(self)
                .find(|he| he.src_vert().run(self).unwrap() == corner)
                .unwrap();
            let he_loop = self.halfedge_loop(corner_edge.next().run(self)?);

            if valence == 3 {
                self.delete_only_face(f)?;
                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_triangle(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                    )?;
                    selection.insert(f);
                }
                // Middle tri
                let f = self.make_triangle(
                    he_loop[0].src_vert().run(self)?,
                    he_loop[2].src_vert().run(self)?,
                    he_loop[4].src_vert().run(self)?,
                )?;
                selection.insert(f);
            }

            if valence == 4 {
                let center = self.get_face_centroid(f)?;
                let v_c = self.add_vertex(center);
                self.delete_only_face(f)?;
                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_quad(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                        v_c,
                    )?;
                    selection.insert(f);
                }
            }

            // valence > 4: ngons simply stay as ngons (same behaviour as blender)
        }

        Ok(selection)
    }

    pub fn combine_with(&mut self, other: SMesh) -> SMeshResult<()> {
        // Copy verts
        let mut v_map = HashMap::new();
        for (id, v) in other.connectivity.vertices {
            let id_new = self.vertices_mut().insert(v.clone());
            v_map.insert(id, id_new);
        }
        // Copy faces
        let mut f_map = HashMap::new();
        for (id, f) in other.connectivity.faces {
            let id_new = self.faces_mut().insert(f.clone());
            f_map.insert(id, id_new);
        }
        // Copy halfedges
        let mut he_map = HashMap::new();
        for (id, he) in other.connectivity.halfedges {
            let mut halfedge = he.clone();
            halfedge.vertex = v_map[&halfedge.vertex];
            if let Some(f) = halfedge.face {
                halfedge.face = Some(f_map[&f]);
            }
            let id_new = self.halfedges_mut().insert(halfedge.clone());
            he_map.insert(id, id_new);
        }
        // Remap remaining ids in halfedges
        for id in he_map.values() {
            let he = self.halfedges_mut().get_mut(*id).unwrap();
            if let Some(opp) = he.opposite {
                he.opposite = Some(he_map[&opp]);
            }
            if let Some(next) = he.next {
                he.next = Some(he_map[&next]);
            }
            if let Some(prev) = he.prev {
                he.prev = Some(he_map[&prev]);
            }
        }
        // Remap remaining ids in vertices
        for id in v_map.values() {
            if let Ok(he) = id.halfedge().run(self) {
                self.get_mut(*id).set_halfedge(Some(he_map[&he]))?;
            }
        }

        // Remap remaining ids in faces
        for id in f_map.values() {
            if let Ok(he) = id.halfedge().run(self) {
                self.get_mut(*id).set_halfedge(Some(he_map[&he]))?;
            }
        }

        // Copy attributes
        for (id, value) in other.positions {
            self.positions.insert(v_map[&id], value);
        }
        if let Some(vertex_normals) = other.vertex_normals {
            for (id, value) in vertex_normals {
                if self.vertex_normals.is_none() {
                    self.vertex_normals = Some(SecondaryMap::new());
                }
                self.vertex_normals
                    .as_mut()
                    .unwrap()
                    .insert(v_map[&id], value);
            }
        }
        if let Some(face_normals) = other.face_normals {
            for (id, value) in face_normals {
                if self.face_normals.is_none() {
                    self.face_normals = Some(SecondaryMap::new());
                }
                self.face_normals
                    .as_mut()
                    .unwrap()
                    .insert(f_map[&id], value);
            }
        }
        if let Some(uvs) = other.uvs {
            for (id, value) in uvs {
                if self.uvs.is_none() {
                    self.uvs = Some(SecondaryMap::new());
                }
                self.uvs.as_mut().unwrap().insert(v_map[&id], value);
            }
        }
        // TODO: copy custom attributes
        // for attr in self.vertex_attributes {
        //
        // }
        Ok(())
    }
}
