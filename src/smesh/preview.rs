use alloc::vec::Vec;

extern crate alloc;

use euc::{buffer::Buffer2d, rasterizer::Triangles, Interpolate, Pipeline, Target};
use glam::{Mat4, Vec3, Vec4};
use image::{ImageBuffer, Rgba, RgbaImage};
use itertools::Itertools;

use crate::prelude::*;

/// A named rendered view of a mesh.
pub struct MeshPreview {
    pub name: String,
    pub image: RgbaImage,
}

/// View direction for rendering.
#[derive(Debug, Clone, Copy)]
pub enum PreviewView {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Diagonal,
}

impl PreviewView {
    /// Standard set of views for mesh inspection.
    pub fn standard() -> Vec<PreviewView> {
        vec![
            PreviewView::Front,
            PreviewView::Right,
            PreviewView::Top,
            PreviewView::Diagonal,
        ]
    }

    fn direction(&self) -> Vec3 {
        match self {
            PreviewView::Front => Vec3::NEG_Z,
            PreviewView::Back => Vec3::Z,
            PreviewView::Left => Vec3::NEG_X,
            PreviewView::Right => Vec3::X,
            PreviewView::Top => Vec3::NEG_Y,
            PreviewView::Bottom => Vec3::Y,
            PreviewView::Diagonal => Vec3::new(1.0, -1.0, 1.0).normalize(),
        }
    }

    fn up(&self) -> Vec3 {
        match self {
            PreviewView::Top | PreviewView::Bottom => Vec3::Z,
            _ => Vec3::Y,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            PreviewView::Front => "front",
            PreviewView::Back => "back",
            PreviewView::Left => "left",
            PreviewView::Right => "right",
            PreviewView::Top => "top",
            PreviewView::Bottom => "bottom",
            PreviewView::Diagonal => "diagonal",
        }
    }
}

/// Vertex data fed into the rasterizer: position + normal.
#[derive(Clone)]
struct MeshVertex {
    position: Vec3,
    normal: Vec3,
}

/// Interpolated normal data: (nx, ny, nz).
/// Uses (f32, f32, f32) which has a built-in Interpolate impl via euc.
type VsOut = (f32, f32, f32);

/// The rendering pipeline for flat-shaded mesh previews.
struct MeshPipeline {
    mvp: Mat4,
    light_dir: Vec3,
}

impl Pipeline for MeshPipeline {
    type Vertex = MeshVertex;
    type VsOut = VsOut;
    type Pixel = [u8; 4];

    fn vert(&self, vertex: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
        let clip = self.mvp * Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
        let n = vertex.normal;
        (
            [clip.x, clip.y, clip.z, clip.w],
            (n.x, n.y, n.z),
        )
    }

    fn frag(&self, vs_out: &Self::VsOut) -> Self::Pixel {
        let n = Vec3::new(vs_out.0, vs_out.1, vs_out.2).normalize_or_zero();
        // Two-sided lighting: use abs(dot) so back-facing triangles
        // still get lit, then add a fill light from the opposite side
        let ndotl = n.dot(self.light_dir).abs();
        let fill_dir = Vec3::new(-0.3, 0.4, -0.6).normalize();
        let fill = n.dot(fill_dir).abs() * 0.3;
        let ambient = 0.2;
        let intensity = (ambient + ndotl * 0.6 + fill).min(1.0);

        let base_color = Vec3::new(0.75, 0.55, 0.35); // warm wood color
        let r = (base_color.x * intensity * 255.0) as u8;
        let g = (base_color.y * intensity * 255.0) as u8;
        let b = (base_color.z * intensity * 255.0) as u8;
        [r, g, b, 255]
    }
}

/// Compute the orthographic MVP matrix that frames the mesh from a given view.
fn compute_mvp(bb_min: Vec3, bb_max: Vec3, view: &PreviewView) -> Mat4 {
    let center = (bb_min + bb_max) * 0.5;
    let extent = bb_max - bb_min;
    let radius = extent.length() * 0.5;

    // Add some padding
    let padded = radius * 1.15;

    let dir = view.direction();
    let eye = center - dir * radius * 3.0;
    let up = view.up();

    let view_mat = Mat4::look_at_rh(eye, center, up);
    let proj = Mat4::orthographic_rh(-padded, padded, -padded, padded, 0.01, radius * 8.0);

    proj * view_mat
}

/// Triangulate the mesh and produce vertex data for the rasterizer.
fn triangulate_mesh(mesh: &SMesh) -> Vec<MeshVertex> {
    let mut vertices = Vec::new();

    for face in mesh.faces() {
        let face_verts: Vec<VertexId> = face.vertices(mesh).collect();
        if face_verts.len() < 3 {
            continue;
        }

        // Compute face normal from first 3 verts
        let positions: Vec<Vec3> = face_verts
            .iter()
            .filter_map(|v| v.position(mesh).ok())
            .collect();
        if positions.len() < 3 {
            continue;
        }

        let e1 = positions[1] - positions[0];
        let e2 = positions[2] - positions[0];
        let normal = e1.cross(e2).normalize_or_zero();

        // Fan triangulation from vertex 0
        for i in 1..positions.len() - 1 {
            vertices.push(MeshVertex {
                position: positions[0],
                normal,
            });
            vertices.push(MeshVertex {
                position: positions[i],
                normal,
            });
            vertices.push(MeshVertex {
                position: positions[i + 1],
                normal,
            });
        }
    }

    vertices
}

/// Convert the euc Buffer2d to an image::RgbaImage.
fn buffer_to_image(buf: &Buffer2d<[u8; 4]>, width: u32, height: u32) -> RgbaImage {
    let raw: &[[u8; 4]] = buf.as_ref();
    let mut img = ImageBuffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let pixel = raw[(y * width + x) as usize];
            img.put_pixel(x, y, Rgba(pixel));
        }
    }
    img
}

impl SMesh {
    /// Render preview images of the mesh from multiple viewpoints.
    ///
    /// Returns a Vec of named images. Each image is a flat-shaded orthographic
    /// render from a standard viewpoint (front, right, top, diagonal).
    ///
    /// ```ignore
    /// mesh.recalculate_normals()?;
    /// let previews = mesh.render_previews(512, 512);
    /// for preview in &previews {
    ///     preview.image.save(format!("{}.png", preview.name)).unwrap();
    /// }
    /// ```
    pub fn render_previews(&self, width: u32, height: u32) -> Vec<MeshPreview> {
        self.render_views(width, height, &PreviewView::standard())
    }

    /// Render the mesh from specific viewpoints.
    pub fn render_views(
        &self,
        width: u32,
        height: u32,
        views: &[PreviewView],
    ) -> Vec<MeshPreview> {
        let vertices = triangulate_mesh(self);
        if vertices.is_empty() {
            return views
                .iter()
                .map(|v| MeshPreview {
                    name: v.label().to_string(),
                    image: ImageBuffer::new(width, height),
                })
                .collect();
        }

        // Compute bounding box
        let mut bb_min = Vec3::splat(f32::INFINITY);
        let mut bb_max = Vec3::splat(f32::NEG_INFINITY);
        for v in &vertices {
            bb_min = bb_min.min(v.position);
            bb_max = bb_max.max(v.position);
        }

        let light_dir = Vec3::new(0.4, 0.7, 0.5).normalize();

        views
            .iter()
            .map(|view| {
                let mvp = compute_mvp(bb_min, bb_max, view);

                let pipeline = MeshPipeline { mvp, light_dir };

                let bg = [30u8, 30, 30, 255]; // dark background
                let mut color_buf = Buffer2d::new([width as usize, height as usize], bg);
                let mut depth_buf = Buffer2d::new([width as usize, height as usize], 1.0f32);

                pipeline.draw::<Triangles<Buffer2d<f32>>, _>(
                    &vertices,
                    &mut color_buf,
                    Some(&mut depth_buf),
                );

                MeshPreview {
                    name: view.label().to_string(),
                    image: buffer_to_image(&color_buf, width, height),
                }
            })
            .collect()
    }

    /// Render previews and save them to files in the given directory.
    /// Returns the file paths of the saved images.
    pub fn save_previews(
        &self,
        width: u32,
        height: u32,
        dir: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let previews = self.render_previews(width, height);
        let mut paths = Vec::new();
        for preview in &previews {
            let path = format!("{}/{}.png", dir, preview.name);
            preview.image.save(&path)?;
            paths.push(path);
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use glam::vec3;

    use super::*;

    #[test]
    fn render_single_quad() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        mesh.make_quad(v0, v1, v2, v3)?;
        mesh.recalculate_normals()?;

        let previews = mesh.render_previews(128, 128);
        assert_eq!(previews.len(), 4);
        assert_eq!(previews[0].name, "front");
        assert_eq!(previews[0].image.width(), 128);

        // The top view should have non-background pixels (the quad is horizontal)
        let top = &previews[2];
        assert_eq!(top.name, "top");
        let has_mesh_pixels = top
            .image
            .pixels()
            .any(|p| p.0 != [30, 30, 30, 255]);
        assert!(has_mesh_pixels, "Top view should show the quad");

        Ok(())
    }

    #[test]
    fn render_empty_mesh() {
        let mesh = SMesh::new();
        let previews = mesh.render_previews(64, 64);
        assert_eq!(previews.len(), 4);
        // Should not panic, just produce empty images
    }

    #[test]
    fn render_cube() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let previews = mesh.render_previews(256, 256);
        // All 4 views should contain visible mesh pixels
        for preview in &previews {
            let has_pixels = preview
                .image
                .pixels()
                .any(|p| p.0 != [30, 30, 30, 255]);
            assert!(
                has_pixels,
                "{} view should show the cube",
                preview.name
            );
        }
        Ok(())
    }
}
