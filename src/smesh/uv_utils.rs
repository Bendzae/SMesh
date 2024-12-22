use glam::{DVec2, Mat4, Vec2};
use sprs::TriMat;

use crate::prelude::*;

impl SMesh {
    fn compute_arap_parameterization(&mut self) -> SMeshResult<()> {
        let num_vertices = self.vertices().len();

        // Initialize UV coordinates (2D parameterization)
        let mut uv_coords = vec![DVec2::ZERO; num_vertices];

        // Build the Laplacian matrix using a triplet builder
        let mut laplacian_builder = TriMat::<f64>::new((num_vertices, num_vertices));

        // Right-hand side vectors for x and y components
        let mut b_x = vec![0.0; num_vertices];
        let mut b_y = vec![0.0; num_vertices];

        // Construct cotangent weights and fill the Laplacian matrix
        for (i, vertex) in self.vertices().enumerate() {
            if let Ok(he_idx) = vertex.halfedge().run(self) {
                let mut current_he_idx = he_idx;
                let mut weight_sum = 0.0;

                loop {
                    let he = current_he_idx;
                    let next_he_idx = he.next().run(self)?;
                    let next_he = &mesh.half_edges[next_he_idx];

                    let vi = vertex.position;
                    let vj = mesh.vertices[he.vertex].position;
                    let vk = mesh.vertices[next_he.vertex].position;

                    // Compute cotangent weights
                    let weight = cotangent_weight(vi, vj, vk);

                    laplacian_builder.add_triplet(i, he.vertex, -weight);
                    weight_sum += weight;

                    // Move to the next half-edge around the vertex
                    if let Some(opposite_idx) = he.opposite {
                        current_he_idx = mesh.half_edges[opposite_idx].next.unwrap();
                        if current_he_idx == he_idx {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                laplacian_builder.add_triplet(i, i, weight_sum);
            }
        }

        // Convert the Laplacian builder into a compressed sparse matrix
        let laplacian = laplacian_builder.to_csr();

        // Apply boundary conditions (fix certain vertices)
        let fixed_vertices = vec![0, 1]; // Fixing the first two vertices
        let mut is_fixed = vec![false; num_vertices];
        for &i in &fixed_vertices {
            is_fixed[i] = true;
        }

        // Map from free vertex index to global vertex index
        let mut free_vertex_indices = Vec::new();
        let mut free_vertex_map = vec![0usize; num_vertices];
        let mut idx = 0;
        for i in 0..num_vertices {
            if !is_fixed[i] {
                free_vertex_map[i] = idx;
                free_vertex_indices.push(i);
                idx += 1;
            }
        }

        let num_free = free_vertex_indices.len();

        // Build the reduced Laplacian matrix and right-hand side vectors
        let mut laplacian_free_builder = TriMat::<f64>::new((num_free, num_free));
        let mut b_free_x = vec![0.0; num_free];
        let mut b_free_y = vec![0.0; num_free];

        for (row_idx, &i) in free_vertex_indices.iter().enumerate() {
            // Fill the reduced Laplacian matrix
            for (col_idx, &j) in free_vertex_indices.iter().enumerate() {
                let value = laplacian.get(i, j).unwrap_or(0.0);
                if value != 0.0 {
                    laplacian_free_builder.add_triplet(row_idx, col_idx, value);
                }
            }
            // Adjust the right-hand side for fixed vertices
            for &j in &fixed_vertices {
                let value = laplacian.get(i, j).unwrap_or(0.0);
                if value != 0.0 {
                    b_free_x[row_idx] -= value * uv_coords[j].x;
                    b_free_y[row_idx] -= value * uv_coords[j].y;
                }
            }
        }

        let laplacian_free = laplacian_free_builder.to_csr();

        // Solve the linear systems using the Conjugate Gradient method
        let x_solution = conjugate_gradient(&laplacian_free, &b_free_x, 1e-8, 1000);
        let y_solution = conjugate_gradient(&laplacian_free, &b_free_y, 1e-8, 1000);

        // Update UV coordinates for free vertices
        for (idx, &i) in free_vertex_indices.iter().enumerate() {
            uv_coords[i] = DVec2::new(x_solution[idx], y_solution[idx]);
        }

        // Assign fixed UV coordinates
        uv_coords[0] = DVec2::new(0.0, 0.0);
        uv_coords[1] = DVec2::new(1.0, 0.0);
        Ok(())
    }
}

fn cotangent_weight(vi: DVec3, vj: DVec3, vk: DVec3) -> f64 {
    let u = vj - vi;
    let v = vk - vi;

    let cross = u.cross(v).length();
    let dot = u.dot(v);

    // Avoid division by zero
    if cross.abs() < 1e-8 {
        0.0
    } else {
        dot / cross
    }
}

fn conjugate_gradient(a: &CsMat<f64>, b: &[f64], tol: f64, max_iter: usize) -> Vec<f64> {
    let n = b.len();
    let mut x = vec![0.0; n];
    let mut r = b.to_vec();
    let mut p = r.clone();
    let mut rsold = dot(&r, &r);

    for _ in 0..max_iter {
        let ap = mat_vec_mul(a, &p);
        let alpha = rsold / dot(&p, &ap);
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        let rsnew = dot(&r, &r);
        if rsnew.sqrt() < tol {
            break;
        }
        p = r
            .iter()
            .zip(&p)
            .map(|(&ri, &pi)| ri + (rsnew / rsold) * pi)
            .collect();
        rsold = rsnew;
    }
    x
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(ai, bi)| ai * bi).sum()
}

fn mat_vec_mul(a: &CsMat<f64>, x: &[f64]) -> Vec<f64> {
    let mut y = vec![0.0; x.len()];
    for (i, xi) in x.iter().enumerate() {
        for (j, &val) in a.outer_view(i).unwrap().iter() {
            y[i] += val * x[j];
        }
    }
    y
}
