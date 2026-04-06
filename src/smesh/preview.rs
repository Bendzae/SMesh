use alloc::vec::Vec;

extern crate alloc;

use euc::{
    buffer::Buffer2d,
    rasterizer::{Lines, Triangles},
    Pipeline,
};
use glam::{Mat4, Vec3, Vec4};
use image::{ImageBuffer, Rgba, RgbaImage};

use std::collections::HashSet;

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
    /// Highlight specific faces in the render (drawn in a distinct color).
    pub highlight_faces: Option<HashSet<FaceId>>,
    /// Draw face normals as short lines from each face centroid.
    pub show_normals: bool,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            views: PreviewView::standard(),
            wireframe: false,
            highlight_faces: None,
            show_normals: false,
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

    pub fn with_highlight_faces(mut self, faces: HashSet<FaceId>) -> Self {
        self.highlight_faces = Some(faces);
        self
    }

    pub fn with_highlight_selection(mut self, selection: &MeshSelection, mesh: &SMesh) -> Self {
        if let Ok(faces) = selection.resolve_to_faces(mesh) {
            self.highlight_faces = Some(faces);
        }
        self
    }

    pub fn with_normals(mut self) -> Self {
        self.show_normals = true;
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

#[derive(Clone, Debug)]
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

/// Pipeline for highlighted faces — tinted to stand out from normal geometry.
struct HighlightPipeline {
    mvp: Mat4,
    light_dir: Vec3,
}

impl Pipeline for HighlightPipeline {
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
        let ambient = 0.25;
        let intensity = (ambient + ndotl * 0.7).min(1.0);

        // Cyan-ish highlight color
        let base_color = Vec3::new(0.2, 0.7, 0.9);
        let r = (base_color.x * intensity * 255.0) as u8;
        let g = (base_color.y * intensity * 255.0) as u8;
        let b = (base_color.z * intensity * 255.0) as u8;
        [r, g, b, 255]
    }
}

/// Pipeline for wireframe edges — emits a constant dark color with a clip-space offset
/// for thickening lines via multi-pass rendering.
struct WireframePipeline {
    mvp: Mat4,
    /// Clip-space offset (in pixels converted to NDC) for line thickening.
    offset: [f32; 2],
}

impl Pipeline for WireframePipeline {
    type Vertex = Vec3;
    type VsOut = ();
    type Pixel = [u8; 4];

    fn vert(&self, pos: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
        let clip = self.mvp * Vec4::new(pos.x, pos.y, pos.z, 1.0);
        (
            [
                clip.x + self.offset[0] * clip.w,
                clip.y + self.offset[1] * clip.w,
                clip.z - 0.005 * clip.w, // depth bias so wireframe draws over solid faces
                clip.w,
            ],
            (),
        )
    }

    fn frag(&self, _: &Self::VsOut) -> Self::Pixel {
        [20, 20, 20, 255]
    }
}

/// Pipeline for normal visualization lines — green color.
struct NormalsPipeline {
    mvp: Mat4,
}

impl Pipeline for NormalsPipeline {
    type Vertex = Vec3;
    type VsOut = ();
    type Pixel = [u8; 4];

    fn vert(&self, pos: &Self::Vertex) -> ([f32; 4], Self::VsOut) {
        let clip = self.mvp * Vec4::new(pos.x, pos.y, pos.z, 1.0);
        ([clip.x, clip.y, clip.z, clip.w], ())
    }

    fn frag(&self, _: &Self::VsOut) -> Self::Pixel {
        [50, 255, 80, 255]
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

/// Result of triangulating a mesh, with optional separation of highlighted faces.
struct TriangulatedMesh {
    /// Vertices for normal (non-highlighted) faces.
    vertices: Vec<MeshVertex>,
    /// Vertices for highlighted faces (empty if no highlight set).
    highlight_vertices: Vec<MeshVertex>,
}

fn triangulate_mesh(mesh: &SMesh, highlight_faces: Option<&HashSet<FaceId>>) -> TriangulatedMesh {
    let mut vertices = Vec::new();
    let mut highlight_vertices = Vec::new();

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

        let is_highlighted = highlight_faces.map_or(false, |hf| hf.contains(&face));
        let target = if is_highlighted {
            &mut highlight_vertices
        } else {
            &mut vertices
        };

        for i in 1..positions.len() - 1 {
            target.push(MeshVertex {
                position: positions[0],
                normal,
            });
            target.push(MeshVertex {
                position: positions[i],
                normal,
            });
            target.push(MeshVertex {
                position: positions[i + 1],
                normal,
            });
        }
    }

    TriangulatedMesh {
        vertices,
        highlight_vertices,
    }
}

/// Extract normal visualization lines: pairs of (centroid, centroid + normal * scale).
fn extract_normals(mesh: &SMesh, scale: f32) -> Vec<Vec3> {
    let mut line_verts = Vec::new();

    for face in mesh.faces() {
        let positions: Vec<Vec3> = face
            .vertices(mesh)
            .filter_map(|v| v.position(mesh).ok())
            .collect();
        if positions.len() < 3 {
            continue;
        }

        let centroid =
            positions.iter().copied().sum::<Vec3>() / positions.len() as f32;
        let e1 = positions[1] - positions[0];
        let e2 = positions[2] - positions[0];
        let normal = e1.cross(e2).normalize_or_zero();

        line_verts.push(centroid);
        line_verts.push(centroid + normal * scale);
    }

    line_verts
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

// --- Minimal bitmap font for view labels (5x7 glyphs) ---

/// Returns a 5x7 bitmap for an ASCII character (each row is a u8 bitmask, MSB-first).
/// Only uppercase letters, digits, and a few symbols are included.
fn glyph_bitmap(ch: char) -> Option<[u8; 7]> {
    Some(match ch.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => return None,
    })
}

/// Draw a text string onto an RgbaImage at (x, y) with the given color.
fn draw_text(img: &mut RgbaImage, x: u32, y: u32, text: &str, color: [u8; 4], scale: u32) {
    let mut cx = x;
    for ch in text.chars() {
        if let Some(bitmap) = glyph_bitmap(ch) {
            for (row, &bits) in bitmap.iter().enumerate() {
                for col in 0..5u32 {
                    if bits & (1 << (4 - col)) != 0 {
                        // Draw a scale x scale block for each pixel
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let px = cx + col * scale + sx;
                                let py = y + (row as u32) * scale + sy;
                                if px < img.width() && py < img.height() {
                                    img.put_pixel(px, py, Rgba(color));
                                }
                            }
                        }
                    }
                }
            }
        }
        cx += 6 * scale; // 5px glyph + 1px spacing, scaled
    }
}

/// Stitch a grid of sub-images into a single composite image.
fn compose_grid(images: &[(String, RgbaImage)], cols: u32, label_scale: u32) -> RgbaImage {
    if images.is_empty() {
        return ImageBuffer::new(1, 1);
    }

    let cell_w = images[0].1.width();
    let cell_h = images[0].1.height();
    let rows = ((images.len() as u32) + cols - 1) / cols;
    let total_w = cell_w * cols;
    let total_h = cell_h * rows;

    let mut composite = ImageBuffer::new(total_w, total_h);
    // Fill with background
    for pixel in composite.pixels_mut() {
        *pixel = Rgba([30, 30, 30, 255]);
    }

    for (i, (label, img)) in images.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let ox = col * cell_w;
        let oy = row * cell_h;

        // Copy sub-image
        for y in 0..img.height().min(cell_h) {
            for x in 0..img.width().min(cell_w) {
                composite.put_pixel(ox + x, oy + y, *img.get_pixel(x, y));
            }
        }

        // Draw label in top-left corner with shadow
        let pad = 4 * label_scale;
        let upper = label.to_uppercase();
        draw_text(&mut composite, ox + pad + 1, oy + pad + 1, &upper, [0, 0, 0, 255], label_scale);
        draw_text(&mut composite, ox + pad, oy + pad, &upper, [220, 220, 220, 255], label_scale);
    }

    composite
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
        let tri_mesh = triangulate_mesh(self, opts.highlight_faces.as_ref());
        let all_verts: Vec<MeshVertex> = tri_mesh
            .vertices
            .iter()
            .chain(tri_mesh.highlight_vertices.iter())
            .cloned()
            .collect();

        if all_verts.is_empty() {
            return opts
                .views
                .iter()
                .map(|v| MeshPreview {
                    name: v.label().to_string(),
                    image: ImageBuffer::new(opts.width, opts.height),
                })
                .collect();
        }

        let (bb_min, bb_max) = compute_bounding_box(&all_verts);
        let light_dir = Vec3::new(0.4, 0.7, 0.5).normalize();
        let edge_verts = if opts.wireframe {
            extract_edges(self)
        } else {
            Vec::new()
        };
        let normal_scale = (bb_max - bb_min).length() * 0.03;
        let normal_verts = if opts.show_normals {
            extract_normals(self, normal_scale)
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

                // Solid shading pass (non-highlighted faces)
                if !tri_mesh.vertices.is_empty() {
                    let mesh_pipeline = MeshPipeline { mvp, light_dir };
                    mesh_pipeline.draw::<Triangles<Buffer2d<f32>>, _>(
                        &tri_mesh.vertices,
                        &mut color_buf,
                        Some(&mut depth_buf),
                    );
                }

                // Highlighted faces pass
                if !tri_mesh.highlight_vertices.is_empty() {
                    let highlight_pipeline = HighlightPipeline { mvp, light_dir };
                    highlight_pipeline.draw::<Triangles<Buffer2d<f32>>, _>(
                        &tri_mesh.highlight_vertices,
                        &mut color_buf,
                        Some(&mut depth_buf),
                    );
                }

                // Wireframe overlay pass
                if opts.wireframe && !edge_verts.is_empty() {
                    let wire_pipeline = WireframePipeline { mvp, offset: [0.0, 0.0] };
                    wire_pipeline.draw::<Lines<Buffer2d<f32>>, _>(
                        &edge_verts,
                        &mut color_buf,
                        Some(&mut depth_buf),
                    );
                }

                // Normals overlay pass
                if opts.show_normals && !normal_verts.is_empty() {
                    let normals_pipeline = NormalsPipeline { mvp };
                    normals_pipeline.draw::<Lines<Buffer2d<f32>>, _>(
                        &normal_verts,
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

    /// Render a single composite image with all views in a 2x2 grid.
    ///
    /// Each cell is rendered at `(width/2, height/2)` and stitched together.
    /// View labels are drawn in each cell's top-left corner.
    pub fn render_composite(&self, opts: &PreviewOptions) -> RgbaImage {
        let cols = if opts.views.len() <= 1 { 1 } else { 2 };
        let cell_w = if cols == 1 { opts.width } else { opts.width / 2 };
        let cell_h = if cols == 1 { opts.height } else { opts.height / 2 };

        let cell_opts = PreviewOptions {
            width: cell_w,
            height: cell_h,
            views: opts.views.clone(),
            wireframe: opts.wireframe,
            highlight_faces: opts.highlight_faces.clone(),
            show_normals: opts.show_normals,
        };

        let previews = self.render(&cell_opts);
        let labeled: Vec<(String, RgbaImage)> = previews
            .into_iter()
            .map(|p| (p.name, p.image))
            .collect();

        let label_scale = (cell_w / 200).max(1);
        compose_grid(&labeled, cols as u32, label_scale)
    }

    /// Render a composite preview and save to a single file.
    pub fn save_composite_preview(
        &self,
        opts: &PreviewOptions,
        path: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let img = self.render_composite(opts);
        img.save(path)?;
        Ok(path.to_string())
    }

    /// Create a snapshot (clone) of the current mesh state.
    ///
    /// Use this to save intermediate states during multi-step sculpts so you can
    /// preview or restore them later without re-running the whole generation.
    pub fn save_checkpoint(&self) -> SMesh {
        self.clone()
    }

    /// Restore mesh state from a previously saved checkpoint.
    ///
    /// Replaces the current mesh contents with the checkpoint's state.
    pub fn restore_checkpoint(&mut self, checkpoint: &SMesh) {
        *self = checkpoint.clone();
    }

    /// Save a checkpoint and immediately render a composite preview of it.
    pub fn preview_checkpoint(
        &self,
        opts: &PreviewOptions,
        path: &str,
    ) -> Result<(SMesh, String), Box<dyn std::error::Error>> {
        let checkpoint = self.save_checkpoint();
        let img = checkpoint.render_composite(opts);
        img.save(path)?;
        Ok((checkpoint, path.to_string()))
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

    #[test]
    fn render_composite_2x2() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let opts = PreviewOptions::default().with_size(512, 512);
        let composite = mesh.render_composite(&opts);

        // 4 views in 2x2 grid → full size image
        assert_eq!(composite.width(), 512);
        assert_eq!(composite.height(), 512);

        // Each quadrant should have non-background pixels
        let bg = Rgba([30, 30, 30, 255]);
        let has_tl = (0..256).any(|y| (0..256).any(|x| *composite.get_pixel(x, y) != bg));
        let has_tr = (0..256).any(|y| (256..512).any(|x| *composite.get_pixel(x, y) != bg));
        let has_bl = (256..512).any(|y| (0..256).any(|x| *composite.get_pixel(x, y) != bg));
        assert!(has_tl, "Top-left quadrant (front) should have content");
        assert!(has_tr, "Top-right quadrant (right) should have content");
        assert!(has_bl, "Bottom-left quadrant (top) should have content");

        Ok(())
    }

    #[test]
    fn render_composite_single_view() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let opts = PreviewOptions::default()
            .with_size(256, 256)
            .with_views(vec![PreviewView::Front]);
        let composite = mesh.render_composite(&opts);

        // Single view → full resolution, no grid
        assert_eq!(composite.width(), 256);
        assert_eq!(composite.height(), 256);

        Ok(())
    }

    #[test]
    fn render_with_highlight() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        // Highlight ALL faces so at least some are visible from every angle
        let highlight: HashSet<FaceId> = mesh.faces().collect();

        let opts = PreviewOptions::default()
            .with_size(256, 256)
            .with_views(vec![PreviewView::Diagonal])
            .with_highlight_faces(highlight);

        let previews = mesh.render(&opts);
        assert_eq!(previews.len(), 1);

        // Should have cyan-ish highlight pixels (blue channel > red channel)
        let has_highlight = previews[0].image.pixels().any(|p| p.0[2] > p.0[0] + 30);
        assert!(has_highlight, "Should have highlighted (cyan) pixels");

        Ok(())
    }

    #[test]
    fn render_with_normals() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::ONE,
        }
        .generate()?;
        mesh.recalculate_normals()?;

        let opts = PreviewOptions::default()
            .with_size(256, 256)
            .with_views(vec![PreviewView::Diagonal])
            .with_normals();

        let previews = mesh.render(&opts);
        assert_eq!(previews.len(), 1);

        // Should have bright green normal-line pixels
        let has_green = previews[0]
            .image
            .pixels()
            .any(|p| p.0[1] > 200 && p.0[0] < 100 && p.0[2] < 120);
        assert!(has_green, "Should have green normal visualization lines");

        Ok(())
    }

    #[test]
    fn checkpoint_save_restore() -> SMeshResult<()> {
        let mesh = &mut SMesh::new();
        let v0 = mesh.add_vertex(vec3(-1.0, 0.0, -1.0));
        let v1 = mesh.add_vertex(vec3(-1.0, 0.0, 1.0));
        let v2 = mesh.add_vertex(vec3(1.0, 0.0, 1.0));
        let v3 = mesh.add_vertex(vec3(1.0, 0.0, -1.0));
        mesh.make_quad(v0, v1, v2, v3)?;

        let checkpoint = mesh.save_checkpoint();
        let original_vert_count = mesh.vertices().count();

        // Modify the mesh
        let v4 = mesh.add_vertex(vec3(0.0, 1.0, 0.0));
        let _ = v4; // just adding a vertex changes the count
        assert_eq!(mesh.vertices().count(), original_vert_count + 1);

        // Restore
        mesh.restore_checkpoint(&checkpoint);
        assert_eq!(mesh.vertices().count(), original_vert_count);

        Ok(())
    }

    #[test]
    #[ignore] // Run with: cargo test --features preview -p smesh preview::tests::save_demo_images -- --ignored --nocapture
    fn save_demo_images() -> SMeshResult<()> {
        use crate::smesh::primitives::{Cube, Primitive};

        let (mut mesh, _) = Cube {
            subdivision: glam::U16Vec3::new(2, 2, 2),
        }
        .generate()?;
        mesh.recalculate_normals()?;

        // Composite with wireframe
        let opts = PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe();
        let path = mesh.save_composite_preview(&opts, "/tmp/preview_composite.png").unwrap();
        eprintln!("Saved {path}");

        // Highlighted faces
        let faces: Vec<FaceId> = mesh.faces().collect();
        let highlight: HashSet<FaceId> = faces[..faces.len() / 3].iter().copied().collect();
        let opts = PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe()
            .with_highlight_faces(highlight);
        let path = mesh.save_composite_preview(&opts, "/tmp/preview_highlight.png").unwrap();
        eprintln!("Saved {path}");

        // Normals
        let opts = PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe()
            .with_normals();
        let path = mesh.save_composite_preview(&opts, "/tmp/preview_normals.png").unwrap();
        eprintln!("Saved {path}");

        Ok(())
    }

    #[test]
    fn bitmap_font_coverage() {
        // Verify all view labels can be rendered
        for label in ["FRONT", "BACK", "LEFT", "RIGHT", "TOP", "BOTTOM", "DIAGONAL"] {
            for ch in label.chars() {
                assert!(
                    glyph_bitmap(ch).is_some(),
                    "Missing glyph for '{}' in label '{}'",
                    ch,
                    label
                );
            }
        }
    }
}
