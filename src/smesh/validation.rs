use std::collections::HashSet;
use std::fmt;

use glam::Vec3;

use crate::prelude::*;

/// A single issue found during mesh validation.
#[derive(Debug, Clone)]
pub enum MeshIssue {
    /// Face has near-zero area.
    DegenerateFace { face: FaceId, area: f32 },
    /// Vertex has more than one boundary gap (non-manifold topology).
    NonManifoldVertex { vertex: VertexId },
    /// Two vertices are closer than the threshold distance.
    DuplicateVertices {
        v1: VertexId,
        v2: VertexId,
        distance: f32,
    },
    /// Edge has near-zero length.
    ZeroLengthEdge {
        halfedge: HalfedgeId,
        src: VertexId,
        dst: VertexId,
        length: f32,
    },
    /// Vertex has no connected edges or faces.
    IsolatedVertex { vertex: VertexId },
    /// A quad or n-gon face whose vertices deviate significantly from a plane.
    NonPlanarFace { face: FaceId, deviation: f32 },
    /// A face whose computed normal points opposite to most of its neighbors.
    InconsistentWinding { face: FaceId },
    /// Halfedge connectivity is broken (missing opposite, next, or prev).
    BrokenConnectivity {
        halfedge: HalfedgeId,
        detail: String,
    },
}

impl fmt::Display for MeshIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshIssue::DegenerateFace { face, area } => {
                write!(f, "Degenerate face {:?}: area={:.6}", face, area)
            }
            MeshIssue::NonManifoldVertex { vertex } => {
                write!(f, "Non-manifold vertex {:?}", vertex)
            }
            MeshIssue::DuplicateVertices { v1, v2, distance } => {
                write!(
                    f,
                    "Duplicate vertices {:?} and {:?}: distance={:.6}",
                    v1, v2, distance
                )
            }
            MeshIssue::ZeroLengthEdge {
                halfedge,
                src,
                dst,
                length,
            } => {
                write!(
                    f,
                    "Zero-length edge {:?} ({:?} -> {:?}): length={:.6}",
                    halfedge, src, dst, length
                )
            }
            MeshIssue::IsolatedVertex { vertex } => {
                write!(f, "Isolated vertex {:?}", vertex)
            }
            MeshIssue::NonPlanarFace { face, deviation } => {
                write!(
                    f,
                    "Non-planar face {:?}: max deviation={:.4}",
                    face, deviation
                )
            }
            MeshIssue::InconsistentWinding { face } => {
                write!(f, "Inconsistent winding on face {:?}", face)
            }
            MeshIssue::BrokenConnectivity { halfedge, detail } => {
                write!(
                    f,
                    "Broken connectivity at halfedge {:?}: {}",
                    halfedge, detail
                )
            }
        }
    }
}

/// Result of mesh validation.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub issues: Vec<MeshIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.is_empty() {
            writeln!(f, "Mesh is valid: no issues found.")?;
        } else {
            writeln!(f, "Mesh validation found {} issue(s):", self.issues.len())?;
            for (i, issue) in self.issues.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, issue)?;
            }
        }
        Ok(())
    }
}

/// Thresholds for mesh validation.
pub struct ValidationOptions {
    /// Faces with area below this are considered degenerate.
    pub degenerate_face_area: f32,
    /// Edges shorter than this are considered zero-length.
    pub zero_length_edge: f32,
    /// Vertices closer than this are considered duplicates.
    pub duplicate_vertex_distance: f32,
    /// Faces with vertex deviation from plane above this are non-planar (only for quads+).
    pub non_planar_threshold: f32,
    /// Check for duplicate vertices (O(n^2), can be slow on large meshes).
    pub check_duplicate_vertices: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            degenerate_face_area: 1e-7,
            zero_length_edge: 1e-6,
            duplicate_vertex_distance: 1e-5,
            non_planar_threshold: 0.01,
            check_duplicate_vertices: true,
        }
    }
}

impl SMesh {
    /// Validate the mesh and return all issues found.
    pub fn validate(&self) -> ValidationReport {
        self.validate_with_options(&ValidationOptions::default())
    }

    /// Validate the mesh with custom thresholds.
    pub fn validate_with_options(&self, opts: &ValidationOptions) -> ValidationReport {
        let mut issues = Vec::new();

        self.validate_connectivity(&mut issues);
        self.validate_vertices(opts, &mut issues);
        self.validate_edges(opts, &mut issues);
        self.validate_faces(opts, &mut issues);
        self.validate_winding(&mut issues);

        if opts.check_duplicate_vertices {
            self.validate_duplicate_vertices(opts, &mut issues);
        }

        ValidationReport { issues }
    }

    fn validate_connectivity(&self, issues: &mut Vec<MeshIssue>) {
        for he in self.halfedges() {
            if he.opposite().run(self).is_err() {
                issues.push(MeshIssue::BrokenConnectivity {
                    halfedge: he,
                    detail: "missing opposite halfedge".to_string(),
                });
            }
            if he.next().run(self).is_err() && !he.is_boundary(self) {
                issues.push(MeshIssue::BrokenConnectivity {
                    halfedge: he,
                    detail: "non-boundary halfedge has no next".to_string(),
                });
            }
            if he.prev().run(self).is_err() && !he.is_boundary(self) {
                issues.push(MeshIssue::BrokenConnectivity {
                    halfedge: he,
                    detail: "non-boundary halfedge has no prev".to_string(),
                });
            }
        }
    }

    fn validate_vertices(&self, _opts: &ValidationOptions, issues: &mut Vec<MeshIssue>) {
        for v in self.vertices() {
            if v.is_isolated(self) {
                issues.push(MeshIssue::IsolatedVertex { vertex: v });
            } else if !v.is_manifold(self) {
                issues.push(MeshIssue::NonManifoldVertex { vertex: v });
            }
        }
    }

    fn validate_edges(&self, opts: &ValidationOptions, issues: &mut Vec<MeshIssue>) {
        let mut checked: HashSet<(HalfedgeId, HalfedgeId)> = HashSet::new();

        for he in self.halfedges() {
            let Ok(opp) = he.opposite().run(self) else {
                continue;
            };
            // Only check each edge pair once
            let key = if he < opp { (he, opp) } else { (opp, he) };
            if !checked.insert(key) {
                continue;
            }

            let Ok(src) = he.src_vert().run(self) else {
                continue;
            };
            let Ok(dst) = he.dst_vert().run(self) else {
                continue;
            };
            let Ok(p0) = src.position(self) else {
                continue;
            };
            let Ok(p1) = dst.position(self) else {
                continue;
            };
            let length = (p1 - p0).length();
            if length < opts.zero_length_edge {
                issues.push(MeshIssue::ZeroLengthEdge {
                    halfedge: he,
                    src,
                    dst,
                    length,
                });
            }
        }
    }

    fn validate_faces(&self, opts: &ValidationOptions, issues: &mut Vec<MeshIssue>) {
        for face in self.faces() {
            let verts: Vec<VertexId> = face.vertices(self).collect();
            let positions: Vec<Vec3> = verts
                .iter()
                .filter_map(|v| v.position(self).ok())
                .collect();

            if positions.len() < 3 {
                continue;
            }

            // Check degenerate face (area)
            let mut total_area = 0.0f32;
            for i in 1..positions.len() - 1 {
                let e1 = positions[i] - positions[0];
                let e2 = positions[i + 1] - positions[0];
                total_area += e1.cross(e2).length() * 0.5;
            }
            if total_area < opts.degenerate_face_area {
                issues.push(MeshIssue::DegenerateFace {
                    face,
                    area: total_area,
                });
            }

            // Check non-planar (only meaningful for quads and n-gons)
            if positions.len() > 3 {
                let e1 = positions[1] - positions[0];
                let e2 = positions[2] - positions[0];
                let normal = e1.cross(e2);
                let normal_len = normal.length();
                if normal_len > 1e-10 {
                    let normal = normal / normal_len;
                    let mut max_dev = 0.0f32;
                    for p in &positions[3..] {
                        let d = (*p - positions[0]).dot(normal).abs();
                        max_dev = max_dev.max(d);
                    }
                    if max_dev > opts.non_planar_threshold {
                        issues.push(MeshIssue::NonPlanarFace {
                            face,
                            deviation: max_dev,
                        });
                    }
                }
            }
        }
    }

    fn validate_winding(&self, issues: &mut Vec<MeshIssue>) {
        // For each face, compute its normal and compare with neighbors.
        // If a face's normal is opposite to most of its neighbors, flag it.
        let face_normals: Vec<(FaceId, Option<Vec3>)> = self
            .faces()
            .map(|face| {
                let positions: Vec<Vec3> = face
                    .vertices(self)
                    .filter_map(|v| v.position(self).ok())
                    .collect();
                if positions.len() < 3 {
                    return (face, None);
                }
                let e1 = positions[1] - positions[0];
                let e2 = positions[2] - positions[0];
                let n = e1.cross(e2).normalize_or_zero();
                if n == Vec3::ZERO {
                    (face, None)
                } else {
                    (face, Some(n))
                }
            })
            .collect();

        let normal_map: std::collections::HashMap<FaceId, Vec3> = face_normals
            .iter()
            .filter_map(|(f, n)| n.map(|n| (*f, n)))
            .collect();

        for (face, normal) in &face_normals {
            let Some(n) = normal else { continue };

            // Get neighboring faces via shared halfedges
            let mut agree = 0i32;
            let mut disagree = 0i32;

            for he in face.halfedges(self) {
                if let Ok(opp) = he.opposite().run(self) {
                    if let Ok(neighbor_face) = opp.face().run(self) {
                        if let Some(neighbor_n) = normal_map.get(&neighbor_face) {
                            if n.dot(*neighbor_n) > 0.0 {
                                agree += 1;
                            } else {
                                disagree += 1;
                            }
                        }
                    }
                }
            }

            // Only flag if there are neighbors and this face disagrees with the majority
            if disagree > 0 && disagree > agree {
                issues.push(MeshIssue::InconsistentWinding { face: *face });
            }
        }
    }

    fn validate_duplicate_vertices(
        &self,
        opts: &ValidationOptions,
        issues: &mut Vec<MeshIssue>,
    ) {
        let verts: Vec<(VertexId, Vec3)> = self
            .vertices()
            .filter_map(|v| v.position(self).ok().map(|p| (v, p)))
            .collect();

        let threshold_sq = opts.duplicate_vertex_distance * opts.duplicate_vertex_distance;
        for i in 0..verts.len() {
            for j in (i + 1)..verts.len() {
                let dist_sq = (verts[i].1 - verts[j].1).length_squared();
                if dist_sq < threshold_sq {
                    issues.push(MeshIssue::DuplicateVertices {
                        v1: verts[i].0,
                        v2: verts[j].0,
                        distance: dist_sq.sqrt(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn valid_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let report = mesh.validate();
        // A single quad is valid (boundary is fine, just no degenerate faces etc.)
        let text = format!("{}", report);
        assert!(!text.contains("Degenerate"));
        assert!(!text.contains("Zero-length"));
        Ok(())
    }

    #[test]
    fn detects_isolated_vertex() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_triangle(v1, v2, v3)?;

        let report = mesh.validate();
        assert!(report.issues.iter().any(|i| matches!(i, MeshIssue::IsolatedVertex { .. })));
        Ok(())
    }

    #[test]
    fn detects_degenerate_face() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        // v2 is collinear with v0-v1 -> degenerate triangle
        let v2 = mesh.add_vertex(vec3(0.5, 0.0, 0.0));
        mesh.make_triangle(v0, v1, v2)?;

        let report = mesh.validate();
        assert!(report.issues.iter().any(|i| matches!(i, MeshIssue::DegenerateFace { .. })));
        Ok(())
    }

    #[test]
    fn detects_duplicate_vertices() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        // v3 is nearly identical to v0
        let _v3 = mesh.add_vertex(vec3(0.0, 0.0, 1e-7));
        mesh.make_triangle(v0, v1, v2)?;
        // v3 is isolated but also a near-duplicate
        let report = mesh.validate();
        assert!(report.issues.iter().any(|i| matches!(i, MeshIssue::DuplicateVertices { .. })));
        Ok(())
    }

    #[test]
    fn display_report() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        mesh.add_vertex(vec3(0.0, 0.0, 0.0)); // isolated
        let report = mesh.validate();
        let text = format!("{}", report);
        assert!(text.contains("issue"));
        Ok(())
    }

    #[test]
    fn valid_mesh_has_no_issues() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let report = mesh.validate();
        assert!(report.is_valid(), "Expected valid mesh, got: {}", report);
        Ok(())
    }
}
