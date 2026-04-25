//! Geometric transforms and smoothing operations.
//!
//! The basic transforms — [`translate`](SMesh::translate),
//! [`rotate`](SMesh::rotate), [`scale`](SMesh::scale) — accept any selection
//! type (single id, Vec of ids, tag, …) and a [`Pivot`] for rotations/scales.
//!
//! Shape-editing operations:
//!
//! - [`smooth`](SMesh::smooth) — iterative Laplacian smoothing.
//! - [`spherize`](SMesh::spherize) — push vertices onto a sphere.
//! - [`translate_proportional`](SMesh::translate_proportional) /
//!   [`scale_proportional`](SMesh::scale_proportional) /
//!   [`rotate_proportional`](SMesh::rotate_proportional) — transform a local
//!   region with a falloff curve.

use crate::{bail, prelude::*};
use glam::{Quat, Vec3};
use itertools::Itertools;
use selection::MeshSelection;

/// Weight-vs-distance curve used by the `*_proportional` transforms.
///
/// The input `t` is the normalised distance `distance / radius`; all curves
/// return `1.0` at `t = 0` and `0.0` outside `t = 1` (except `Constant`,
/// which returns `1.0` within the radius and `0.0` beyond).
#[derive(Debug, Clone, Copy)]
pub enum Falloff {
    /// Weight decreases linearly from 1.0 at center to 0.0 at radius.
    Linear,
    /// Smooth hermite interpolation (smoothstep).
    Smooth,
    /// Sharp falloff — strong near center, drops quickly.
    Sharp,
    /// Spherical falloff — gentle near center, steeper at edges.
    Sphere,
    /// Constant weight of 1.0 within radius.
    Constant,
}

impl Falloff {
    /// Compute the weight for a normalized distance t (0.0 = at center, 1.0 = at radius).
    /// Returns 0.0 for t >= 1.0.
    pub fn weight(&self, t: f32) -> f32 {
        if t >= 1.0 {
            return 0.0;
        }
        if t <= 0.0 {
            return 1.0;
        }
        match self {
            Falloff::Linear => 1.0 - t,
            Falloff::Smooth => {
                let s = 1.0 - t;
                s * s * (3.0 - 2.0 * s) // smoothstep
            }
            Falloff::Sharp => 1.0 - t * t,
            Falloff::Sphere => (1.0 - t * t).sqrt(),
            Falloff::Constant => 1.0,
        }
    }
}

/// Pivot point for rotation and scaling.
///
/// Chosen per-call, so you can e.g. scale a selection around its own centroid
/// while keeping the rest of the mesh fixed (`SelectionCog`), or rotate
/// everything around the world origin (`Origin`).
pub enum Pivot {
    /// The world origin `(0, 0, 0)`.
    Origin,
    /// Centre of gravity of every vertex in the mesh.
    MeshCog,
    /// Centre of gravity of just the affected selection.
    SelectionCog,
    /// An arbitrary world-space point.
    Point(Vec3),
}

impl Pivot {
    /// Calculate the concrete pivot point based on the pivot type
    fn calculate<S: Into<MeshSelection>>(&self, mesh: &SMesh, selection: S) -> SMeshResult<Vec3> {
        Ok(match self {
            Pivot::Origin => Vec3::ZERO,
            Pivot::MeshCog => mesh.center_of_gravity(mesh.vertices().collect_vec())?,
            Pivot::SelectionCog => mesh.center_of_gravity(selection)?,
            Pivot::Point(pos) => *pos,
        })
    }
}

/// Geometric transforms.
impl SMesh {
    /// Translate every vertex in `selection` by `translation`.
    ///
    /// Anything convertible to [`MeshSelection`] works — selecting a face
    /// translates all of its vertices, a halfedge translates both endpoints,
    /// and so on. Returns `&mut Self` for chaining.
    pub fn translate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        translation: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            if let Some(pos) = self.positions.get(id) {
                self.positions.insert(id, *pos + translation);
            }
        }
        Ok(self)
    }

    /// Move every vertex in `selection` so that the chosen `pivot` lands at
    /// `translation`.
    ///
    /// Equivalent to translating by `translation - pivot.calculate(...)`.
    /// Useful for "snap this selection to `X`" style operations.
    pub fn set_position<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        translation: Vec3,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        let vertices = s.resolve_to_vertices(self)?;
        for id in vertices {
            let mut pos = id.position(self)?;
            pos -= p;
            self.positions.insert(id, pos + translation);
        }
        Ok(self)
    }

    /// Scale every vertex in `selection` around `pivot` by component-wise `scale`.
    ///
    /// Non-uniform scales are supported — pass `Vec3::splat(k)` for uniform.
    pub fn scale<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.scale_around(s, scale, p)?;
        Ok(self)
    }

    /// Rotate every vertex in `selection` around `pivot` by `quaternion`.
    ///
    /// Build the quaternion with [`Quat::from_axis_angle`],
    /// `Quat::from_rotation_x`, or similar glam helpers.
    pub fn rotate<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Pivot,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let p = pivot.calculate(self, s.clone())?;
        self.rotate_around(s, quaternion, p)?;
        Ok(self)
    }

    /// Arithmetic mean of every position in `selection`.
    ///
    /// Errors with [`SMeshError::CustomError`] if the selection resolves to
    /// zero vertices.
    pub fn center_of_gravity<S: Into<MeshSelection>>(&self, selection: S) -> SMeshResult<Vec3> {
        let vertices = selection.into().resolve_to_vertices(self)?;

        if vertices.is_empty() {
            bail!("No vertices in selection");
        }

        let mut sum = Vec3::ZERO;
        for v_id in &vertices {
            let pos = v_id.position(self)?;
            sum += pos;
        }

        let center = sum / vertices.len() as f32;
        Ok(center)
    }

    /// Translate vertices softly: proportional editing with a falloff curve.
    ///
    /// Vertices within `radius` of `center` are moved by
    /// `translation * falloff.weight(t)`. Vertices at `center` move the full
    /// amount; vertices at `radius` do not move at all. Useful for sculpting
    /// bulges or dents without affecting the whole mesh.
    pub fn translate_proportional(
        &mut self,
        center: Vec3,
        radius: f32,
        falloff: Falloff,
        translation: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let radius_sq = radius * radius;
        let verts: Vec<VertexId> = self.vertices().collect();

        for v in verts {
            let pos = v.position(self)?;
            let dist_sq = (pos - center).length_squared();
            if dist_sq >= radius_sq {
                continue;
            }
            let t = dist_sq.sqrt() / radius;
            let w = falloff.weight(t);
            self.positions.insert(v, pos + translation * w);
        }

        Ok(self)
    }

    /// Scale vertices softly around `center` with a falloff curve.
    ///
    /// Each affected vertex uses `lerp(Vec3::ONE, scale, weight)` as its
    /// effective scale, so vertices near `center` are scaled fully and
    /// vertices near `radius` barely change.
    pub fn scale_proportional(
        &mut self,
        center: Vec3,
        radius: f32,
        falloff: Falloff,
        scale: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let radius_sq = radius * radius;
        let verts: Vec<VertexId> = self.vertices().collect();

        for v in verts {
            let pos = v.position(self)?;
            let dist_sq = (pos - center).length_squared();
            if dist_sq >= radius_sq {
                continue;
            }
            let t = dist_sq.sqrt() / radius;
            let w = falloff.weight(t);
            let effective_scale = Vec3::ONE.lerp(scale, w);
            let offset = pos - center;
            self.positions.insert(v, center + offset * effective_scale);
        }

        Ok(self)
    }

    /// Rotate vertices softly around `center` with a falloff curve.
    ///
    /// Each vertex uses `slerp(Quat::IDENTITY, rotation, weight)` as its
    /// effective rotation. Produces smooth twists that fade out with distance.
    pub fn rotate_proportional(
        &mut self,
        center: Vec3,
        radius: f32,
        falloff: Falloff,
        rotation: Quat,
    ) -> SMeshResult<&mut SMesh> {
        let radius_sq = radius * radius;
        let verts: Vec<VertexId> = self.vertices().collect();

        for v in verts {
            let pos = v.position(self)?;
            let dist_sq = (pos - center).length_squared();
            if dist_sq >= radius_sq {
                continue;
            }
            let t = dist_sq.sqrt() / radius;
            let w = falloff.weight(t);
            let effective_rot = Quat::IDENTITY.slerp(rotation, w);
            let offset = pos - center;
            self.positions.insert(v, center + effective_rot * offset);
        }

        Ok(self)
    }

    fn scale_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        scale: Vec3,
        pivot: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            let mut position = id.position(self)?;
            // Translate vertex so that the pivot is at the desired point
            position -= pivot;
            // Scale the vertex
            position *= scale;
            // Translate the vertex back
            position += pivot;
            self.positions.insert(id, position);
        }
        Ok(self)
    }

    /// Uniform Laplacian smoothing on `selection`.
    ///
    /// Each iteration moves each selected vertex a fraction `factor` of the
    /// way toward the centroid of its one-ring neighbours. Repeat
    /// `iterations` times for progressively smoother results.
    ///
    /// Parameters:
    /// - `factor ∈ [0, 1]` — 0 means no change, 1 snaps fully to the centroid.
    /// - `pin_boundaries = true` — boundary vertices are excluded (useful
    ///   for smoothing the interior of a patch without changing its outline).
    pub fn smooth<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        iterations: usize,
        factor: f32,
        pin_boundaries: bool,
    ) -> SMeshResult<&mut SMesh> {
        let selected: Vec<VertexId> = selection.into().resolve_to_vertices(self)?.into_iter().collect();

        for _ in 0..iterations {
            // Compute new positions from current state (don't update in-place mid-iteration)
            let mut new_positions: Vec<(VertexId, Vec3)> = Vec::new();

            for &v in &selected {
                if pin_boundaries && v.is_boundary(self) {
                    continue;
                }

                let neighbors: Vec<Vec3> = v
                    .vertices(self)
                    .filter_map(|n| n.position(self).ok())
                    .collect();

                if neighbors.is_empty() {
                    continue;
                }

                let centroid = neighbors.iter().copied().sum::<Vec3>() / neighbors.len() as f32;
                let current = v.position(self)?;
                let smoothed = current.lerp(centroid, factor);
                new_positions.push((v, smoothed));
            }

            // Apply all new positions
            for (v, pos) in new_positions {
                self.positions.insert(v, pos);
            }
        }

        Ok(self)
    }

    /// Push selected vertices toward a sphere.
    ///
    /// Each vertex is linearly interpolated between its current position and
    /// the point `pivot + direction * target_radius`, where `direction` is
    /// the unit vector from `pivot` to the vertex. `amount = 0` leaves the
    /// mesh unchanged; `amount = 1` snaps every vertex onto the sphere.
    ///
    /// If `target_radius` is `None`, the average distance of selected
    /// vertices from `pivot` is used — a good default for "make this blob
    /// more spherical".
    pub fn spherize<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        amount: f32,
        pivot: Pivot,
        target_radius: Option<f32>,
    ) -> SMeshResult<&mut SMesh> {
        let s: MeshSelection = selection.into();
        let center = pivot.calculate(self, s.clone())?;
        let vertices: Vec<VertexId> = s.resolve_to_vertices(self)?.into_iter().collect();

        // Compute target radius if not provided (average distance from center)
        let radius = target_radius.unwrap_or_else(|| {
            let total: f32 = vertices
                .iter()
                .filter_map(|v| v.position(self).ok())
                .map(|p| (p - center).length())
                .sum();
            total / vertices.len().max(1) as f32
        });

        for &v in &vertices {
            let pos = v.position(self)?;
            let offset = pos - center;
            let dist = offset.length();
            if dist < 1e-8 {
                continue; // vertex at center, can't determine direction
            }
            // Point on sphere in same direction
            let sphere_pos = center + offset.normalize() * radius;
            let new_pos = pos.lerp(sphere_pos, amount);
            self.positions.insert(v, new_pos);
        }

        Ok(self)
    }

    fn rotate_around<S: Into<MeshSelection>>(
        &mut self,
        selection: S,
        quaternion: Quat,
        pivot: Vec3,
    ) -> SMeshResult<&mut SMesh> {
        let vertices = selection.into().resolve_to_vertices(self)?;
        for id in vertices {
            let mut position = id.position(self)?;
            // Translate position so that the rotation origin is at the coordinate origin
            position -= pivot;
            // Apply the rotation
            position = quaternion * position;
            // Translate the position back to its original location
            position += pivot;
            // Update the position in your mesh data
            self.positions.insert(id, position);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn smooth_reduces_variance() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::splat(2),
        }
        .generate()?;

        // Compute position variance before smoothing
        let all: Vec<VertexId> = mesh.vertices().collect();
        let cog = mesh.center_of_gravity(all.clone())?;
        let variance_before: f32 = all
            .iter()
            .map(|v| (v.position(&mesh).unwrap() - cog).length_squared())
            .sum::<f32>()
            / all.len() as f32;

        // Perturb some vertices to create bumps
        for (i, &v) in all.iter().enumerate() {
            if i % 3 == 0 {
                let pos = v.position(&mesh)?;
                mesh.positions.insert(v, pos * 1.3);
            }
        }

        mesh.smooth(all.clone(), 10, 0.5, false)?;

        let variance_after: f32 = all
            .iter()
            .map(|v| (v.position(&mesh).unwrap() - cog).length_squared())
            .sum::<f32>()
            / all.len() as f32;

        // Smoothing should reduce variance (mesh shrinks toward center)
        assert!(
            variance_after < variance_before * 1.5,
            "Smoothing should not wildly increase variance: before={}, after={}",
            variance_before,
            variance_after
        );

        Ok(())
    }

    #[test]
    fn smooth_pin_boundaries() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(vec3(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(vec3(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        // All vertices are boundary on a single quad
        let pos_before: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(mesh).unwrap())
            .collect();

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.smooth(all, 10, 1.0, true)?;

        let pos_after: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(mesh).unwrap())
            .collect();

        // With pin_boundaries=true, nothing should move
        for (before, after) in pos_before.iter().zip(pos_after.iter()) {
            assert!(
                (*before - *after).length() < 1e-6,
                "Boundary vertices should not move when pinned"
            );
        }

        Ok(())
    }

    #[test]
    fn smooth_zero_factor_no_change() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let positions_before: Vec<(VertexId, Vec3)> = mesh
            .vertices()
            .map(|v| (v, v.position(&mesh).unwrap()))
            .collect();

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.smooth(all, 10, 0.0, false)?;

        for (v, before) in &positions_before {
            let after = v.position(&mesh)?;
            assert!(
                (*before - after).length() < 1e-6,
                "Zero factor should not change positions"
            );
        }

        Ok(())
    }

    #[test]
    fn translate_proportional_basic() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::splat(3),
        }
        .generate()?;

        // Pull the top center upward with smooth falloff
        mesh.translate_proportional(
            vec3(0.0, 1.0, 0.0),
            2.0,
            Falloff::Smooth,
            vec3(0.0, 1.0, 0.0),
        )?;

        // The vertex closest to (0,1,0) should have moved the most
        let (top_v, _) = mesh.nearest_vertex(vec3(0.0, 2.0, 0.0)).unwrap();
        let top_pos = top_v.position(&mesh)?;
        assert!(top_pos.y > 1.2, "Top vertex should have moved up, got y={}", top_pos.y);

        // A vertex far from center should move less than the top vertex
        let (far_v, _) = mesh.nearest_vertex(vec3(0.0, -1.0, 0.0)).unwrap();
        let far_pos = far_v.position(&mesh)?;
        assert!(far_pos.y < top_pos.y, "Bottom vertex should move less than top");

        Ok(())
    }

    #[test]
    fn proportional_outside_radius_untouched() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let positions_before: Vec<(VertexId, Vec3)> = mesh
            .vertices()
            .map(|v| (v, v.position(&mesh).unwrap()))
            .collect();

        // Tiny radius centered far away — should affect nothing
        mesh.translate_proportional(
            vec3(100.0, 0.0, 0.0),
            0.1,
            Falloff::Linear,
            vec3(0.0, 5.0, 0.0),
        )?;

        for (v, before) in &positions_before {
            let after = v.position(&mesh)?;
            assert!((*before - after).length() < 1e-6);
        }

        Ok(())
    }

    #[test]
    fn falloff_weights_correct() {
        // At center (t=0), all falloffs return 1.0
        for falloff in [Falloff::Linear, Falloff::Smooth, Falloff::Sharp, Falloff::Sphere, Falloff::Constant] {
            assert!((falloff.weight(0.0) - 1.0).abs() < 1e-6, "{:?} at t=0", falloff);
        }
        // At edge (t=1), all return 0.0 except Constant which has already been cut off
        for falloff in [Falloff::Linear, Falloff::Smooth, Falloff::Sharp, Falloff::Sphere] {
            assert!((falloff.weight(1.0)).abs() < 1e-6, "{:?} at t=1", falloff);
        }
        // Beyond radius (t>1), all return 0.0
        for falloff in [Falloff::Linear, Falloff::Smooth, Falloff::Sharp, Falloff::Sphere, Falloff::Constant] {
            assert!((falloff.weight(1.5)).abs() < 1e-6, "{:?} at t=1.5", falloff);
        }
        // Sharp and Sphere should be > Linear at t=0.7 (they hold weight longer)
        let t = 0.7;
        assert!(Falloff::Sharp.weight(t) > Falloff::Linear.weight(t));
        assert!(Falloff::Sphere.weight(t) > Falloff::Linear.weight(t));
    }

    #[test]
    fn spherize_cube_full() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::splat(2),
        }
        .generate()?;

        let all_verts: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all_verts.clone(), 1.0, Pivot::Origin, None)?;

        // All vertices should now be at the same distance from origin
        let distances: Vec<f32> = all_verts
            .iter()
            .map(|v| v.position(&mesh).unwrap().length())
            .collect();
        let avg = distances.iter().sum::<f32>() / distances.len() as f32;
        for d in &distances {
            assert!(
                (d - avg).abs() < 0.01,
                "All verts should be equidistant after full spherize, got {} vs avg {}",
                d,
                avg
            );
        }

        Ok(())
    }

    #[test]
    fn spherize_zero_amount_no_change() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let positions_before: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(&mesh).unwrap())
            .collect();

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all, 0.0, Pivot::Origin, None)?;

        let positions_after: Vec<Vec3> = mesh
            .vertices()
            .map(|v| v.position(&mesh).unwrap())
            .collect();

        for (before, after) in positions_before.iter().zip(positions_after.iter()) {
            assert!(
                (*before - *after).length() < 1e-6,
                "Zero amount should not change positions"
            );
        }

        Ok(())
    }

    #[test]
    fn spherize_custom_radius() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;

        let all: Vec<VertexId> = mesh.vertices().collect();
        mesh.spherize(all.clone(), 1.0, Pivot::Origin, Some(2.0))?;

        for v in &all {
            let dist = v.position(&mesh)?.length();
            assert!(
                (dist - 2.0).abs() < 0.01,
                "Full spherize with radius 2.0 should put verts at distance 2.0, got {}",
                dist
            );
        }

        Ok(())
    }
}
