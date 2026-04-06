use glam::Vec3;

use crate::prelude::*;

/// Result of a raycast against the mesh.
#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    pub face: FaceId,
    pub point: Vec3,
    pub distance: f32,
    pub normal: Vec3,
}

impl SMesh {
    /// Find all vertices within a given radius of a point.
    pub fn query_region(&self, center: Vec3, radius: f32) -> MeshSelection {
        let radius_sq = radius * radius;
        let verts: Vec<VertexId> = self
            .vertices()
            .filter(|v| {
                v.position(self)
                    .map(|p| (p - center).length_squared() <= radius_sq)
                    .unwrap_or(false)
            })
            .collect();
        verts.into()
    }

    /// Select all vertices whose position satisfies the given predicate.
    pub fn vertices_where<F: Fn(Vec3) -> bool>(&self, predicate: F) -> MeshSelection {
        let verts: Vec<VertexId> = self
            .vertices()
            .filter(|v| {
                v.position(self)
                    .map(|p| predicate(p))
                    .unwrap_or(false)
            })
            .collect();
        verts.into()
    }

    /// Find the closest vertex to a point.
    pub fn nearest_vertex(&self, point: Vec3) -> Option<(VertexId, f32)> {
        let mut best: Option<(VertexId, f32)> = None;
        for v in self.vertices() {
            if let Ok(pos) = v.position(self) {
                let dist = (pos - point).length();
                if best.map_or(true, |(_, d)| dist < d) {
                    best = Some((v, dist));
                }
            }
        }
        best
    }

    /// Find all faces whose computed normal points roughly in the given direction.
    ///
    /// `threshold_angle` is in radians. A face is included if the angle between
    /// its normal and `direction` is less than `threshold_angle`.
    pub fn faces_facing(&self, direction: Vec3, threshold_angle: f32) -> Vec<FaceId> {
        let dir = direction.normalize_or_zero();
        if dir == Vec3::ZERO {
            return vec![];
        }
        let cos_threshold = threshold_angle.cos();

        self.faces()
            .filter(|face| {
                let positions: Vec<Vec3> = face
                    .vertices(self)
                    .filter_map(|v| v.position(self).ok())
                    .collect();
                if positions.len() < 3 {
                    return false;
                }
                let e1 = positions[1] - positions[0];
                let e2 = positions[2] - positions[0];
                let normal = e1.cross(e2).normalize_or_zero();
                if normal == Vec3::ZERO {
                    return false;
                }
                normal.dot(dir) >= cos_threshold
            })
            .collect()
    }

    /// Select a connected region of faces near a point.
    ///
    /// Finds the closest face to `center`, then flood-fills to neighboring faces
    /// whose centroids are within `radius` of `center`. Optionally filters by
    /// face normal direction.
    ///
    /// # Parameters
    ///
    /// - `center`: World-space point to select around.
    /// - `radius`: Maximum distance from `center` for face centroids.
    /// - `normal_filter`: Optional `(direction, max_angle)` — only include faces
    ///   whose normal is within `max_angle` radians of `direction`.
    pub fn select_region(
        &self,
        center: Vec3,
        radius: f32,
        normal_filter: Option<(Vec3, f32)>,
    ) -> Vec<FaceId> {
        let radius_sq = radius * radius;

        // Find seed face (closest centroid to center)
        let seed = self.faces().min_by(|a, b| {
            let da = self
                .get_face_centroid(*a)
                .map(|c| (c - center).length_squared())
                .unwrap_or(f32::INFINITY);
            let db = self
                .get_face_centroid(*b)
                .map(|c| (c - center).length_squared())
                .unwrap_or(f32::INFINITY);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(seed) = seed else {
            return vec![];
        };

        // Flood-fill from seed
        let mut selected = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(seed);
        visited.insert(seed);

        let normal_cos = normal_filter.map(|(dir, angle)| (dir.normalize_or_zero(), angle.cos()));

        while let Some(face) = queue.pop_front() {
            // Check centroid within radius
            let centroid = match self.get_face_centroid(face) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if (centroid - center).length_squared() > radius_sq {
                continue;
            }

            // Check normal filter
            if let Some((dir, cos_threshold)) = normal_cos {
                let positions: Vec<Vec3> = face
                    .vertices(self)
                    .filter_map(|v| v.position(self).ok())
                    .collect();
                if positions.len() >= 3 {
                    let e1 = positions[1] - positions[0];
                    let e2 = positions[2] - positions[0];
                    let normal = e1.cross(e2).normalize_or_zero();
                    if normal.dot(dir) < cos_threshold {
                        continue;
                    }
                }
            }

            selected.push(face);

            // Enqueue neighboring faces (faces sharing an edge)
            for he in face.halfedges(self) {
                if let Ok(opp) = he.opposite().run(self) {
                    if let Ok(neighbor_face) = opp.face().run(self) {
                        if visited.insert(neighbor_face) {
                            queue.push_back(neighbor_face);
                        }
                    }
                }
            }
        }

        selected
    }

    /// Cast a ray and return the closest face hit.
    ///
    /// Uses Möller–Trumbore intersection for triangulated faces.
    pub fn raycast(&self, origin: Vec3, direction: Vec3) -> Option<RaycastHit> {
        let dir = direction.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }

        let mut closest: Option<RaycastHit> = None;

        for face in self.faces() {
            let positions: Vec<Vec3> = face
                .vertices(self)
                .filter_map(|v| v.position(self).ok())
                .collect();
            if positions.len() < 3 {
                continue;
            }

            // Fan-triangulate the face and test each triangle
            for i in 1..positions.len() - 1 {
                if let Some((t, point)) =
                    ray_triangle_intersection(origin, dir, positions[0], positions[i], positions[i + 1])
                {
                    if t > 0.0 && closest.map_or(true, |c| t < c.distance) {
                        let e1 = positions[i] - positions[0];
                        let e2 = positions[i + 1] - positions[0];
                        let normal = e1.cross(e2).normalize_or_zero();
                        closest = Some(RaycastHit {
                            face,
                            point,
                            distance: t,
                            normal,
                        });
                    }
                }
            }
        }

        closest
    }
}

/// Möller–Trumbore ray-triangle intersection.
/// Returns (t, hit_point) if the ray intersects the triangle.
fn ray_triangle_intersection(
    origin: Vec3,
    dir: Vec3,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
) -> Option<(f32, Vec3)> {
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let h = dir.cross(e2);
    let a = e1.dot(h);

    const EPSILON: f32 = 1e-7;
    if a.abs() < EPSILON {
        return None;
    }

    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * e2.dot(q);
    if t > EPSILON {
        Some((t, origin + dir * t))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use glam::vec3;

    use super::*;
    use crate::smesh::primitives::{Cube, Primitive};

    fn make_quad_mesh() -> (SMesh, VertexId, VertexId, VertexId, VertexId, FaceId) {
        let mut mesh = SMesh::new();
        // CCW winding when viewed from above -> normal points up (+Y)
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let f = mesh.make_quad(v0, v1, v2, v3).unwrap();
        (mesh, v0, v1, v2, v3, f)
    }

    #[test]
    fn test_vertices_where_above_y() {
        let (mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()
        .unwrap();
        let sel = mesh.vertices_where(|pos| pos.y > 0.0);
        let verts = sel.resolve_to_vertices(&mesh).unwrap();
        assert_eq!(verts.len(), 4);
    }

    #[test]
    fn test_vertices_where_all() {
        let (mesh, _, _, _, _, _) = make_quad_mesh();
        let sel = mesh.vertices_where(|_| true);
        let verts = sel.resolve_to_vertices(&mesh).unwrap();
        assert_eq!(verts.len(), 4);
    }

    #[test]
    fn test_vertices_where_none() {
        let (mesh, _, _, _, _, _) = make_quad_mesh();
        let sel = mesh.vertices_where(|_| false);
        let verts = sel.resolve_to_vertices(&mesh).unwrap();
        assert_eq!(verts.len(), 0);
    }

    #[test]
    fn test_vertices_where_with_transform() -> SMeshResult<()> {
        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let sel = mesh.vertices_where(|pos| pos.y > 0.0);
        let top_verts: Vec<VertexId> = sel.resolve_to_vertices(&mesh)?.into_iter().collect();

        // Move top vertices up
        for &v in &top_verts {
            let pos = v.position(&mesh)?;
            mesh.positions.insert(v, pos + vec3(0.0, 1.0, 0.0));
        }

        // Verify positions changed
        for &v in &top_verts {
            let pos = v.position(&mesh)?;
            assert!(pos.y > 1.0, "Top vertex should have been moved up");
        }
        Ok(())
    }

    #[test]
    fn nearest_vertex_basic() {
        let (mesh, v0, _, _, _, _) = make_quad_mesh();
        let (v, dist) = mesh.nearest_vertex(vec3(-1.1, 0.0, -1.1)).unwrap();
        assert_eq!(v, v0);
        assert!(dist < 0.2);
    }

    #[test]
    fn query_region_finds_nearby() {
        let (mesh, _, _, _, _, _) = make_quad_mesh();
        // All 4 vertices are within 1.5 of origin
        let sel = mesh.query_region(Vec3::ZERO, 1.5);
        let verts = sel.resolve_to_vertices(&mesh).unwrap();
        assert_eq!(verts.len(), 4);

        // Only vertices near (-1, 0, -1) within radius 0.5
        let sel2 = mesh.query_region(vec3(-1.0, 0.0, -1.0), 0.5);
        let verts2 = sel2.resolve_to_vertices(&mesh).unwrap();
        assert_eq!(verts2.len(), 1);
    }

    #[test]
    fn faces_facing_up() {
        let (mesh, _, _, _, _, f) = make_quad_mesh();
        // The quad lies in the XZ plane, normal should point up (Y+)
        let up_faces = mesh.faces_facing(Vec3::Y, FRAC_PI_4);
        assert!(up_faces.contains(&f));

        // Should not face down
        let down_faces = mesh.faces_facing(Vec3::NEG_Y, FRAC_PI_4);
        assert!(!down_faces.contains(&f));
    }

    #[test]
    fn raycast_hits_quad() {
        let (mesh, _, _, _, _, f) = make_quad_mesh();
        // Ray from above pointing down
        let hit = mesh.raycast(vec3(0.0, 5.0, 0.0), Vec3::NEG_Y);
        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.face, f);
        assert!((hit.distance - 5.0).abs() < 1e-4);
        assert!((hit.point.y).abs() < 1e-4);
    }

    #[test]
    fn raycast_misses() {
        let (mesh, _, _, _, _, _) = make_quad_mesh();
        // Ray pointing away from the mesh
        let hit = mesh.raycast(vec3(0.0, 5.0, 0.0), Vec3::Y);
        assert!(hit.is_none());
    }

    #[test]
    fn nearest_vertex_empty_mesh() {
        let mesh = SMesh::new();
        assert!(mesh.nearest_vertex(Vec3::ZERO).is_none());
    }

    #[test]
    fn select_region_basic() -> SMeshResult<()> {
        let (mesh, _) = Cube {
            subdivision: glam::U16Vec3::splat(2),
        }
        .generate()?;

        // Large radius should get all faces
        let all = mesh.select_region(Vec3::ZERO, 10.0, None);
        assert_eq!(all.len(), mesh.faces().count());

        // Small radius centered on one face should get fewer
        let some = mesh.select_region(vec3(0.0, 1.0, 0.0), 0.8, None);
        assert!(some.len() < mesh.faces().count());
        assert!(!some.is_empty());

        Ok(())
    }

    #[test]
    fn select_region_with_normal_filter() -> SMeshResult<()> {
        let (mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        // Select only upward-facing faces near the top
        let top_faces = mesh.select_region(
            vec3(0.0, 1.0, 0.0),
            2.0,
            Some((Vec3::Y, FRAC_PI_4)),
        );

        // Should only get the top face of the cube
        assert_eq!(top_faces.len(), 1);

        // All selected faces should face up
        for f in &top_faces {
            let positions: Vec<Vec3> = f
                .vertices(&mesh)
                .filter_map(|v| v.position(&mesh).ok())
                .collect();
            let e1 = positions[1] - positions[0];
            let e2 = positions[2] - positions[0];
            let normal = e1.cross(e2).normalize_or_zero();
            assert!(normal.dot(Vec3::Y) > 0.5);
        }

        Ok(())
    }

    #[test]
    fn select_region_empty_mesh() {
        let mesh = SMesh::new();
        let result = mesh.select_region(Vec3::ZERO, 1.0, None);
        assert!(result.is_empty());
    }

    #[test]
    fn faces_facing_with_multiple_faces() -> SMeshResult<()> {
        let mut mesh = SMesh::new();
        // Horizontal quad (CCW from above -> normal up)
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let f_floor = mesh.make_quad(v0, v1, v2, v3)?;

        // Vertical quad (normal in +X)
        let v4 = mesh.add_vertex(vec3(2.0, 0.0, -1.0));
        let v5 = mesh.add_vertex(vec3(2.0, 2.0, -1.0));
        let v6 = mesh.add_vertex(vec3(2.0, 2.0, 1.0));
        let v7 = mesh.add_vertex(vec3(2.0, 0.0, 1.0));
        let f_wall = mesh.make_quad(v4, v5, v6, v7)?;

        let up = mesh.faces_facing(Vec3::Y, FRAC_PI_4);
        assert!(up.contains(&f_floor));
        assert!(!up.contains(&f_wall));

        Ok(())
    }
}
