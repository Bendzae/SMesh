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
