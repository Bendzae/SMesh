//! Structured reports for inspecting a mesh and its selections.
//!
//! These reports are computed on demand, contain no references to the mesh,
//! and both `Debug` and `Display` cleanly — suitable for logging, LLM context,
//! or human inspection. Prefer them over ad-hoc `println!` debugging.
//!
//! - [`SMesh::describe`] — whole-mesh summary (counts, bounds, topology flags).
//! - [`SMesh::describe_selection`] — per-tag / per-selection summary.
//! - [`SMesh::describe_faces`] — per-face centroid/area/normal list.

use std::collections::{HashSet, VecDeque};
use std::fmt;

use glam::Vec3;

use crate::prelude::*;

/// Structured summary of a whole mesh — counts, bounds, and topology flags.
///
/// Produced by [`SMesh::describe`]. Implements [`Display`](fmt::Display) for
/// human-readable output.
#[derive(Debug, Clone)]
pub struct MeshReport {
    /// Total vertices, including isolated ones.
    pub vertex_count: usize,
    /// Total faces.
    pub face_count: usize,
    /// Undirected edges (`halfedge_count / 2`).
    pub edge_count: usize,
    /// Directed halfedges (two per edge).
    pub halfedge_count: usize,
    /// `(min, max)` corner of the axis-aligned bounding box.
    pub bounding_box: (Vec3, Vec3),
    /// `bounding_box.1 - bounding_box.0`.
    pub dimensions: Vec3,
    /// Arithmetic mean of all vertex positions.
    pub center_of_mass: Vec3,
    /// `true` iff the mesh has no boundary and no isolated vertices.
    pub is_closed: bool,
    /// `true` iff every vertex has at most one boundary gap (disc one-ring).
    pub is_manifold: bool,
    /// `true` if any vertex has no incident halfedge.
    pub has_isolated_vertices: bool,
    /// Counts grouped by polygon type.
    pub face_type_breakdown: FaceTypeBreakdown,
    /// Number of distinct boundary loops (holes plus outer boundaries).
    pub boundary_loops: usize,
    /// Number of connected components in the vertex adjacency graph.
    pub connected_components: usize,
}

/// Counts of triangles, quads, and larger polygons in a mesh or selection.
#[derive(Debug, Clone, Default)]
pub struct FaceTypeBreakdown {
    /// Number of 3-sided faces.
    pub triangles: usize,
    /// Number of 4-sided faces.
    pub quads: usize,
    /// Number of faces with 5 or more vertices.
    pub ngons: usize,
}

/// Summary of a selection (tagged region, query result, etc.).
///
/// Produced by [`SMesh::describe_selection`]. Counts, bounds, and face-type
/// breakdown are restricted to elements resolvable from the selection.
#[derive(Debug, Clone)]
pub struct SelectionReport {
    /// Vertices resolvable from the selection.
    pub vertex_count: usize,
    /// Faces resolvable from the selection.
    pub face_count: usize,
    /// `(min, max)` of the selected vertex positions.
    pub bounding_box: (Vec3, Vec3),
    /// Bounding-box size.
    pub dimensions: Vec3,
    /// Mean of the selected vertex positions.
    pub center: Vec3,
    /// Counts grouped by polygon type.
    pub face_type_breakdown: FaceTypeBreakdown,
}

impl fmt::Display for SelectionReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Selection: {} vertices, {} faces",
            self.vertex_count, self.face_count,
        )?;
        writeln!(
            f,
            "  Bounding box: ({:.3}, {:.3}, {:.3}) to ({:.3}, {:.3}, {:.3})",
            self.bounding_box.0.x,
            self.bounding_box.0.y,
            self.bounding_box.0.z,
            self.bounding_box.1.x,
            self.bounding_box.1.y,
            self.bounding_box.1.z,
        )?;
        writeln!(
            f,
            "  Dimensions: {:.3} x {:.3} x {:.3}",
            self.dimensions.x, self.dimensions.y, self.dimensions.z,
        )?;
        writeln!(
            f,
            "  Center: ({:.3}, {:.3}, {:.3})",
            self.center.x, self.center.y, self.center.z,
        )?;
        writeln!(
            f,
            "  Faces: {} tris, {} quads, {} ngons",
            self.face_type_breakdown.triangles,
            self.face_type_breakdown.quads,
            self.face_type_breakdown.ngons,
        )?;
        Ok(())
    }
}

/// Per-face geometry record — centroid, normal, area, and vertex count.
///
/// Returned in a `Vec` by [`SMesh::describe_faces`]. Useful for targeted
/// debugging or for filtering faces by spatial predicate.
#[derive(Debug, Clone)]
pub struct FaceReport {
    /// The face this record describes.
    pub face: FaceId,
    /// Arithmetic mean of the face's vertex positions.
    pub centroid: Vec3,
    /// Geometric normal, or `None` if the face is degenerate.
    pub normal: Option<Vec3>,
    /// Face area (sum of triangle-fan triangles).
    pub area: f32,
    /// Number of vertices bounding the face.
    pub vertex_count: usize,
}

impl fmt::Display for MeshReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Mesh Report ===")?;
        writeln!(
            f,
            "Elements: {} vertices, {} faces, {} edges",
            self.vertex_count, self.face_count, self.edge_count
        )?;
        writeln!(
            f,
            "Faces: {} tris, {} quads, {} ngons",
            self.face_type_breakdown.triangles,
            self.face_type_breakdown.quads,
            self.face_type_breakdown.ngons
        )?;
        writeln!(
            f,
            "Bounding box: ({:.3}, {:.3}, {:.3}) to ({:.3}, {:.3}, {:.3})",
            self.bounding_box.0.x,
            self.bounding_box.0.y,
            self.bounding_box.0.z,
            self.bounding_box.1.x,
            self.bounding_box.1.y,
            self.bounding_box.1.z
        )?;
        writeln!(
            f,
            "Dimensions: {:.3} x {:.3} x {:.3}",
            self.dimensions.x, self.dimensions.y, self.dimensions.z
        )?;
        writeln!(
            f,
            "Center of mass: ({:.3}, {:.3}, {:.3})",
            self.center_of_mass.x, self.center_of_mass.y, self.center_of_mass.z
        )?;
        writeln!(
            f,
            "Topology: closed={}, manifold={}, boundary_loops={}, components={}",
            self.is_closed, self.is_manifold, self.boundary_loops, self.connected_components
        )?;
        if self.has_isolated_vertices {
            writeln!(f, "Warning: mesh has isolated vertices")?;
        }
        Ok(())
    }
}

impl fmt::Display for FaceReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Face {:?}: centroid=({:.3}, {:.3}, {:.3}), area={:.4}, verts={}",
            self.face,
            self.centroid.x,
            self.centroid.y,
            self.centroid.z,
            self.area,
            self.vertex_count,
        )?;
        if let Some(n) = self.normal {
            write!(f, ", normal=({:.3}, {:.3}, {:.3})", n.x, n.y, n.z)?;
        }
        Ok(())
    }
}

impl SMesh {
    /// Summarise the entire mesh in a structured [`MeshReport`].
    ///
    /// Runs in O(V + F + E) time. The report is self-contained (no references
    /// into the mesh) and safe to log, serialise, or hand off to tests.
    pub fn describe(&self) -> MeshReport {
        let vertex_count = self.vertices().len();
        let face_count = self.faces().len();
        let halfedge_count = self.halfedges().len();
        let edge_count = halfedge_count / 2;

        // Bounding box and center of mass
        let (bb_min, bb_max, center) = if vertex_count > 0 {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            let mut sum = Vec3::ZERO;
            let mut count = 0u32;
            for v in self.vertices() {
                if let Some(&pos) = self.positions.get(v) {
                    min = min.min(pos);
                    max = max.max(pos);
                    sum += pos;
                    count += 1;
                }
            }
            if count > 0 {
                (min, max, sum / count as f32)
            } else {
                (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
            }
        } else {
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
        };

        // Face type breakdown
        let mut breakdown = FaceTypeBreakdown::default();
        for face in self.faces() {
            match face.valence(self) {
                3 => breakdown.triangles += 1,
                4 => breakdown.quads += 1,
                _ => breakdown.ngons += 1,
            }
        }

        // Boundary detection and manifold check
        let mut boundary_halfedges: HashSet<HalfedgeId> = HashSet::new();
        let mut is_manifold = true;
        let mut has_isolated = false;

        for v in self.vertices() {
            if v.is_isolated(self) {
                has_isolated = true;
            } else if !v.is_manifold(self) {
                is_manifold = false;
            }
        }

        for he in self.halfedges() {
            if he.is_boundary(self) {
                boundary_halfedges.insert(he);
            }
        }

        let is_closed = boundary_halfedges.is_empty() && !has_isolated;

        // Count boundary loops
        let boundary_loops = count_boundary_loops(self, &boundary_halfedges);

        // Count connected components via vertex adjacency
        let connected_components = count_connected_components(self);

        MeshReport {
            vertex_count,
            face_count,
            edge_count,
            halfedge_count,
            bounding_box: (bb_min, bb_max),
            dimensions: bb_max - bb_min,
            center_of_mass: center,
            is_closed,
            is_manifold,
            has_isolated_vertices: has_isolated,
            face_type_breakdown: breakdown,
            boundary_loops,
            connected_components,
        }
    }

    /// Summarise a subset of the mesh.
    ///
    /// Typical callers pass a tag (`mesh.get_tag("backrest").unwrap().clone()`),
    /// a `Vec<VertexId>`, or any other type convertible to a
    /// [`MeshSelection`]. Vertices and faces that cannot be resolved from the
    /// selection are ignored.
    pub fn describe_selection<S: Into<MeshSelection>>(&self, selection: S) -> SelectionReport {
        let sel = selection.into();
        let vertices = sel.resolve_to_vertices(self).unwrap_or_default();
        let faces = sel.resolve_to_faces(self).unwrap_or_default();

        let vertex_count = vertices.len();
        let face_count = faces.len();

        let (bb_min, bb_max, center) = if !vertices.is_empty() {
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            let mut sum = Vec3::ZERO;
            let mut count = 0u32;
            for &v in &vertices {
                if let Some(&pos) = self.positions.get(v) {
                    min = min.min(pos);
                    max = max.max(pos);
                    sum += pos;
                    count += 1;
                }
            }
            if count > 0 {
                (min, max, sum / count as f32)
            } else {
                (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
            }
        } else {
            (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO)
        };

        let mut breakdown = FaceTypeBreakdown::default();
        for &face in &faces {
            match face.valence(self) {
                3 => breakdown.triangles += 1,
                4 => breakdown.quads += 1,
                _ => breakdown.ngons += 1,
            }
        }

        SelectionReport {
            vertex_count,
            face_count,
            bounding_box: (bb_min, bb_max),
            dimensions: bb_max - bb_min,
            center: center,
            face_type_breakdown: breakdown,
        }
    }

    /// Produce one [`FaceReport`] per face — centroid, normal, and area.
    ///
    /// Faces with fewer than three usable vertex positions return `normal =
    /// None` and `area = 0.0`. Area is computed as the sum of triangle-fan
    /// triangles, which is exact for triangles/quads and a reasonable
    /// approximation for planar n-gons.
    pub fn describe_faces(&self) -> Vec<FaceReport> {
        self.faces()
            .map(|face| {
                let verts: Vec<VertexId> = face.vertices(self).collect();
                let vertex_count = verts.len();

                let centroid = self.get_face_centroid(face).unwrap_or(Vec3::ZERO);

                // Compute face normal and area from first 3 vertices
                let (normal, area) = if vertex_count >= 3 {
                    let positions: Vec<Vec3> = verts
                        .iter()
                        .filter_map(|v| v.position(self).ok())
                        .collect();
                    if positions.len() >= 3 {
                        // For triangles, exact area. For polygons, sum triangle fan areas.
                        let mut total_area = 0.0f32;
                        let mut total_normal = Vec3::ZERO;
                        for i in 1..positions.len() - 1 {
                            let e1 = positions[i] - positions[0];
                            let e2 = positions[i + 1] - positions[0];
                            let cross = e1.cross(e2);
                            total_normal += cross;
                            total_area += cross.length() * 0.5;
                        }
                        let n = total_normal.normalize_or_zero();
                        let normal = if n == Vec3::ZERO { None } else { Some(n) };
                        (normal, total_area)
                    } else {
                        (None, 0.0)
                    }
                } else {
                    (None, 0.0)
                };

                FaceReport {
                    face,
                    centroid,
                    normal,
                    area,
                    vertex_count,
                }
            })
            .collect()
    }
}

fn count_boundary_loops(mesh: &SMesh, boundary_halfedges: &HashSet<HalfedgeId>) -> usize {
    let mut visited: HashSet<HalfedgeId> = HashSet::new();
    let mut loops = 0;

    for &he in boundary_halfedges {
        if visited.contains(&he) {
            continue;
        }
        // Walk this boundary loop
        let mut current = he;
        loop {
            visited.insert(current);
            // Next boundary halfedge: follow next pointers staying on boundary
            let Ok(next) = current.next().run(mesh) else {
                break;
            };
            current = next;
            if current == he {
                break;
            }
        }
        loops += 1;
    }
    loops
}

fn count_connected_components(mesh: &SMesh) -> usize {
    let all_verts: Vec<VertexId> = mesh.vertices().collect();
    if all_verts.is_empty() {
        return 0;
    }

    let mut visited: HashSet<VertexId> = HashSet::new();
    let mut components = 0;

    for &start in &all_verts {
        if visited.contains(&start) {
            continue;
        }
        components += 1;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(v) = queue.pop_front() {
            for neighbor in v.vertices(mesh) {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }
    components
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn describe_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let report = mesh.describe();
        assert_eq!(report.vertex_count, 4);
        assert_eq!(report.face_count, 1);
        assert_eq!(report.edge_count, 4);
        assert_eq!(report.face_type_breakdown.quads, 1);
        assert_eq!(report.face_type_breakdown.triangles, 0);
        assert!(!report.is_closed);
        assert!(report.is_manifold);
        assert_eq!(report.boundary_loops, 1);
        assert_eq!(report.connected_components, 1);

        // Display should not panic
        let text = format!("{}", report);
        assert!(text.contains("4 vertices"));
        assert!(text.contains("1 faces"));

        Ok(())
    }

    #[test]
    fn describe_faces_single_tri() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_triangle(v0, v1, v2)?;

        let faces = mesh.describe_faces();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].vertex_count, 3);
        assert!(faces[0].area > 0.0);
        assert!(faces[0].normal.is_some());

        Ok(())
    }

    #[test]
    fn describe_empty_mesh() {
        let mesh = SMesh::new();
        let report = mesh.describe();
        assert_eq!(report.vertex_count, 0);
        assert_eq!(report.face_count, 0);
        assert_eq!(report.connected_components, 0);
    }

    #[test]
    fn describe_isolated_vertex() {
        let mesh = &mut SMesh::new();
        mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let report = mesh.describe();
        assert!(report.has_isolated_vertices);
        assert!(!report.is_closed);
        assert_eq!(report.connected_components, 1);
    }

    #[test]
    fn describe_two_components() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        // Triangle 1
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_triangle(v0, v1, v2)?;
        // Triangle 2 (disconnected)
        let v3 = mesh.add_vertex(vec3(5.0, 0.0, 0.0));
        let v4 = mesh.add_vertex(vec3(6.0, 0.0, 0.0));
        let v5 = mesh.add_vertex(vec3(5.0, 1.0, 0.0));
        mesh.make_triangle(v3, v4, v5)?;

        let report = mesh.describe();
        assert_eq!(report.connected_components, 2);

        Ok(())
    }
}
