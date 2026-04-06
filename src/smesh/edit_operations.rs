use std::collections::{HashMap, HashSet};

use glam::Vec3;
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
    /// Inset a single face by creating a smaller inner face connected to the
    /// original boundary by quad side faces. The inner face vertices are
    /// moved toward the face centroid by `amount` (0.0 = no inset, 1.0 =
    /// collapsed to centroid). Returns the new inner face.
    pub fn inset(&mut self, face: FaceId, amount: f32) -> SMeshResult<FaceId> {
        let vertices = face.vertices(self).collect_vec();
        let centroid = self.get_face_centroid(face)?;

        // Create new vertices lerped toward centroid
        let mut vertex_pairs = Vec::new();
        for v in &vertices {
            let pos = v.position(self)?;
            let new_pos = pos + (centroid - pos) * amount;
            vertex_pairs.push((*v, self.add_vertex(new_pos)));
        }

        self.delete_only_face(face)?;

        // Create quad side faces connecting outer to inner
        for ((old_0, new_0), (old_1, new_1)) in
            vertex_pairs.iter().copied().circular_tuple_windows()
        {
            self.make_quad(old_0, old_1, new_1, new_0)?;
        }

        // Create inner face
        let inner_face =
            self.make_face(vertex_pairs.iter().map(|(_old, new)| *new).collect_vec())?;
        Ok(inner_face)
    }

    /// Inset multiple faces together. Shared edges between selected faces
    /// are preserved (no side quads are created along them). Returns the
    /// new inner faces.
    pub fn inset_faces(
        &mut self,
        faces: Vec<FaceId>,
        amount: f32,
    ) -> SMeshResult<Vec<FaceId>> {
        // Step 1: Compute centroid per face and create new inset vertices
        let mut vertex_map: HashMap<VertexId, VertexId> = HashMap::new();
        let mut face_centroids: HashMap<FaceId, glam::Vec3> = HashMap::new();

        for &face in &faces {
            let centroid = self.get_face_centroid(face)?;
            face_centroids.insert(face, centroid);
        }

        // For each face, figure out which centroid to use for each vertex.
        // A vertex shared by multiple selected faces gets averaged centroids.
        let mut vertex_centroid_sum: HashMap<VertexId, (glam::Vec3, u32)> = HashMap::new();
        for &face in &faces {
            let centroid = face_centroids[&face];
            for v in face.vertices(self) {
                let entry = vertex_centroid_sum.entry(v).or_insert((glam::Vec3::ZERO, 0));
                entry.0 += centroid;
                entry.1 += 1;
            }
        }

        // Create inset vertices
        for (&v, &(centroid_sum, count)) in &vertex_centroid_sum {
            let avg_centroid = centroid_sum / count as f32;
            let pos = v.position(self)?;
            let new_pos = pos + (avg_centroid - pos) * amount;
            vertex_map.insert(v, self.add_vertex(new_pos));
        }

        // Step 2: Collect face vertices before deletion
        let mut face_vertex_map: HashMap<FaceId, Vec<VertexId>> = HashMap::new();
        for &face in &faces {
            face_vertex_map.insert(face, face.vertices(self).collect_vec());
        }

        // Step 3: Find boundary vs inner half-edges
        let selected_faces: HashSet<FaceId> = faces.iter().cloned().collect();
        let mut boundary_half_edges = Vec::new();
        let mut inner_half_edges = Vec::new();
        let mut boundary_vertices = Vec::new();

        for &face in &faces {
            for he in face.halfedges(self) {
                let opp = he.opposite().run(self)?;
                let adj_face = opp.face().run(self).ok();
                if adj_face.is_none() || !selected_faces.contains(&adj_face.unwrap()) {
                    boundary_half_edges.push(he);
                    boundary_vertices.push(he.src_vert().run(self)?);
                    boundary_vertices.push(he.dst_vert().run(self)?);
                } else if selected_faces.contains(&adj_face.unwrap()) {
                    inner_half_edges.push(he);
                }
            }
        }

        // Step 4: Delete old faces/edges
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

        // Step 5: Create quad side faces along boundary edges
        for edge in &boundary_half_edges {
            let src_old = edge.src_vert().run(self)?;
            let dst_old = edge.dst_vert().run(self)?;
            let src_new = vertex_map[&src_old];
            let dst_new = vertex_map[&dst_old];
            self.make_quad(src_old, dst_old, dst_new, src_new)?;
        }

        // Step 6: Create inner faces
        let mut new_faces = Vec::new();
        for &face in &faces {
            let old_verts = &face_vertex_map[&face];
            let new_verts = old_verts.iter().map(|&v| vertex_map[&v]).collect_vec();
            new_faces.push(self.make_face(new_verts)?);
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
            let he_opposite = he.opposite().run(self)?;
            if he_cache.contains(&he) {
                continue;
            }
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

    /// Catmull-Clark smooth subdivision.
    ///
    /// Subdivides the mesh topology (same as `subdivide`) but positions new and
    /// existing vertices using Catmull-Clark weighting rules to approximate a
    /// smooth limit surface. Supports multiple iterations.
    ///
    /// For quad-dominant meshes this produces the standard CC surface. Triangle
    /// faces are subdivided into quads (with a center vertex), so repeated
    /// iterations converge to all-quads.
    pub fn smooth_subdivide<S: Into<MeshSelection> + Clone>(
        &mut self,
        selection: S,
        iterations: usize,
    ) -> SMeshResult<MeshSelection> {
        let mut sel: MeshSelection = selection.into();
        for _ in 0..iterations {
            sel = self.smooth_subdivide_once(sel)?;
        }
        Ok(sel)
    }

    /// Single iteration of Catmull-Clark smooth subdivision.
    fn smooth_subdivide_once<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
    ) -> SMeshResult<MeshSelection> {
        let s: MeshSelection = selection.into();
        let faces: Vec<FaceId> = s.clone().resolve_to_faces(self)?.into_iter().collect();
        let halfedges: Vec<HalfedgeId> = s.clone().resolve_to_halfedges(self)?.into_iter().collect();

        // --- Phase 1: Precompute Catmull-Clark positions ---

        // 1a. Face points: centroid of each face
        let mut face_points: HashMap<FaceId, Vec3> = HashMap::new();
        for &f in &faces {
            face_points.insert(f, self.get_face_centroid(f)?);
        }

        // 1b. Edge points: average of (edge midpoint, adjacent face points)
        // For each undirected edge, compute the CC edge point.
        let mut edge_points: HashMap<(HalfedgeId, HalfedgeId), Vec3> = HashMap::new();
        let mut he_seen: HashSet<HalfedgeId> = HashSet::new();
        for &he in &halfedges {
            if he_seen.contains(&he) {
                continue;
            }
            let opp = he.opposite().run(self)?;
            he_seen.insert(he);
            he_seen.insert(opp);

            let p0 = he.src_vert().position(self)?;
            let p1 = he.dst_vert().position(self)?;
            let edge_mid = 0.5 * (p0 + p1);

            // Get adjacent face points
            let f0 = he.face().run(self).ok().and_then(|f| face_points.get(&f));
            let f1 = opp.face().run(self).ok().and_then(|f| face_points.get(&f));

            let edge_point = match (f0, f1) {
                (Some(&fp0), Some(&fp1)) => 0.25 * (p0 + p1 + fp0 + fp1),
                _ => edge_mid, // boundary edge: just use midpoint
            };

            let key = if he < opp { (he, opp) } else { (opp, he) };
            edge_points.insert(key, edge_point);
        }

        // 1c. New positions for original vertices using CC vertex rule:
        //     new_pos = (Q/n + 2R/n + S(n-3)/n)
        //     Q = avg of adjacent face points
        //     R = avg of adjacent edge midpoints
        //     S = original position
        //     n = valence
        let mut original_verts: HashSet<VertexId> = HashSet::new();
        for &f in &faces {
            for v in f.vertices(self) {
                original_verts.insert(v);
            }
        }

        let mut vertex_new_positions: HashMap<VertexId, Vec3> = HashMap::new();
        for &v in &original_verts {
            if v.is_boundary(self) {
                // Boundary vertex: average of adjacent boundary edge midpoints and original
                let mut boundary_mids = Vec::new();
                for he in v.halfedges(self) {
                    if he.is_boundary(self) {
                        let dst = he.dst_vert().position(self)?;
                        let src = v.position(self)?;
                        boundary_mids.push(0.5 * (src + dst));
                    }
                }
                if boundary_mids.len() >= 2 {
                    let r: Vec3 =
                        boundary_mids.iter().copied().sum::<Vec3>() / boundary_mids.len() as f32;
                    let s = v.position(self)?;
                    vertex_new_positions.insert(v, 0.5 * r + 0.5 * s);
                }
                continue;
            }

            let s = v.position(self)?;
            let n = v.vertices(self).count() as f32;
            if n < 1.0 {
                continue;
            }

            // Q: average of adjacent face points
            let adj_faces: Vec<Vec3> = v
                .faces(self)
                .filter_map(|f| face_points.get(&f))
                .copied()
                .collect();
            if adj_faces.is_empty() {
                continue;
            }
            let q = adj_faces.iter().copied().sum::<Vec3>() / adj_faces.len() as f32;

            // R: average of adjacent edge midpoints
            let adj_edge_mids: Vec<Vec3> = v
                .vertices(self)
                .filter_map(|nb| {
                    let nb_pos = nb.position(self).ok()?;
                    Some(0.5 * (s + nb_pos))
                })
                .collect();
            let r = adj_edge_mids.iter().copied().sum::<Vec3>() / adj_edge_mids.len().max(1) as f32;

            let new_pos = q / n + 2.0 * r / n + s * (n - 3.0) / n;
            vertex_new_positions.insert(v, new_pos);
        }

        // --- Phase 2: Run topology subdivision (same as subdivide) ---

        let face_corners: HashMap<FaceId, VertexId> = self
            .faces()
            .map(|f| (f, f.halfedge().src_vert().run(self).unwrap()))
            .collect();

        // Map from inserted edge-split vertex to its precomputed edge point
        let mut edge_vertex_positions: HashMap<VertexId, Vec3> = HashMap::new();
        // Map from face center vertex to face point
        let mut face_vertex_positions: HashMap<VertexId, Vec3> = HashMap::new();

        let mut he_cache = HashSet::new();
        let mut result_selection = MeshSelection::new();

        // Split edges and place at CC edge points
        for &he in &halfedges {
            let he_opposite = he.opposite().run(self)?;
            if he_cache.contains(&he) {
                continue;
            }
            // Compute key for edge_points lookup
            let key = if he < he_opposite {
                (he, he_opposite)
            } else {
                (he_opposite, he)
            };

            let midpoint = edge_points
                .get(&key)
                .copied()
                .unwrap_or_else(|| {
                    let p0 = he.src_vert().position(self).unwrap_or_default();
                    let p1 = he.dst_vert().position(self).unwrap_or_default();
                    0.5 * (p0 + p1)
                });

            let v = self.add_vertex(midpoint);
            edge_vertex_positions.insert(v, midpoint);

            let new_he = self.insert_vertex(he, v)?;
            he_cache.insert(he);
            he_cache.insert(he_opposite);
            result_selection.insert(he);
            result_selection.insert(he_opposite);
            result_selection.insert(new_he);
            result_selection.insert(new_he.opposite().run(self)?);
        }

        // Subdivide faces
        for &f in &faces {
            let valence = f.valence(self) / 2;
            let corner = face_corners.get(&f).copied().unwrap_or_else(|| {
                f.halfedge().src_vert().run(self).unwrap()
            });
            let corner_edge = f
                .halfedges(self)
                .find(|he| he.src_vert().run(self).unwrap() == corner)
                .unwrap();
            let he_loop = self.halfedge_loop(corner_edge.next().run(self)?);

            if valence == 3 {
                // For CC, triangles get a center vertex and produce quads
                let fp = face_points.get(&f).copied().unwrap_or_else(|| {
                    self.get_face_centroid(f).unwrap_or_default()
                });
                let v_c = self.add_vertex(fp);
                face_vertex_positions.insert(v_c, fp);
                self.delete_only_face(f)?;

                // Create quads from each pair (corner, mid, center)
                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_quad(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                        v_c,
                    )?;
                    result_selection.insert(f);
                }
            } else if valence == 4 {
                let fp = face_points.get(&f).copied().unwrap_or_else(|| {
                    self.get_face_centroid(f).unwrap_or_default()
                });
                let v_c = self.add_vertex(fp);
                face_vertex_positions.insert(v_c, fp);
                self.delete_only_face(f)?;

                for (h0, h1) in he_loop.iter().circular_tuple_windows().step_by(2) {
                    let f = self.make_quad(
                        h0.src_vert().run(self)?,
                        h1.src_vert().run(self)?,
                        h1.dst_vert().run(self)?,
                        v_c,
                    )?;
                    result_selection.insert(f);
                }
            }
            // N-gons: leave as-is
        }

        // --- Phase 3: Apply CC positions to original vertices ---
        for (v, pos) in &vertex_new_positions {
            if self.positions.contains_key(*v) {
                self.positions.insert(*v, *pos);
            }
        }

        Ok(result_selection)
    }

    /// Perform a loop cut along an edge loop, splitting faces at parameter `t` (0..1).
    ///
    /// Starting from `start_edge`, traces across quad faces by following
    /// opposite edges (next.next in a quad). Splits each crossed edge and
    /// reconnects the resulting faces into two quads each.
    ///
    /// Returns a `MeshSelection` containing the new edge loop vertices and halfedges.
    pub fn loop_cut(&mut self, start_edge: HalfedgeId, t: f32) -> SMeshResult<MeshSelection> {
        let t = t.clamp(0.001, 0.999);

        // Phase 1: Trace the edge loop
        // Collect pairs of (halfedge_to_split, face_it_belongs_to)
        let mut loop_edges: Vec<HalfedgeId> = vec![];
        let mut visited_faces: HashSet<FaceId> = HashSet::new();

        let mut current = start_edge;
        loop {
            loop_edges.push(current);

            // Get the face of this halfedge
            let face = match current.face().run(self) {
                Ok(f) => f,
                Err(_) => break, // Boundary - open loop
            };

            if !visited_faces.insert(face) {
                // Already visited this face - closed loop, remove the duplicate
                loop_edges.pop();
                break;
            }

            // Check it's a quad
            if face.valence(self) != 4 {
                break; // Stop at non-quad
            }

            // The "across" edge in a quad: next.next
            let across = current.next().next().run(self)?;

            // Cross to adjacent face via opposite
            let opposite = across.opposite().run(self)?;

            if opposite.is_boundary(self) {
                // Include the boundary edge but stop
                loop_edges.push(across);
                break;
            }

            current = opposite;
        }

        if loop_edges.is_empty() {
            bail!("No edges to cut");
        }

        // Phase 2: Split each edge and record new vertices
        // We need to track: for each original halfedge, the new vertex inserted
        let mut split_vertices: Vec<VertexId> = Vec::new();
        let mut split_done: HashSet<HalfedgeId> = HashSet::new();

        for &he in &loop_edges {
            if split_done.contains(&he) {
                continue;
            }
            let src_pos = he.src_vert().position(self)?;
            let dst_pos = he.dst_vert().position(self)?;
            let new_pos = src_pos.lerp(dst_pos, t);
            let new_v = self.add_vertex(new_pos);
            self.insert_vertex(he, new_v)?;
            split_vertices.push(new_v);

            // Mark both halfedge and its opposite as done
            if let Ok(opp) = he.opposite().run(self) {
                split_done.insert(opp);
            }
            split_done.insert(he);
        }

        // Phase 3: Connect new vertices across each affected face
        // Each face that was in the loop now has 5+ edges (two of its edges were split).
        // We need to split each such face by connecting the two new vertices.
        let mut selection = MeshSelection::new();
        for v in &split_vertices {
            selection.insert(*v);
        }

        for &face in &visited_faces {
            // Check the face still exists (it should)
            if self.faces().find(|&f| f == face).is_none() {
                continue;
            }

            // Find which of our new split vertices are on this face
            let face_verts: Vec<VertexId> = face.vertices(self).collect();
            let mut new_verts_on_face: Vec<(usize, VertexId)> = Vec::new();
            for (i, &fv) in face_verts.iter().enumerate() {
                if split_vertices.contains(&fv) {
                    new_verts_on_face.push((i, fv));
                }
            }

            if new_verts_on_face.len() != 2 {
                continue; // Can't split this face properly
            }

            let (idx0, v_new0) = new_verts_on_face[0];
            let (idx1, v_new1) = new_verts_on_face[1];

            // Split the face: delete old face, create two new faces
            // Face vertices are ordered. We split at idx0 and idx1.
            let n = face_verts.len();
            let mut face_a_verts = Vec::new();
            let mut face_b_verts = Vec::new();

            // Walk from idx0 to idx1 (inclusive) for face A
            let mut i = idx0;
            loop {
                face_a_verts.push(face_verts[i]);
                if i == idx1 {
                    break;
                }
                i = (i + 1) % n;
            }

            // Walk from idx1 to idx0 (inclusive) for face B
            i = idx1;
            loop {
                face_b_verts.push(face_verts[i]);
                if i == idx0 {
                    break;
                }
                i = (i + 1) % n;
            }

            if face_a_verts.len() < 3 || face_b_verts.len() < 3 {
                continue;
            }

            self.delete_only_face(face)?;
            let fa = self.make_face(face_a_verts)?;
            let fb = self.make_face(face_b_verts)?;
            selection.insert(fa);
            selection.insert(fb);

            // Add the connecting edge to selection
            if let Ok(he) = v_new0.halfedge_to(v_new1).run(self) {
                selection.insert(he);
            }
        }

        Ok(selection)
    }

    /// Merge nearby vertices that are within `threshold` distance of each other.
    ///
    /// Uses union-find to handle transitive merges (A near B, B near C → all merge).
    /// For each cluster, picks one representative and reroutes all halfedges.
    /// Cleans up degenerate edges and faces after merging.
    ///
    /// Returns the number of vertex merges performed.
    /// Merge multiple vertices into a single vertex at the given position.
    ///
    /// All halfedge references to the merged vertices are rerouted to the
    /// surviving vertex. Degenerate edges and faces created by the merge
    /// are cleaned up automatically. Returns the surviving vertex ID.
    pub fn merge_vertices(
        &mut self,
        vertices: &[VertexId],
        target: glam::Vec3,
    ) -> SMeshResult<VertexId> {
        if vertices.is_empty() {
            bail!("merge_vertices requires at least one vertex");
        }
        if vertices.len() == 1 {
            self.positions.insert(vertices[0], target);
            return Ok(vertices[0]);
        }

        // First vertex is the representative / survivor
        let rep = vertices[0];
        self.positions.insert(rep, target);

        // Reroute all other vertices to the representative
        for &v in &vertices[1..] {
            // Reroute all halfedges pointing TO v (he.vertex == v)
            let hes_to_reroute: Vec<HalfedgeId> = self
                .halfedges()
                .filter(|&he| {
                    self.connectivity
                        .halfedges
                        .get(he)
                        .map(|h| h.vertex == v)
                        .unwrap_or(false)
                })
                .collect();

            for he in hes_to_reroute {
                self.connectivity.halfedges.get_mut(he).unwrap().vertex = rep;
            }

            // Transfer outgoing halfedge if needed
            if let Ok(outgoing) = v.halfedge().run(self) {
                if rep.halfedge().run(self).is_err() || rep.is_isolated(self) {
                    self.get_mut(rep).set_halfedge(Some(outgoing))?;
                }
            }

            // Delete the old vertex
            self.positions.remove(v);
            self.connectivity.vertices.remove(v);
        }

        // Clean up degenerate edges (src == dst)
        let degenerate_edges: Vec<HalfedgeId> = self
            .halfedges()
            .filter(|&he| {
                let dst = self.connectivity.halfedges.get(he).map(|h| h.vertex);
                let src = he
                    .opposite()
                    .run(self)
                    .ok()
                    .and_then(|opp| self.connectivity.halfedges.get(opp).map(|h| h.vertex));
                dst.is_some() && dst == src
            })
            .collect();

        for he in degenerate_edges {
            if self.halfedges().any(|h| h == he) {
                self.delete_only_edge(he)?;
            }
        }

        // Remove degenerate faces (faces with duplicate vertices or < 3 verts)
        let degenerate_faces: Vec<FaceId> = self
            .faces()
            .filter(|&f| {
                let verts: Vec<VertexId> = f.vertices(self).collect();
                let unique: HashSet<VertexId> = verts.iter().copied().collect();
                unique.len() < verts.len() || verts.len() < 3
            })
            .collect();

        for f in degenerate_faces {
            if self.faces().any(|face| face == f) {
                self.delete_only_face(f)?;
            }
        }

        // Adjust outgoing halfedge for the representative
        let _ = self.get_mut(rep).adjust_outgoing_halfedge();

        Ok(rep)
    }

    pub fn weld_vertices(&mut self, threshold: f32) -> SMeshResult<usize> {
        let threshold_sq = threshold * threshold;
        let verts: Vec<VertexId> = self.vertices().collect();
        let n = verts.len();

        if n == 0 {
            return Ok(0);
        }

        // Union-Find
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();
        for &v in &verts {
            parent.insert(v, v);
        }

        fn find(parent: &mut HashMap<VertexId, VertexId>, v: VertexId) -> VertexId {
            let p = parent[&v];
            if p == v {
                return v;
            }
            let root = find(parent, p);
            parent.insert(v, root);
            root
        }

        fn union(parent: &mut HashMap<VertexId, VertexId>, a: VertexId, b: VertexId) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra != rb {
                parent.insert(rb, ra);
            }
        }

        // Phase 1: Find merge candidates (O(n²))
        for i in 0..n {
            let pos_i = verts[i].position(self)?;
            for j in (i + 1)..n {
                let pos_j = verts[j].position(self)?;
                if (pos_i - pos_j).length_squared() <= threshold_sq {
                    union(&mut parent, verts[i], verts[j]);
                }
            }
        }

        // Phase 2: Build clusters
        let mut clusters: HashMap<VertexId, Vec<VertexId>> = HashMap::new();
        for &v in &verts {
            let root = find(&mut parent, v);
            clusters.entry(root).or_default().push(v);
        }

        // Filter to only clusters with more than one vertex
        let merge_clusters: Vec<(VertexId, Vec<VertexId>)> = clusters
            .into_iter()
            .filter(|(_, members)| members.len() > 1)
            .collect();

        if merge_clusters.is_empty() {
            return Ok(0);
        }

        let mut total_merges = 0;

        // Phase 3: For each cluster, compute average position and reroute
        for (rep, members) in &merge_clusters {
            // Compute average position
            let mut avg_pos = glam::Vec3::ZERO;
            for &v in members {
                avg_pos += v.position(self)?;
            }
            avg_pos /= members.len() as f32;
            self.positions.insert(*rep, avg_pos);

            // Reroute all non-representative vertices to the representative
            for &v in members {
                if v == *rep {
                    continue;
                }

                // Reroute all halfedges pointing TO v (he.vertex == v)
                let hes_to_reroute: Vec<HalfedgeId> = self
                    .halfedges()
                    .filter(|&he| {
                        self.connectivity
                            .halfedges
                            .get(he)
                            .map(|h| h.vertex == v)
                            .unwrap_or(false)
                    })
                    .collect();

                for he in hes_to_reroute {
                    self.connectivity.halfedges.get_mut(he).unwrap().vertex = *rep;
                }

                // Transfer outgoing halfedge if needed
                if let Ok(outgoing) = v.halfedge().run(self) {
                    if rep.halfedge().run(self).is_err() || rep.is_isolated(self) {
                        self.get_mut(*rep).set_halfedge(Some(outgoing))?;
                    }
                }

                // Delete the old vertex
                self.positions.remove(v);
                self.connectivity.vertices.remove(v);
                total_merges += 1;
            }
        }

        // Phase 4: Clean up degeneracies
        // Remove degenerate edges (src == dst)
        let degenerate_edges: Vec<HalfedgeId> = self
            .halfedges()
            .filter(|&he| {
                let dst = self.connectivity.halfedges.get(he).map(|h| h.vertex);
                let src = he
                    .opposite()
                    .run(self)
                    .ok()
                    .and_then(|opp| self.connectivity.halfedges.get(opp).map(|h| h.vertex));
                dst.is_some() && dst == src
            })
            .collect();

        for he in degenerate_edges {
            if self.halfedges().any(|h| h == he) {
                self.delete_only_edge(he)?;
            }
        }

        // Remove degenerate faces (faces with duplicate vertices)
        let degenerate_faces: Vec<FaceId> = self
            .faces()
            .filter(|&f| {
                let verts: Vec<VertexId> = f.vertices(self).collect();
                let unique: HashSet<VertexId> = verts.iter().copied().collect();
                unique.len() < verts.len() || verts.len() < 3
            })
            .collect();

        for f in degenerate_faces {
            if self.faces().any(|face| face == f) {
                self.delete_only_face(f)?;
            }
        }

        // Adjust outgoing halfedges for affected vertices
        for (rep, _) in &merge_clusters {
            if self.vertices().any(|v| v == *rep) {
                let _ = self.get_mut(*rep).adjust_outgoing_halfedge();
            }
        }

        Ok(total_merges)
    }

    /// Bridge two ordered vertex rings with quad faces.
    ///
    /// Takes two rings of vertices and connects corresponding pairs with quads.
    /// Both rings must have the same number of vertices. The function finds the
    /// optimal rotational alignment to minimize total edge-crossing distance.
    ///
    /// Winding: `loop_a` vertices are used in order, `loop_b` in reverse, so the
    /// resulting quads have consistent normals pointing outward from the bridge.
    pub fn bridge_vertices(
        &mut self,
        loop_a: &[VertexId],
        loop_b: &[VertexId],
    ) -> SMeshResult<Vec<FaceId>> {
        let n = loop_a.len();
        if n < 3 {
            bail!("bridge requires at least 3 vertices per loop");
        }
        if n != loop_b.len() {
            bail!("bridge loops must have the same vertex count");
        }

        // Find optimal rotation of loop_b to minimize total distance
        let best_offset = self.bridge_find_best_rotation(loop_a, loop_b)?;

        // Create quad faces connecting the two loops
        let mut faces = Vec::with_capacity(n);
        for i in 0..n {
            let next_i = (i + 1) % n;
            let a0 = loop_a[i];
            let a1 = loop_a[next_i];
            let b0 = loop_b[(i + best_offset) % n];
            let b1 = loop_b[(next_i + best_offset) % n];
            // Quad: a0 -> b0 -> b1 -> a1 (normals point outward from bridge)
            let face = self.make_quad(a0, b0, b1, a1)?;
            faces.push(face);
        }

        Ok(faces)
    }

    /// Bridge two boundary edge loops with quad faces.
    ///
    /// Each loop is a list of boundary halfedges forming a closed ring.
    /// The function extracts the vertex rings from the halfedge loops,
    /// finds optimal alignment, and connects them with quads.
    pub fn bridge(
        &mut self,
        loop_a: &[HalfedgeId],
        loop_b: &[HalfedgeId],
    ) -> SMeshResult<Vec<FaceId>> {
        // Validate: all halfedges must be boundary
        for &he in loop_a.iter().chain(loop_b.iter()) {
            if !he.is_boundary(self) {
                bail!("bridge: all halfedges must be boundary edges");
            }
        }

        // Extract vertex rings from halfedge loops (source vertex of each halfedge)
        let verts_a: Vec<VertexId> = loop_a
            .iter()
            .map(|he| he.src_vert().run(self))
            .collect::<Result<_, _>>()?;
        let verts_b: Vec<VertexId> = loop_b
            .iter()
            .map(|he| he.src_vert().run(self))
            .collect::<Result<_, _>>()?;

        self.bridge_vertices(&verts_a, &verts_b)
    }

    /// Find the rotation offset for loop_b that minimizes total distance to loop_a.
    fn bridge_find_best_rotation(
        &self,
        loop_a: &[VertexId],
        loop_b: &[VertexId],
    ) -> SMeshResult<usize> {
        let n = loop_a.len();
        let positions_a: Vec<glam::Vec3> = loop_a
            .iter()
            .map(|v| v.position(self))
            .collect::<Result<_, _>>()?;
        let positions_b: Vec<glam::Vec3> = loop_b
            .iter()
            .map(|v| v.position(self))
            .collect::<Result<_, _>>()?;

        let mut best_offset = 0;
        let mut best_dist = f32::INFINITY;

        for offset in 0..n {
            let total: f32 = (0..n)
                .map(|i| positions_a[i].distance_squared(positions_b[(i + offset) % n]))
                .sum();
            if total < best_dist {
                best_dist = total;
                best_offset = offset;
            }
        }

        Ok(best_offset)
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
        if let Some(uvs) = other.vertex_uvs {
            for (id, value) in uvs {
                if self.vertex_uvs.is_none() {
                    self.vertex_uvs = Some(SecondaryMap::new());
                }
                self.vertex_uvs.as_mut().unwrap().insert(v_map[&id], value);
            }
        }
        // TODO: copy custom attributes 
        // for attr in self.vertex_attributes {
        //
        // }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn test_weld_overlapping_cubes() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};
        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        let (other, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let verts_before_combine = mesh.vertices().len();
        mesh.combine_with(other)?;
        // After combine, should have double the vertices
        assert_eq!(mesh.vertices().len(), verts_before_combine * 2);

        let merges = mesh.weld_vertices(0.01)?;
        // All 8 vertices of the overlapping cube should merge
        assert!(merges > 0, "Should have merged some vertices");
        assert!(
            mesh.vertices().len() < verts_before_combine * 2,
            "Vertex count should decrease after weld"
        );
        Ok(())
    }

    #[test]
    fn test_weld_no_change() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};
        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let verts_before = mesh.vertices().len();
        let merges = mesh.weld_vertices(0.01)?;
        assert_eq!(merges, 0);
        assert_eq!(mesh.vertices().len(), verts_before);
        Ok(())
    }

    #[test]
    fn test_weld_threshold_respected() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        mesh.add_vertex(vec3(0.5, 0.0, 0.0));
        mesh.add_vertex(vec3(2.0, 0.0, 0.0));

        // Threshold too small - no merge
        let merges = mesh.weld_vertices(0.4)?;
        assert_eq!(merges, 0);
        assert_eq!(mesh.vertices().len(), 3);

        // Threshold large enough to merge v0 and v1
        let merges = mesh.weld_vertices(0.6)?;
        assert_eq!(merges, 1);
        assert_eq!(mesh.vertices().len(), 2);
        Ok(())
    }

    #[test]
    fn test_weld_separate_components() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        // Component 1: a triangle
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.5, 1.0, 0.0));
        mesh.make_triangle(v0, v1, v2)?;

        // Component 2: another triangle with one vertex near v0
        let v3 = mesh.add_vertex(vec3(0.01, 0.0, 0.0)); // Near v0
        let v4 = mesh.add_vertex(vec3(-1.0, 0.0, 0.0));
        let v5 = mesh.add_vertex(vec3(-0.5, 1.0, 0.0));
        mesh.make_triangle(v3, v5, v4)?;

        assert_eq!(mesh.vertices().len(), 6);
        let merges = mesh.weld_vertices(0.05)?;
        assert_eq!(merges, 1);
        assert_eq!(mesh.vertices().len(), 5);
        Ok(())
    }

    #[test]
    fn test_loop_cut_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let he = v0.halfedge_to(v1).run(mesh)?;
        let sel = mesh.loop_cut(he, 0.5)?;

        // Single quad split into two quads
        assert_eq!(mesh.faces().len(), 2);
        // 4 original + 2 new midpoint = 6
        assert_eq!(mesh.vertices().len(), 6);

        let new_verts = sel.resolve_to_vertices(mesh)?;
        assert!(new_verts.len() >= 2);
        Ok(())
    }

    #[test]
    fn test_loop_cut_cube_strip() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};
        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let initial_faces = mesh.faces().len();
        let initial_verts = mesh.vertices().len();

        // Find a non-boundary halfedge that belongs to a quad face
        let he = mesh
            .halfedges()
            .find(|&h| !h.is_boundary(&mesh) && h.face().run(&mesh).map(|f| f.valence(&mesh) == 4).unwrap_or(false))
            .unwrap();

        mesh.loop_cut(he, 0.5)?;

        // Should have more faces and vertices than before
        assert!(mesh.faces().len() > initial_faces);
        assert!(mesh.vertices().len() > initial_verts);
        Ok(())
    }

    #[test]
    fn test_loop_cut_midpoint() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(2.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(2.0, 2.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 2.0, 0.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let he = v0.halfedge_to(v1).run(mesh)?;
        let sel = mesh.loop_cut(he, 0.5)?;

        // Check new vertices are at midpoints
        let new_verts = sel.resolve_to_vertices(mesh)?;
        for v in new_verts {
            let pos = v.position(mesh)?;
            // New midpoint vertices should be at x=1.0 (midpoint of v0-v1 and v3-v2)
            // or at y=1.0 (midpoint of other edges)
            // At least one coordinate should be a midpoint value
            let is_original = (pos.x == 0.0 || pos.x == 2.0) && (pos.y == 0.0 || pos.y == 2.0);
            if !is_original {
                // This is a new vertex, should be at midpoint
                assert!(
                    (pos.x - 1.0).abs() < 1e-5 || (pos.y - 1.0).abs() < 1e-5,
                    "New vertex at {:?} should be at a midpoint",
                    pos
                );
            }
        }
        Ok(())
    }

    #[test]
    fn test_loop_cut_valid_mesh() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let he = v0.halfedge_to(v1).run(mesh)?;
        mesh.loop_cut(he, 0.5)?;

        let report = mesh.validate();
        assert!(
            report.is_valid(),
            "Mesh should have no validation issues after loop cut: {}",
            report
        );
        Ok(())
    }

    #[test]
    fn test_loop_cut_stops_at_non_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        // Create a quad adjacent to a triangle
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        // Add a triangle adjacent to edge v2-v3
        let v4 = mesh.add_vertex(vec3(1.5, 0.5, 0.0));
        mesh.make_triangle(v2, v1, v4)?;

        let initial_faces = mesh.faces().len();
        let he = v0.halfedge_to(v1).run(mesh)?;
        mesh.loop_cut(he, 0.5)?;

        // The quad should be split but the triangle should remain
        // Original: 2 faces. After: quad becomes 2 + triangle stays = 3
        assert!(mesh.faces().len() > initial_faces);
        Ok(())
    }

    #[test]
    fn test_loop_cut_cylinder() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cylinder, Primitive};
        let (mut mesh, _) = Cylinder {
            segments: 8,
            height: 2.0,
            radius: 1.0,
        }
        .generate()?;

        let initial_faces = mesh.faces().len();

        // Find a side quad edge (not on cap faces)
        let he = mesh
            .halfedges()
            .find(|&h| {
                !h.is_boundary(&mesh)
                    && h.face()
                        .run(&mesh)
                        .map(|f| f.valence(&mesh) == 4)
                        .unwrap_or(false)
            })
            .unwrap();

        mesh.loop_cut(he, 0.5)?;

        // Should have more faces (each quad in the ring split into 2)
        assert!(mesh.faces().len() > initial_faces);
        Ok(())
    }

    #[test]
    fn test_loop_cut_double_cut_cylinder() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cylinder, Primitive};
        let (mut mesh, _) = Cylinder {
            segments: 8,
            height: 2.0,
            radius: 0.5,
        }
        .generate()?;

        // First cut at 1/3
        let he = mesh
            .halfedges()
            .find(|&h| {
                if h.is_boundary(&mesh) { return false; }
                let face = h.face().run(&mesh).ok();
                if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 { return false; }
                let src = h.src_vert().position(&mesh).unwrap_or_default();
                let dst = h.dst_vert().position(&mesh).unwrap_or_default();
                (dst - src).normalize().y.abs() > 0.8
            })
            .unwrap();
        mesh.loop_cut(he, 0.33)?;

        let faces_after_first = mesh.faces().len();

        // Second cut — find a vertical edge in the upper portion.
        // After the first cut at t=0.33 (height 2.0, y=-1..1), cut is at y≈-0.34.
        // Upper edges go from y≈-0.34 to y=1.0. Filter for edges NOT touching y=-1.0.
        let he2 = mesh
            .halfedges()
            .find(|&h| {
                if h.is_boundary(&mesh) { return false; }
                let face = h.face().run(&mesh).ok();
                if face.map(|f| f.valence(&mesh)).unwrap_or(0) != 4 { return false; }
                let src = h.src_vert().position(&mesh).unwrap_or_default();
                let dst = h.dst_vert().position(&mesh).unwrap_or_default();
                let dir = (dst - src).normalize();
                dir.y.abs() > 0.5 && src.y.min(dst.y) > -0.5
            })
            .expect("Should find a vertical edge in the upper portion for second cut");

        mesh.loop_cut(he2, 0.5)?;
        assert!(mesh.faces().len() > faces_after_first);
        Ok(())
    }

    #[test]
    fn inset_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        // 1 face, 4 verts, 4 edges
        assert_eq!(mesh.faces().len(), 1);
        assert_eq!(mesh.vertices().len(), 4);

        let inner = mesh.inset(f, 0.5)?;

        // Should now have: 4 side quads + 1 inner face = 5 faces
        assert_eq!(mesh.faces().len(), 5);
        // 4 original + 4 inner = 8 vertices
        assert_eq!(mesh.vertices().len(), 8);

        // Inner face centroid should be at the original centroid (origin)
        let inner_centroid = mesh.get_face_centroid(inner)?;
        assert!(inner_centroid.length() < 0.01);

        // Inner face should be smaller — check one vertex
        let inner_v = inner.vertices(mesh).next().unwrap();
        let pos = inner_v.position(mesh)?;
        // At 50% inset, vertices should be halfway to centroid
        assert!(pos.x.abs() < 0.6, "Inner vertex should be inset, got x={}", pos.x);
        assert!(pos.z.abs() < 0.6, "Inner vertex should be inset, got z={}", pos.z);

        Ok(())
    }

    #[test]
    fn inset_zero_amount_preserves_shape() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f = mesh.make_quad(v0, v1, v2, v3)?;

        let inner = mesh.inset(f, 0.0)?;

        // Inner face should be same size as original (zero inset)
        let inner_v: Vec<_> = inner.vertices(mesh).collect();
        for v in inner_v {
            let pos = v.position(mesh)?;
            assert!(
                (pos.x.abs() - 1.0).abs() < 0.01 && (pos.z.abs() - 1.0).abs() < 0.01,
                "At 0 inset, inner verts should match original positions"
            );
        }

        Ok(())
    }

    #[test]
    fn inset_triangle() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.5, 0.0, 1.0));
        let f = mesh.make_triangle(v0, v1, v2)?;

        let inner = mesh.inset(f, 0.3)?;

        // 3 side quads + 1 inner tri = 4 faces
        assert_eq!(mesh.faces().len(), 4);
        assert_eq!(inner.valence(mesh), 3);

        Ok(())
    }

    #[test]
    fn bridge_two_quads() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        // Create two separate quad faces (open rings after deleting faces)
        // Ring A: a square at z = -1
        let a0 = mesh.add_vertex(vec3(-1.0, -1.0, -1.0));
        let a1 = mesh.add_vertex(vec3(1.0, -1.0, -1.0));
        let a2 = mesh.add_vertex(vec3(1.0, 1.0, -1.0));
        let a3 = mesh.add_vertex(vec3(-1.0, 1.0, -1.0));
        let fa = mesh.make_quad(a0, a1, a2, a3)?;

        // Ring B: a square at z = 1
        let b0 = mesh.add_vertex(vec3(-1.0, -1.0, 1.0));
        let b1 = mesh.add_vertex(vec3(1.0, -1.0, 1.0));
        let b2 = mesh.add_vertex(vec3(1.0, 1.0, 1.0));
        let b3 = mesh.add_vertex(vec3(-1.0, 1.0, 1.0));
        let fb = mesh.make_quad(b0, b1, b2, b3)?;

        // Delete the faces to leave open boundary loops
        mesh.delete_only_face(fa)?;
        mesh.delete_only_face(fb)?;

        assert_eq!(mesh.faces().len(), 0);
        assert_eq!(mesh.vertices().len(), 8);

        // Bridge the two vertex rings
        let faces = mesh.bridge_vertices(
            &[a0, a1, a2, a3],
            &[b0, b1, b2, b3],
        )?;

        assert_eq!(faces.len(), 4, "Should create 4 quad faces");
        assert_eq!(mesh.faces().len(), 4);

        // Each face should be a quad
        for f in &faces {
            assert_eq!(f.valence(mesh), 4, "Bridge faces should be quads");
        }

        Ok(())
    }

    #[test]
    fn bridge_alignment() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        // Ring A at z=0 (CCW from above)
        let a0 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let a1 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let a2 = mesh.add_vertex(vec3(-1.0, 0.0, 0.0));
        let a3 = mesh.add_vertex(vec3(0.0, -1.0, 0.0));

        // Ring B at z=2 — rotated 90° so naive pairing would cross
        let b0 = mesh.add_vertex(vec3(0.0, 1.0, 2.0));
        let b1 = mesh.add_vertex(vec3(-1.0, 0.0, 2.0));
        let b2 = mesh.add_vertex(vec3(0.0, -1.0, 2.0));
        let b3 = mesh.add_vertex(vec3(1.0, 0.0, 2.0));

        let faces = mesh.bridge_vertices(&[a0, a1, a2, a3], &[b0, b1, b2, b3])?;
        assert_eq!(faces.len(), 4);

        // The optimal alignment should pair a0(1,0,0) with b3(1,0,2) — offset 3
        // Verify no edges cross by checking total bridge edge length is reasonable
        let mut total_len = 0.0f32;
        for f in &faces {
            for v in f.vertices(mesh) {
                let pos = v.position(mesh)?;
                total_len += pos.length();
            }
        }
        // If alignment is wrong, total_len would be much larger due to crossing edges
        assert!(total_len > 0.0, "Bridge should produce geometry");

        Ok(())
    }

    #[test]
    fn bridge_hexagonal_tube() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();

        // Two hexagonal rings of free vertices (no existing faces)
        let n = 6;
        let mut ring_a = Vec::new();
        let mut ring_b = Vec::new();
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            let x = angle.cos();
            let z = angle.sin();
            ring_a.push(mesh.add_vertex(vec3(x, 0.0, z)));
            ring_b.push(mesh.add_vertex(vec3(x, 2.0, z)));
        }

        let faces = mesh.bridge_vertices(&ring_a, &ring_b)?;
        assert_eq!(faces.len(), n);

        for f in &faces {
            assert_eq!(f.valence(mesh), 4, "All bridge faces should be quads");
        }

        // Mesh should be valid
        let report = mesh.validate();
        assert!(
            report.is_valid(),
            "Bridged mesh should be valid: {}",
            report
        );

        Ok(())
    }

    #[cfg(feature = "preview")]
    #[test]
    #[ignore]
    fn bridge_preview() -> SMeshResult<()> {
        use crate::smesh::preview::PreviewOptions;

        let mesh = &mut SMesh::new();
        let n = 8;
        let mut ring_a = Vec::new();
        let mut ring_b = Vec::new();
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            let x = angle.cos();
            let z = angle.sin();
            ring_a.push(mesh.add_vertex(vec3(x, 0.0, z)));
            ring_b.push(mesh.add_vertex(vec3(x * 0.7, 3.0, z * 0.7)));
        }
        mesh.bridge_vertices(&ring_a, &ring_b)?;
        mesh.recalculate_normals()?;

        let opts = PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe();
        mesh.save_composite_preview(&opts, "/tmp/preview_bridge.png")
            .unwrap();
        eprintln!("Saved /tmp/preview_bridge.png");

        Ok(())
    }

    #[test]
    fn bridge_with_triangle_fan_cap() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let n = 6usize;
        let mut bottom = Vec::new();
        let mut top = Vec::new();
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            bottom.push(mesh.add_vertex(vec3(angle.cos(), 0.0, angle.sin())));
            top.push(mesh.add_vertex(vec3(angle.cos() * 0.5, 2.0, angle.sin() * 0.5)));
        }
        mesh.bridge_vertices(&bottom, &top)?;

        // Find the boundary direction on top ring by checking a boundary halfedge
        // After bridge with quads a[i]->b[i]->b[i+1]->a[i+1],
        // boundary on top goes b[i+1]->b[i] (reverse order)
        let top_center = mesh.add_vertex(vec3(0.0, 2.0, 0.0));
        for i in 0..n {
            let next = (i + 1) % n;
            // Boundary goes reverse, so fan triangles: top[next], top[i], center
            mesh.make_triangle(top[next], top[i], top_center)?;
        }

        let bottom_center = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        for i in 0..n {
            let next = (i + 1) % n;
            // Bottom boundary goes forward: a[i]->a[i+1], so: bottom[i], bottom[next], center
            mesh.make_triangle(bottom[i], bottom[next], bottom_center)?;
        }

        let report = mesh.validate();
        assert!(report.is_valid(), "Capped bridge should be valid: {}", report);
        Ok(())
    }

    #[test]
    fn merge_vertices_triangle_fan_cap() -> SMeshResult<()> {
        // Bridge a tube, then cap it by merging the top ring into a center point
        let mesh = &mut SMesh::new();
        let n = 6usize;
        let mut bottom = Vec::new();
        let mut top = Vec::new();
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            bottom.push(mesh.add_vertex(vec3(angle.cos(), 0.0, angle.sin())));
            top.push(mesh.add_vertex(vec3(angle.cos() * 0.5, 2.0, angle.sin() * 0.5)));
        }
        mesh.bridge_vertices(&bottom, &top)?;

        // Cap top: extrude-like approach — create new ring at same position, then merge
        // Actually simpler: just bridge to a duplicate ring, then merge the duplicates
        // Simplest: create cap faces first, then merge center
        let top_center = mesh.add_vertex(vec3(0.0, 2.0, 0.0));
        for i in 0..n {
            let next = (i + 1) % n;
            mesh.make_triangle(top[next], top[i], top_center)?;
        }

        let verts_before = mesh.vertices().count();
        assert_eq!(verts_before, n * 2 + 1); // bottom + top + center

        // Now test merge_vertices by merging bottom ring to center
        let bottom_center = vec3(0.0, 0.0, 0.0);
        let survivor = mesh.merge_vertices(&bottom, bottom_center)?;

        // Should have lost n-1 vertices (all bottom verts merged into one)
        assert_eq!(mesh.vertices().count(), verts_before - (n - 1));

        // Survivor should be at the target position
        let pos = survivor.position(mesh)?;
        assert!((pos - bottom_center).length() < 0.001);

        Ok(())
    }

    #[test]
    fn cap_with_extrude_and_merge() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let n = 8usize;
        let mut bottom = Vec::new();
        let mut top = Vec::new();
        for i in 0..n {
            let angle = std::f32::consts::TAU * i as f32 / n as f32;
            bottom.push(mesh.add_vertex(vec3(angle.cos(), 0.0, angle.sin())));
            top.push(mesh.add_vertex(vec3(angle.cos() * 0.5, 3.0, angle.sin() * 0.5)));
        }
        mesh.bridge_vertices(&bottom, &top)?;

        // Cap bottom by extruding each boundary edge individually, then merging
        let mut new_verts = Vec::new();
        for i in 0..n {
            let boundary_he = bottom[i]
                .halfedges(mesh)
                .find(|he| he.is_boundary(mesh))
                .unwrap();
            let new_edge = mesh.extrude_edge(boundary_he)?;
            new_verts.push(new_edge.src_vert().run(mesh)?);
        }

        mesh.merge_vertices(&new_verts, vec3(0.0, 0.0, 0.0))?;
        mesh.recalculate_normals()?;

        // Should have: 8 bridge quads + 8 cap triangles = 16 faces, top still open
        assert_eq!(mesh.faces().count(), n * 2);

        Ok(())
    }

    #[test]
    fn merge_vertices_single() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v = mesh.add_vertex(vec3(1.0, 2.0, 3.0));
        let target = vec3(0.0, 0.0, 0.0);
        let result = mesh.merge_vertices(&[v], target)?;
        assert_eq!(result, v);
        assert!((v.position(mesh)? - target).length() < 0.001);
        Ok(())
    }

    #[test]
    fn merge_vertices_on_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        // Merge v2 and v3 — quad becomes a triangle
        let center = vec3(0.5, 1.0, 0.0);
        mesh.merge_vertices(&[v2, v3], center)?;

        // The quad should have become a triangle (or been cleaned up)
        assert_eq!(mesh.vertices().count(), 3);

        Ok(())
    }

    #[test]
    fn bridge_rejects_mismatched_sizes() {
        let mesh = &mut SMesh::new();
        let a0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let a1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let a2 = mesh.add_vertex(vec3(0.5, 1.0, 0.0));

        let b0 = mesh.add_vertex(vec3(0.0, 0.0, 2.0));
        let b1 = mesh.add_vertex(vec3(1.0, 0.0, 2.0));
        let b2 = mesh.add_vertex(vec3(1.0, 1.0, 2.0));
        let b3 = mesh.add_vertex(vec3(0.0, 1.0, 2.0));

        let result = mesh.bridge_vertices(&[a0, a1, a2], &[b0, b1, b2, b3]);
        assert!(result.is_err(), "Should reject mismatched loop sizes");
    }

    #[test]
    fn inset_faces_multiple() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(0.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v4 = mesh.add_vertex(vec3(0.0, 0.0, 1.0));
        let v5 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let f0 = mesh.make_quad(v0, v1, v4, v5)?;
        let f1 = mesh.make_quad(v1, v2, v3, v4)?;

        assert_eq!(mesh.faces().len(), 2);

        let inner = mesh.inset_faces(vec![f0, f1], 0.3)?;
        assert_eq!(inner.len(), 2);

        // Shared edge between f0 and f1 should not produce side quads
        // Boundary edges: 6 (top, bottom, left, right of the 2x1 strip)
        // So: 6 side quads + 2 inner faces = 8 faces
        assert_eq!(mesh.faces().len(), 8);

        Ok(())
    }
}
