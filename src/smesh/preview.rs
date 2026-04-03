use alloc::vec::Vec;

extern crate alloc;

use euc::{
    buffer::Buffer2d,
    rasterizer::{Lines, Triangles},
    Interpolate, Pipeline, Target,
};
use glam::{Mat4, Vec3, Vec4};
use image::{ImageBuffer, Rgba, RgbaImage};

use crate::prelude::*;

/// A named rendered view of a mesh.
pub struct MeshPreview {
    pub name: String,
    pub image: RgbaImage,
}

/// Options for mesh preview rendering.
#[derive(Debug, Clone)]
pub struct PreviewOptions {
    pub width: u32,
    pub height: u32,
    pub views: Vec<PreviewView>,
    pub wireframe: bool,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            views: PreviewView::standard(),
            wireframe: false,
        }
    }
}

impl PreviewOptions {
    pub fn with_wireframe(mut self) -> Self {
        self.wireframe = true;
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_views(mut self, views: Vec<PreviewView>) -> Self {
        self.views = views;
        self
    }
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

// --- Shader pipelines ---

#[derive(Clone)]
struct MeshVertex {
    position: Vec3,
    normal: Vec3,
}

/// Interpolated normal: (nx, ny, nz).
type VsOut = (f32, f32, f32);

struct MeshPipeline {
    mvp: Mat4,
    light_dir: Vec3,
}

impl Pipeline for MeshPipeline {
    type Vertex = MeshVertex;
    type VsOut = VsOut;
    type Pixel = [u8; 4];

    fn vert(&self, vertex: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
        let clip =
            self.mvp * Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
        let n = vertex.normal;
        ([clip.x, clip.y, clip.z, clip.w], (n.x, n.y, n.z))
    }

    fn frag(&self, vs_out: &Self::VsOut) -> Self::Pixel {
        let n = Vec3::new(vs_out.0, vs_out.1, vs_out.2).normalize_or_zero();
        let ndotl = n.dot(self.light_dir).abs();
        let fill_dir = Vec3::new(-0.3, 0.4, -0.6).normalize();
        let fill = n.dot(fill_dir).abs() * 0.3;
        let ambient = 0.2;
        let intensity = (ambient + ndotl * 0.6 + fill).min(1.0);

        let base_color = Vec3::new(0.75, 0.55, 0.35);
        let r = (base_color.x * intensity * 255.0) as u8;
        let g = (base_color.y * intensity * 255.0) as u8;
        let b = (base_color.z * intensity * 255.0) as u8;
        [r, g, b, 255]
    }
}

/// Pipeline for wireframe edges — just emits a constant dark color.
struct WireframePipeline {
    mvp: Mat4,
}

impl Pipeline for WireframePipeline {
    type Vertex = Vec3;
    type VsOut = ();
    type Pixel = [u8; 4];

    fn vert(&self, pos: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
        let clip = self.mvp * Vec4::new(pos.x, pos.y, pos.z, 1.0);
        ([clip.x, clip.y, clip.z, clip.w], ())
    }

    fn frag(&self, _: &Self::VsOut) -> Self::Pixel {
        [20, 20, 20, 255]
    }
}

// --- Geometry extraction ---

fn compute_mvp(bb_min: Vec3, bb_max: Vec3, view: &PreviewView) -> Mat4 {
    let center = (bb_min + bb_max) * 0.5;
    let extent = bb_max - bb_min;
    let radius = extent.length() * 0.5;
    let padded = radius * 1.15;

    let dir = view.direction();
    let eye = center - dir * radius * 3.0;
    let up = view.up();

    let view_mat = Mat4::look_at_rh(eye, center, up);
    let proj = Mat4::orthographic_rh(-padded, padded, -padded, padded, 0.01, radius * 8.0);
    proj * view_mat
}

fn triangulate_mesh(mesh: &SMesh) -> Vec<MeshVertex> {
    let mut vertices = Vec::new();

    for face in mesh.faces() {
        let face_verts: Vec<VertexId> = face.vertices(mesh).collect();
        if face_verts.len() < 3 {
            continue;
        }

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

/// Extract edge line segments (pairs of positions) from the mesh.
/// Each edge is emitted once as [src_pos, dst_pos].
fn extract_edges(mesh: &SMesh) -> Vec<Vec3> {
    let mut edge_verts = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for he in mesh.halfedges() {
        let Ok(opp) = he.opposite().run(mesh) else {
            continue;
        };
        let key = if he < opp { (he, opp) } else { (opp, he) };
        if !seen.insert(key) {
            continue;
        }

        let Ok(src) = he.src_vert().run(mesh) else {
            continue;
        };
        let Ok(dst) = he.dst_vert().run(mesh) else {
            continue;
        };
        let Ok(p0) = src.position(mesh) else { continue };
        let Ok(p1) = dst.position(mesh) else { continue };

        edge_verts.push(p0);
        edge_verts.push(p1);
    }

    edge_verts
}

fn compute_bounding_box(vertices: &[MeshVertex]) -> (Vec3, Vec3) {
    let mut bb_min = Vec3::splat(f32::INFINITY);
    let mut bb_max = Vec3::splat(f32::NEG_INFINITY);
    for v in vertices {
        bb_min = bb_min.min(v.position);
        bb_max = bb_max.max(v.position);
    }
    (bb_min, bb_max)
}

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

// --- Public API ---

impl SMesh {
    /// Render preview images with default options (512x512, standard views, no wireframe).
    pub fn render_previews(&self, width: u32, height: u32) -> Vec<MeshPreview> {
        self.render(
            &PreviewOptions::default()
                .with_size(width, height),
        )
    }

    /// Render the mesh from specific viewpoints.
    pub fn render_views(
        &self,
        width: u32,
        height: u32,
        views: &[PreviewView],
    ) -> Vec<MeshPreview> {
        self.render(
            &PreviewOptions::default()
                .with_size(width, height)
                .with_views(views.to_vec()),
        )
    }

    /// Render previews with full control over options.
    pub fn render(&self, opts: &PreviewOptions) -> Vec<MeshPreview> {
        let tri_verts = triangulate_mesh(self);
        if tri_verts.is_empty() {
            return opts
                .views
                .iter()
                .map(|v| MeshPreview {
                    name: v.label().to_string(),
                    image: ImageBuffer::new(opts.width, opts.height),
                })
                .collect();
        }

        let (bb_min, bb_max) = compute_bounding_box(&tri_verts);
        let light_dir = Vec3::new(0.4, 0.7, 0.5).normalize();
        let edge_verts = if opts.wireframe {
            extract_edges(self)
        } else {
            Vec::new()
        };
        let w = opts.width as usize;
        let h = opts.height as usize;

        opts.views
            .iter()
            .map(|view| {
                let mvp = compute_mvp(bb_min, bb_max, view);
                let bg = [30u8, 30, 30, 255];
                let mut color_buf = Buffer2d::new([w, h], bg);
                let mut depth_buf = Buffer2d::new([w, h], 1.0f32);

                // Solid shading pass
                let mesh_pipeline = MeshPipeline { mvp, light_dir };
                mesh_pipeline.draw::<Triangles<Buffer2d<f32>>, _>(
                    &tri_verts,
                    &mut color_buf,
                    Some(&mut depth_buf),
                );

                // Wireframe overlay pass
                if opts.wireframe && !edge_verts.is_empty() {
                    let wire_pipeline = WireframePipeline { mvp };
                    wire_pipeline.draw::<Lines<Buffer2d<f32>>, _>(
                        &edge_verts,
                        &mut color_buf,
                        Some(&mut depth_buf),
                    );
                }

                MeshPreview {
                    name: view.label().to_string(),
                    image: buffer_to_image(&color_buf, opts.width, opts.height),
                }
            })
            .collect()
    }

    /// Render previews and save them to files in the given directory.
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

    /// Render previews with options and save to files.
    pub fn save_preview_with_options(
        &self,
        opts: &PreviewOptions,
        dir: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let previews = self.render(opts);
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

        let top = &previews[2];
        assert_eq!(top.name, "top");
        let has_mesh_pixels = top.image.pixels().any(|p| p.0 != [30, 30, 30, 255]);
        assert!(has_mesh_pixels, "Top view should show the quad");

        Ok(())
    }

    #[test]
    fn render_empty_mesh() {
        let mesh = SMesh::new();
        let previews = mesh.render_previews(64, 64);
        assert_eq!(previews.len(), 4);
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
        for preview in &previews {
            let has_pixels = preview.image.pixels().any(|p| p.0 != [30, 30, 30, 255]);
            assert!(has_pixels, "{} view should show the cube", preview.name);
        }
        Ok(())
    }

    #[test]
    fn render_with_wireframe() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let opts = PreviewOptions::default()
            .with_size(256, 256)
            .with_views(vec![PreviewView::Diagonal])
            .with_wireframe();

        let previews = mesh.render(&opts);
        assert_eq!(previews.len(), 1);
        assert_eq!(previews[0].name, "diagonal");

        let has_pixels = previews[0].image.pixels().any(|p| p.0 != [30, 30, 30, 255]);
        assert!(has_pixels, "Wireframe diagonal should show the cube");

        // Wireframe should have dark edge pixels (near [20,20,20])
        let has_edge_pixels = previews[0].image.pixels().any(|p| {
            p.0[0] < 25 && p.0[1] < 25 && p.0[2] < 25 && p.0[3] == 255 && p.0 != [30, 30, 30, 255]
        });
        assert!(has_edge_pixels, "Wireframe should have dark edge lines");

        Ok(())
    }

    #[test]
    fn render_custom_views() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let previews = mesh.render_views(128, 128, &[PreviewView::Top, PreviewView::Back]);
        assert_eq!(previews.len(), 2);
        assert_eq!(previews[0].name, "top");
        assert_eq!(previews[1].name, "back");

        Ok(())
    }
}
