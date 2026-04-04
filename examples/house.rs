use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_inspector_egui::{
    inspector_options::ReflectInspectorOptions, quick::ResourceInspectorPlugin, InspectorOptions,
};
use glam::vec3;

use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use smesh::{
    adapters::bevy::{DebugRenderSMesh, Selection, ShowcasePlugin},
    prelude::*,
};
use primitives::Primitive;
use transform::Pivot;

#[derive(Reflect, Resource, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
struct HouseParameters {
    #[inspector(min = 5.0, max = 10.0)]
    pub main_width: f32,
    #[inspector(min = 4.0, max = 8.0)]
    pub main_depth: f32,
    #[inspector(min = 2.5, max = 4.0)]
    pub story_height: f32,
    #[inspector(min = 2.0, max = 5.0)]
    pub wing_width: f32,
    #[inspector(min = 3.0, max = 6.0)]
    pub wing_depth: f32,
    #[inspector(min = 0.1, max = 0.4)]
    pub wall_thickness: f32,
    pub show_chimney: bool,
    pub show_porch: bool,
    pub show_balcony: bool,
    #[inspector(min = 2, max = 5)]
    pub windows_per_floor: usize,
    #[inspector(min = 15.0, max = 45.0)]
    pub roof_angle: f32,
}

impl Default for HouseParameters {
    fn default() -> Self {
        Self {
            main_width: 7.0,
            main_depth: 6.0,
            story_height: 3.0,
            wing_width: 3.5,
            wing_depth: 5.0,
            wall_thickness: 0.25,
            show_chimney: true,
            show_porch: true,
            show_balcony: true,
            windows_per_floor: 3,
            roof_angle: 30.0,
        }
    }
}

#[derive(Component)]
struct HouseTag;

// Fixed proportional constants
const ROOF_THICKNESS: f32 = 0.15;
const ROOF_OVERHANG: f32 = 0.4;
const WINDOW_WIDTH: f32 = 0.9;
const WINDOW_HEIGHT: f32 = 1.3;
const WINDOW_FRAME_DEPTH: f32 = 0.08;
const WINDOW_SILL_DEPTH: f32 = 0.15;
const WINDOW_SILL_HEIGHT: f32 = 0.06;
const WINDOW_HEADER_HEIGHT: f32 = 0.1;
const WINDOW_HEADER_DEPTH: f32 = 0.1;
const DOOR_WIDTH: f32 = 1.1;
const DOOR_HEIGHT: f32 = 2.3;
const DOOR_FRAME_WIDTH: f32 = 0.12;
const DOOR_FRAME_DEPTH: f32 = 0.08;
const PORCH_OVERHANG_DEPTH: f32 = 1.5;
const PORCH_OVERHANG_HEIGHT: f32 = 0.12;
const PORCH_COLUMN_RADIUS: f32 = 0.1;
const PORCH_COLUMN_SEGMENTS: usize = 8;
const CHIMNEY_WIDTH: f32 = 0.8;
const CHIMNEY_DEPTH: f32 = 0.6;
const CHIMNEY_HEIGHT: f32 = 1.8;
const CHIMNEY_CAP_OVERHANG: f32 = 0.08;
const CHIMNEY_CAP_HEIGHT: f32 = 0.1;
const BALCONY_DEPTH: f32 = 1.2;
const BALCONY_SLAB_HEIGHT: f32 = 0.15;
const BALCONY_RAIL_HEIGHT: f32 = 1.0;
const BALCONY_RAIL_THICKNESS: f32 = 0.06;
const BALCONY_POST_SIZE: f32 = 0.06;
const FOUNDATION_HEIGHT: f32 = 0.3;
const FOUNDATION_OVERHANG: f32 = 0.05;
const CORNICE_HEIGHT: f32 = 0.15;
const CORNICE_OVERHANG: f32 = 0.1;
const WINDOW_DIVIDER_SIZE: f32 = 0.03;

fn make_box(width: f32, height: f32, depth: f32, position: Vec3) -> SMeshResult<SMesh> {
    let (mut part, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = part.select_all();
    part.scale(all.clone(), vec3(width, height, depth), Pivot::Origin)?;
    part.translate(all, position)?;
    Ok(part)
}

fn make_cylinder(radius: f32, height: f32, segments: usize, position: Vec3) -> SMeshResult<SMesh> {
    let (mut cyl, _) = primitives::Cylinder { segments, height, radius }.generate()?;
    let all = cyl.select_all();
    cyl.translate(all, position)?;
    Ok(cyl)
}

fn make_window(position: Vec3, facing: Vec3) -> SMeshResult<SMesh> {
    let mut win = SMesh::new();

    // Determine rotation based on facing direction
    let rot = if facing.z.abs() > 0.5 {
        let depth_dir = facing.z.signum();
        Quat::from_rotation_y(if depth_dir > 0.0 { 0.0 } else { PI })
    } else {
        let depth_dir = facing.x.signum();
        Quat::from_rotation_y(if depth_dir > 0.0 { PI / 2.0 } else { -PI / 2.0 })
    };

    // Frame (outer border, slightly proud of wall)
    let frame_outer_w = WINDOW_WIDTH + DOOR_FRAME_WIDTH * 2.0;
    let frame_outer_h = WINDOW_HEIGHT + DOOR_FRAME_WIDTH + WINDOW_HEADER_HEIGHT;
    let (mut frame, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = frame.select_all();
    frame.scale(all.clone(), vec3(frame_outer_w, frame_outer_h, WINDOW_FRAME_DEPTH), Pivot::Origin)?;
    frame.rotate(all.clone(), rot, Pivot::Origin)?;
    frame.translate(all, position + vec3(0.0, WINDOW_HEADER_HEIGHT * 0.3, 0.0))?;
    win.combine_with(frame)?;

    // Recessed glass pane (slightly behind the frame)
    let (mut pane, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = pane.select_all();
    pane.scale(all.clone(), vec3(WINDOW_WIDTH, WINDOW_HEIGHT, 0.02), Pivot::Origin)?;
    pane.rotate(all.clone(), rot, Pivot::Origin)?;
    pane.translate(all, position)?;
    win.combine_with(pane)?;

    // Window sill
    let (mut sill, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = sill.select_all();
    sill.scale(all.clone(), vec3(frame_outer_w + 0.06, WINDOW_SILL_HEIGHT, WINDOW_SILL_DEPTH), Pivot::Origin)?;
    sill.rotate(all.clone(), rot, Pivot::Origin)?;
    sill.translate(all, position + vec3(0.0, -WINDOW_HEIGHT / 2.0 - WINDOW_SILL_HEIGHT / 2.0, 0.0))?;
    win.combine_with(sill)?;

    // Window header / lintel
    let (mut header, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = header.select_all();
    header.scale(all.clone(), vec3(frame_outer_w + 0.04, WINDOW_HEADER_HEIGHT, WINDOW_HEADER_DEPTH), Pivot::Origin)?;
    header.rotate(all.clone(), rot, Pivot::Origin)?;
    header.translate(all, position + vec3(0.0, WINDOW_HEIGHT / 2.0 + WINDOW_HEADER_HEIGHT / 2.0 + DOOR_FRAME_WIDTH * 0.5, 0.0))?;
    win.combine_with(header)?;

    // Cross dividers (mullions)
    // Vertical divider
    let (mut vdiv, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = vdiv.select_all();
    vdiv.scale(all.clone(), vec3(WINDOW_DIVIDER_SIZE, WINDOW_HEIGHT, WINDOW_FRAME_DEPTH * 0.8), Pivot::Origin)?;
    vdiv.rotate(all.clone(), rot, Pivot::Origin)?;
    vdiv.translate(all, position)?;
    win.combine_with(vdiv)?;

    // Horizontal divider
    let (mut hdiv, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = hdiv.select_all();
    hdiv.scale(all.clone(), vec3(WINDOW_WIDTH, WINDOW_DIVIDER_SIZE, WINDOW_FRAME_DEPTH * 0.8), Pivot::Origin)?;
    hdiv.rotate(all.clone(), rot, Pivot::Origin)?;
    hdiv.translate(all, position + vec3(0.0, 0.1, 0.0))?;
    win.combine_with(hdiv)?;

    Ok(win)
}

fn make_door(position: Vec3, facing_z: f32) -> SMeshResult<SMesh> {
    let mut door = SMesh::new();
    let z_sign = facing_z.signum();

    // Door frame
    // Left jamb
    door.combine_with(make_box(
        DOOR_FRAME_WIDTH, DOOR_HEIGHT, DOOR_FRAME_DEPTH,
        position + vec3(-DOOR_WIDTH / 2.0 - DOOR_FRAME_WIDTH / 2.0, 0.0, 0.0),
    )?)?;
    // Right jamb
    door.combine_with(make_box(
        DOOR_FRAME_WIDTH, DOOR_HEIGHT, DOOR_FRAME_DEPTH,
        position + vec3(DOOR_WIDTH / 2.0 + DOOR_FRAME_WIDTH / 2.0, 0.0, 0.0),
    )?)?;
    // Header
    door.combine_with(make_box(
        DOOR_WIDTH + DOOR_FRAME_WIDTH * 2.0, DOOR_FRAME_WIDTH * 1.5, DOOR_FRAME_DEPTH,
        position + vec3(0.0, DOOR_HEIGHT / 2.0 + DOOR_FRAME_WIDTH * 0.75, 0.0),
    )?)?;

    // Door panel (slightly recessed)
    door.combine_with(make_box(
        DOOR_WIDTH, DOOR_HEIGHT, 0.05,
        position + vec3(0.0, 0.0, -z_sign * 0.02),
    )?)?;

    // Door panels (decorative raised panels on door)
    let panel_w = DOOR_WIDTH * 0.35;
    let panel_h = DOOR_HEIGHT * 0.35;
    for &px in &[-DOOR_WIDTH * 0.22, DOOR_WIDTH * 0.22] {
        for &py in &[-DOOR_HEIGHT * 0.18, DOOR_HEIGHT * 0.22] {
            door.combine_with(make_box(
                panel_w, panel_h, 0.03,
                position + vec3(px, py, z_sign * 0.02),
            )?)?;
        }
    }

    // Transom window above door
    let transom_y = position.y + DOOR_HEIGHT / 2.0 + DOOR_FRAME_WIDTH * 1.5 + 0.2;
    door.combine_with(make_box(
        DOOR_WIDTH + DOOR_FRAME_WIDTH, 0.35, DOOR_FRAME_DEPTH,
        vec3(position.x, transom_y, position.z),
    )?)?;

    Ok(door)
}

/// Create a roof slope panel.
/// `half_span` is the horizontal distance from wall edge to ridge (= building_width / 2).
/// `length` is the roof length along the ridge.
/// `angle` is the roof pitch in degrees.
/// `wall_edge_pos` is the point where the wall meets the roof (top of wall at the eave side).
/// `flip` = false: slope rises from left to right (left eave).
/// `flip` = true: slope rises from right to left (right eave).
fn make_roof_slope(half_span: f32, length: f32, angle: f32, wall_edge_pos: Vec3, flip: bool) -> SMeshResult<SMesh> {
    let (mut roof, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
    let all = roof.select_all();
    let theta = angle.to_radians();

    // Slope from wall edge to ridge (center), plus overhang past the wall
    let slope_to_ridge = half_span / theta.cos();
    let slope_total = slope_to_ridge + ROOF_OVERHANG;

    roof.scale(all.clone(), vec3(slope_total, ROOF_THICKNESS, length + ROOF_OVERHANG * 2.0), Pivot::Origin)?;

    // Shift so the wall-edge point (where overhang ends and main roof begins) is at origin.
    // The eave tip is at -ROOF_OVERHANG from origin along x (for left slope).
    // The ridge is at +slope_to_ridge from origin along x (for left slope).
    // Box center is at 0, box spans [-slope_total/2, +slope_total/2].
    // Wall edge should be at: -slope_total/2 + ROOF_OVERHANG = -(slope_to_ridge/2 - ROOF_OVERHANG/2)...
    // Simpler: shift box so wall-edge point lands at origin.
    // Wall edge is ROOF_OVERHANG from the eave end (-x end): local x = -slope_total/2 + ROOF_OVERHANG
    // To put that at origin: shift by slope_total/2 - ROOF_OVERHANG
    let wall_edge_shift = if flip {
        -(slope_total / 2.0 - ROOF_OVERHANG)
    } else {
        slope_total / 2.0 - ROOF_OVERHANG
    };
    roof.translate(all.clone(), vec3(wall_edge_shift, 0.0, 0.0))?;

    // Now wall-edge point is at origin. Rotate around origin.
    let rot_angle = if flip { -theta } else { theta };
    roof.rotate(all.clone(), Quat::from_rotation_z(rot_angle), Pivot::Origin)?;

    // Translate so origin (wall-edge point) goes to wall_edge_pos
    roof.translate(all, wall_edge_pos)?;
    Ok(roof)
}

fn generate_house(params: &HouseParameters) -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();

    let total_height = params.story_height * 2.0;
    let roof_rise = (params.main_width / 2.0) * params.roof_angle.to_radians().tan();
    let wing_height = total_height - 0.5;
    let wing_offset_x = params.main_width / 2.0 + params.wing_width / 2.0;

    // === Foundation ===
    mesh.combine_with(make_box(
        params.main_width + FOUNDATION_OVERHANG * 2.0,
        FOUNDATION_HEIGHT,
        params.main_depth + FOUNDATION_OVERHANG * 2.0,
        vec3(0.0, FOUNDATION_HEIGHT / 2.0, 0.0),
    )?)?;
    mesh.combine_with(make_box(
        params.wing_width + FOUNDATION_OVERHANG * 2.0,
        FOUNDATION_HEIGHT,
        params.wing_depth + FOUNDATION_OVERHANG * 2.0,
        vec3(wing_offset_x, FOUNDATION_HEIGHT / 2.0, -(params.main_depth - params.wing_depth) / 2.0),
    )?)?;

    // === Main body ===
    // Walls stop at eave height — roof slopes and gable fills handle everything above
    let eave_height = total_height;
    mesh.combine_with(make_box(
        params.main_width, eave_height, params.main_depth,
        vec3(0.0, eave_height / 2.0 + FOUNDATION_HEIGHT, 0.0),
    )?)?;

    // === Side wing ===
    let wing_z = -(params.main_depth - params.wing_depth) / 2.0;
    mesh.combine_with(make_box(
        params.wing_width, wing_height, params.wing_depth,
        vec3(wing_offset_x, wing_height / 2.0 + FOUNDATION_HEIGHT, wing_z),
    )?)?;

    // === Cornice / trim at eave line (sides only, not gable ends) ===
    // Front and back cornices (run along the depth/sides under the roof slope)
    let cornice_y = eave_height + FOUNDATION_HEIGHT + CORNICE_HEIGHT / 2.0;
    // Left side cornice
    mesh.combine_with(make_box(
        CORNICE_OVERHANG * 2.0 + params.wall_thickness,
        CORNICE_HEIGHT,
        params.main_depth + CORNICE_OVERHANG * 2.0,
        vec3(-params.main_width / 2.0, cornice_y, 0.0),
    )?)?;
    // Right side cornice
    mesh.combine_with(make_box(
        CORNICE_OVERHANG * 2.0 + params.wall_thickness,
        CORNICE_HEIGHT,
        params.main_depth + CORNICE_OVERHANG * 2.0,
        vec3(params.main_width / 2.0, cornice_y, 0.0),
    )?)?;
    let wing_cornice_y = wing_height + FOUNDATION_HEIGHT + CORNICE_HEIGHT / 2.0;
    mesh.combine_with(make_box(
        CORNICE_OVERHANG * 2.0 + params.wall_thickness,
        CORNICE_HEIGHT,
        params.wing_depth + CORNICE_OVERHANG * 2.0,
        vec3(wing_offset_x - params.wing_width / 2.0, wing_cornice_y, wing_z),
    )?)?;
    mesh.combine_with(make_box(
        CORNICE_OVERHANG * 2.0 + params.wall_thickness,
        CORNICE_HEIGHT,
        params.wing_depth + CORNICE_OVERHANG * 2.0,
        vec3(wing_offset_x + params.wing_width / 2.0, wing_cornice_y, wing_z),
    )?)?;

    // === Floor line trim (between stories) ===
    let floor_line_y = params.story_height + FOUNDATION_HEIGHT;
    let trim_h = 0.08;
    let trim_overhang = 0.04;
    mesh.combine_with(make_box(
        params.main_width + trim_overhang * 2.0, trim_h, params.main_depth + trim_overhang * 2.0,
        vec3(0.0, floor_line_y, 0.0),
    )?)?;
    mesh.combine_with(make_box(
        params.wing_width + trim_overhang * 2.0, trim_h, params.wing_depth + trim_overhang * 2.0,
        vec3(wing_offset_x, floor_line_y, wing_z),
    )?)?;

    // === Roof - Main body (gable) ===
    let roof_base_y = cornice_y + CORNICE_HEIGHT / 2.0;
    // Left slope: wall edge at left side, rises toward center ridge
    mesh.combine_with(make_roof_slope(
        params.main_width / 2.0, params.main_depth, params.roof_angle,
        vec3(-params.main_width / 2.0, roof_base_y, 0.0),
        false,
    )?)?;
    // Right slope: wall edge at right side, rises toward center ridge
    mesh.combine_with(make_roof_slope(
        params.main_width / 2.0, params.main_depth, params.roof_angle,
        vec3(params.main_width / 2.0, roof_base_y, 0.0),
        true,
    )?)?;

    // Gable end fills — stepped boxes approximating a triangle
    // Start at wall top, not cornice top, to avoid a gap
    let gable_start_y = eave_height + FOUNDATION_HEIGHT;
    let gable_total_h = roof_base_y + roof_rise - gable_start_y;
    let n_gable_layers = 6;
    let layer_h = gable_total_h / n_gable_layers as f32;
    let tan_angle = params.roof_angle.to_radians().tan();
    for i in 0..n_gable_layers {
        let layer_bottom = gable_start_y + layer_h * i as f32;
        let layer_top = layer_bottom + layer_h;
        let layer_mid_y = (layer_bottom + layer_top) / 2.0;
        // Width narrows linearly based on distance from roof_base_y (where slope starts)
        let height_into_slope = (layer_top - roof_base_y).max(0.0);
        let width_at_top = params.main_width - 2.0 * height_into_slope / tan_angle;
        if width_at_top <= 0.0 {
            break;
        }
        for &z in &[params.main_depth / 2.0, -params.main_depth / 2.0] {
            mesh.combine_with(make_box(
                width_at_top, layer_h, params.wall_thickness,
                vec3(0.0, layer_mid_y, z),
            )?)?;
        }
    }

    // === Roof - Wing (lower with slightly less steep slope) ===
    let wing_angle = params.roof_angle * 0.7;
    let wing_roof_y = wing_cornice_y + CORNICE_HEIGHT / 2.0;
    let wing_roof_rise = (params.wing_width / 2.0) * wing_angle.to_radians().tan();
    mesh.combine_with(make_roof_slope(
        params.wing_width / 2.0, params.wing_depth, wing_angle,
        vec3(wing_offset_x - params.wing_width / 2.0, wing_roof_y, wing_z),
        false,
    )?)?;
    mesh.combine_with(make_roof_slope(
        params.wing_width / 2.0, params.wing_depth, wing_angle,
        vec3(wing_offset_x + params.wing_width / 2.0, wing_roof_y, wing_z),
        true,
    )?)?;

    // Wing gable fills
    // Wing gable fills — start at wing wall top
    let wing_gable_start_y = wing_height + FOUNDATION_HEIGHT;
    let wing_gable_total_h = wing_roof_y + wing_roof_rise - wing_gable_start_y;
    let wing_tan = wing_angle.to_radians().tan();
    let wing_layer_h = wing_gable_total_h / n_gable_layers as f32;
    for i in 0..n_gable_layers {
        let layer_bottom = wing_gable_start_y + wing_layer_h * i as f32;
        let layer_top = layer_bottom + wing_layer_h;
        let layer_mid_y = (layer_bottom + layer_top) / 2.0;
        let height_into_slope = (layer_top - wing_roof_y).max(0.0);
        let width_at_top = params.wing_width - 2.0 * height_into_slope / wing_tan;
        if width_at_top <= 0.0 {
            break;
        }
        let wing_front_z = wing_z + params.wing_depth / 2.0;
        let wing_back_z = wing_z - params.wing_depth / 2.0;
        for &z in &[wing_front_z, wing_back_z] {
            mesh.combine_with(make_box(
                width_at_top, wing_layer_h, params.wall_thickness,
                vec3(wing_offset_x, layer_mid_y, z),
            )?)?;
        }
    }

    // === Windows - Front face (main body, +Z side) ===
    let front_z = params.main_depth / 2.0 + WINDOW_FRAME_DEPTH / 2.0;
    let n_win = params.windows_per_floor;
    let win_spacing = params.main_width / (n_win as f32 + 1.0);

    for floor in 0..2 {
        let win_y = FOUNDATION_HEIGHT + params.story_height * floor as f32 + params.story_height * 0.55;
        for i in 0..n_win {
            let win_x = -params.main_width / 2.0 + win_spacing * (i as f32 + 1.0);
            mesh.combine_with(make_window(
                vec3(win_x, win_y, front_z),
                Vec3::Z,
            )?)?;
        }
    }

    // === Windows - Back face (main body, -Z side) ===
    let back_z = -params.main_depth / 2.0 - WINDOW_FRAME_DEPTH / 2.0;
    for floor in 0..2 {
        let win_y = FOUNDATION_HEIGHT + params.story_height * floor as f32 + params.story_height * 0.55;
        for i in 0..n_win {
            let win_x = -params.main_width / 2.0 + win_spacing * (i as f32 + 1.0);
            mesh.combine_with(make_window(
                vec3(win_x, win_y, back_z),
                Vec3::NEG_Z,
            )?)?;
        }
    }

    // === Windows - Left side (main body, -X side) ===
    let left_x = -params.main_width / 2.0 - WINDOW_FRAME_DEPTH / 2.0;
    let side_spacing = params.main_depth / 3.0;
    for floor in 0..2 {
        let win_y = FOUNDATION_HEIGHT + params.story_height * floor as f32 + params.story_height * 0.55;
        for i in 0..2 {
            let win_z = -params.main_depth / 2.0 + side_spacing * (i as f32 + 1.0);
            mesh.combine_with(make_window(
                vec3(left_x, win_y, win_z),
                Vec3::NEG_X,
            )?)?;
        }
    }

    // === Windows - Wing front (+Z side) ===
    let wing_front_z = wing_z + params.wing_depth / 2.0 + WINDOW_FRAME_DEPTH / 2.0;
    for floor in 0..2 {
        let win_y = FOUNDATION_HEIGHT + params.story_height * floor as f32 + params.story_height * 0.55;
        mesh.combine_with(make_window(
            vec3(wing_offset_x, win_y, wing_front_z),
            Vec3::Z,
        )?)?;
    }

    // === Windows - Wing right side (+X side) ===
    let wing_right_x = wing_offset_x + params.wing_width / 2.0 + WINDOW_FRAME_DEPTH / 2.0;
    let wing_side_spacing = params.wing_depth / 3.0;
    for floor in 0..2 {
        let win_y = FOUNDATION_HEIGHT + params.story_height * floor as f32 + params.story_height * 0.55;
        for i in 0..2 {
            let win_z = wing_z - params.wing_depth / 2.0 + wing_side_spacing * (i as f32 + 1.0);
            mesh.combine_with(make_window(
                vec3(wing_right_x, win_y, win_z),
                Vec3::X,
            )?)?;
        }
    }

    // === Front door ===
    let door_y = FOUNDATION_HEIGHT + DOOR_HEIGHT / 2.0;
    let door_z = params.main_depth / 2.0 + DOOR_FRAME_DEPTH / 2.0;
    mesh.combine_with(make_door(
        vec3(0.0, door_y, door_z),
        1.0,
    )?)?;

    // Step
    mesh.combine_with(make_box(
        DOOR_WIDTH + DOOR_FRAME_WIDTH * 4.0, 0.15, 0.4,
        vec3(0.0, FOUNDATION_HEIGHT / 2.0, params.main_depth / 2.0 + 0.2),
    )?)?;

    // === Porch ===
    if params.show_porch {
        let porch_w = DOOR_WIDTH + 2.0;
        let porch_y = FOUNDATION_HEIGHT + DOOR_HEIGHT + DOOR_FRAME_WIDTH * 2.0 + 0.1;
        let porch_z = params.main_depth / 2.0 + PORCH_OVERHANG_DEPTH / 2.0;

        // Overhang
        mesh.combine_with(make_box(
            porch_w, PORCH_OVERHANG_HEIGHT, PORCH_OVERHANG_DEPTH,
            vec3(0.0, porch_y, porch_z),
        )?)?;

        // Columns
        let col_z = params.main_depth / 2.0 + PORCH_OVERHANG_DEPTH - 0.15;
        let col_height = porch_y - PORCH_OVERHANG_HEIGHT / 2.0;
        let col_y = col_height / 2.0;
        mesh.combine_with(make_cylinder(
            PORCH_COLUMN_RADIUS, col_height, PORCH_COLUMN_SEGMENTS,
            vec3(-porch_w / 2.0 + 0.15, col_y, col_z),
        )?)?;
        mesh.combine_with(make_cylinder(
            PORCH_COLUMN_RADIUS, col_height, PORCH_COLUMN_SEGMENTS,
            vec3(porch_w / 2.0 - 0.15, col_y, col_z),
        )?)?;

        // Column bases
        mesh.combine_with(make_box(
            PORCH_COLUMN_RADIUS * 3.0, 0.08, PORCH_COLUMN_RADIUS * 3.0,
            vec3(-porch_w / 2.0 + 0.15, 0.04, col_z),
        )?)?;
        mesh.combine_with(make_box(
            PORCH_COLUMN_RADIUS * 3.0, 0.08, PORCH_COLUMN_RADIUS * 3.0,
            vec3(porch_w / 2.0 - 0.15, 0.04, col_z),
        )?)?;

        // Column capitals
        mesh.combine_with(make_box(
            PORCH_COLUMN_RADIUS * 3.5, 0.06, PORCH_COLUMN_RADIUS * 3.5,
            vec3(-porch_w / 2.0 + 0.15, porch_y - PORCH_OVERHANG_HEIGHT / 2.0 - 0.03, col_z),
        )?)?;
        mesh.combine_with(make_box(
            PORCH_COLUMN_RADIUS * 3.5, 0.06, PORCH_COLUMN_RADIUS * 3.5,
            vec3(porch_w / 2.0 - 0.15, porch_y - PORCH_OVERHANG_HEIGHT / 2.0 - 0.03, col_z),
        )?)?;
    }

    // === Chimney ===
    if params.show_chimney {
        let chimney_x = -params.main_width * 0.25;
        // Compute roof surface height at chimney position
        let dist_from_edge = params.main_width / 2.0 - chimney_x.abs();
        let roof_surface_y = roof_base_y + dist_from_edge * params.roof_angle.to_radians().tan();
        // Chimney penetrates roof — base below roof surface, top above
        let chimney_base_y = roof_surface_y - 0.3;
        let chimney_total_h = CHIMNEY_HEIGHT + 0.3;
        let chimney_y = chimney_base_y + chimney_total_h / 2.0;

        mesh.combine_with(make_box(
            CHIMNEY_WIDTH, chimney_total_h, CHIMNEY_DEPTH,
            vec3(chimney_x, chimney_y, 0.0),
        )?)?;

        // Chimney cap
        let chimney_top_y = chimney_base_y + chimney_total_h;
        mesh.combine_with(make_box(
            CHIMNEY_WIDTH + CHIMNEY_CAP_OVERHANG * 2.0,
            CHIMNEY_CAP_HEIGHT,
            CHIMNEY_DEPTH + CHIMNEY_CAP_OVERHANG * 2.0,
            vec3(chimney_x, chimney_top_y + CHIMNEY_CAP_HEIGHT / 2.0, 0.0),
        )?)?;

        // Chimney flue (small box on top)
        mesh.combine_with(make_box(
            CHIMNEY_WIDTH * 0.5, 0.06, CHIMNEY_DEPTH * 0.5,
            vec3(chimney_x, chimney_top_y + CHIMNEY_CAP_HEIGHT + 0.03, 0.0),
        )?)?;
    }

    // === Balcony on wing ===
    if params.show_balcony {
        let balcony_y = FOUNDATION_HEIGHT + params.story_height;
        let balcony_z = wing_z + params.wing_depth / 2.0;

        // Slab
        mesh.combine_with(make_box(
            params.wing_width * 0.8,
            BALCONY_SLAB_HEIGHT,
            BALCONY_DEPTH,
            vec3(wing_offset_x, balcony_y - BALCONY_SLAB_HEIGHT / 2.0, balcony_z + BALCONY_DEPTH / 2.0),
        )?)?;

        // Railing - front
        let rail_y = balcony_y + BALCONY_RAIL_HEIGHT / 2.0;
        let rail_z = balcony_z + BALCONY_DEPTH - BALCONY_RAIL_THICKNESS / 2.0;
        mesh.combine_with(make_box(
            params.wing_width * 0.8,
            BALCONY_RAIL_THICKNESS,
            BALCONY_RAIL_THICKNESS,
            vec3(wing_offset_x, balcony_y + BALCONY_RAIL_HEIGHT, rail_z),
        )?)?;

        // Railing - sides
        let rail_half_w = params.wing_width * 0.4;
        mesh.combine_with(make_box(
            BALCONY_RAIL_THICKNESS,
            BALCONY_RAIL_THICKNESS,
            BALCONY_DEPTH,
            vec3(wing_offset_x - rail_half_w, balcony_y + BALCONY_RAIL_HEIGHT, balcony_z + BALCONY_DEPTH / 2.0),
        )?)?;
        mesh.combine_with(make_box(
            BALCONY_RAIL_THICKNESS,
            BALCONY_RAIL_THICKNESS,
            BALCONY_DEPTH,
            vec3(wing_offset_x + rail_half_w, balcony_y + BALCONY_RAIL_HEIGHT, balcony_z + BALCONY_DEPTH / 2.0),
        )?)?;

        // Baluster posts
        let n_posts = 5;
        let post_spacing = params.wing_width * 0.8 / (n_posts as f32 + 1.0);
        for i in 0..n_posts {
            let px = wing_offset_x - rail_half_w + post_spacing * (i as f32 + 1.0);
            mesh.combine_with(make_box(
                BALCONY_POST_SIZE, BALCONY_RAIL_HEIGHT, BALCONY_POST_SIZE,
                vec3(px, rail_y, rail_z),
            )?)?;
        }

        // Corner posts (thicker)
        mesh.combine_with(make_box(
            BALCONY_POST_SIZE * 1.5, BALCONY_RAIL_HEIGHT, BALCONY_POST_SIZE * 1.5,
            vec3(wing_offset_x - rail_half_w, rail_y, rail_z),
        )?)?;
        mesh.combine_with(make_box(
            BALCONY_POST_SIZE * 1.5, BALCONY_RAIL_HEIGHT, BALCONY_POST_SIZE * 1.5,
            vec3(wing_offset_x + rail_half_w, rail_y, rail_z),
        )?)?;

        // Balcony support brackets
        let bracket_h = 0.4;
        let (mut bracket, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
        let all = bracket.select_all();
        bracket.scale(all.clone(), vec3(0.08, bracket_h, 0.3), Pivot::Origin)?;
        bracket.rotate(all.clone(), Quat::from_rotation_x(-0.15), Pivot::Origin)?;
        bracket.translate(all, vec3(
            wing_offset_x - rail_half_w + 0.1,
            balcony_y - BALCONY_SLAB_HEIGHT - bracket_h / 2.0,
            balcony_z + BALCONY_DEPTH * 0.5,
        ))?;
        mesh.combine_with(bracket)?;

        let (mut bracket2, _) = primitives::Cube { subdivision: glam::U16Vec3::ONE }.generate()?;
        let all = bracket2.select_all();
        bracket2.scale(all.clone(), vec3(0.08, bracket_h, 0.3), Pivot::Origin)?;
        bracket2.rotate(all.clone(), Quat::from_rotation_x(-0.15), Pivot::Origin)?;
        bracket2.translate(all, vec3(
            wing_offset_x + rail_half_w - 0.1,
            balcony_y - BALCONY_SLAB_HEIGHT - bracket_h / 2.0,
            balcony_z + BALCONY_DEPTH * 0.5,
        ))?;
        mesh.combine_with(bracket2)?;
    }

    // === Corner quoins (decorative corner stones) ===
    let quoin_w = 0.12;
    let quoin_d = 0.06;
    let n_quoins = 8;
    let quoin_h = (total_height - 0.2) / n_quoins as f32;
    for i in (0..n_quoins).step_by(2) {
        let qy = FOUNDATION_HEIGHT + 0.1 + quoin_h * i as f32 + quoin_h / 2.0;
        // Front-left corner
        mesh.combine_with(make_box(
            quoin_w, quoin_h * 0.85, quoin_d,
            vec3(-params.main_width / 2.0 - quoin_d / 2.0 + 0.01, qy, params.main_depth / 2.0 - quoin_w / 2.0),
        )?)?;
        mesh.combine_with(make_box(
            quoin_d, quoin_h * 0.85, quoin_w,
            vec3(-params.main_width / 2.0 + quoin_d / 2.0 - 0.01, qy, params.main_depth / 2.0 + quoin_d / 2.0 - 0.01),
        )?)?;
        // Front-right corner (main-wing junction)
        mesh.combine_with(make_box(
            quoin_d, quoin_h * 0.85, quoin_w,
            vec3(params.main_width / 2.0 - quoin_d / 2.0 + 0.01, qy, params.main_depth / 2.0 + quoin_d / 2.0 - 0.01),
        )?)?;
    }

    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn update_house_system(
    params: Res<HouseParameters>,
    houses: Query<Entity, With<HouseTag>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if params.is_changed() {
        for e in &houses {
            let smesh = generate_house(&params).unwrap();
            let v0 = smesh.vertices().next().unwrap();
            commands.entity(e).insert((
                Mesh3d(meshes.add(Mesh::from(smesh.clone()))),
                DebugRenderSMesh {
                    mesh: smesh,
                    selection: Selection::Vertex(v0),
                    visible: false,
                },
            ));
        }
    }
}

fn init_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(HouseParameters::default());

    commands.spawn((
        HouseTag,
        Transform::from_scale(Vec3::splat(0.1)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.85, 0.78, 0.65),
            perceptual_roughness: 0.8,
            ..default()
        })),
    ));

    commands.spawn((
        Camera3d::default(),
        Msaa::Off,
        bevy::pbr::ScreenSpaceAmbientOcclusion::default(),
        bevy::anti_alias::taa::TemporalAntiAliasing::default(),
        PanOrbitCamera {
            focus: vec3(0.2, 0.4, 0.0),
            radius: Some(2.5),
            yaw: Some(0.5),
            pitch: Some(0.35),
            ..default()
        },
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ShowcasePlugin, PanOrbitCameraPlugin, EguiPlugin::default()))
        .add_plugins(ResourceInspectorPlugin::<HouseParameters>::default())
        .add_systems(Startup, init_system)
        .add_systems(Update, update_house_system)
        .register_type::<HouseParameters>()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_generation_produces_valid_mesh() {
        let mesh = generate_house(&HouseParameters::default()).unwrap();
        let report = mesh.describe();
        let validation = mesh.validate();

        eprintln!("=== House mesh ===\n{}", report);
        eprintln!("{}", validation);

        assert!(report.vertex_count > 500, "Too few vertices: {}", report.vertex_count);
        assert!(report.face_count > 200, "Too few faces: {}", report.face_count);
        assert!(report.is_manifold, "House should be manifold");

        for issue in &validation.issues {
            match issue {
                MeshIssue::IsolatedVertex { .. } => panic!("Has isolated vertices"),
                MeshIssue::BrokenConnectivity { .. } => panic!("Broken connectivity: {}", issue),
                _ => {}
            }
        }

        // House should be roughly 10.5m wide (main 7 + wing 3.5), ~9m tall (with roof), ~6m deep
        assert!(report.dimensions.x > 8.0 && report.dimensions.x < 14.0,
            "Width should be ~10.5m, got {}", report.dimensions.x);
        assert!(report.dimensions.y > 6.0 && report.dimensions.y < 13.0,
            "Height should be ~9m, got {}", report.dimensions.y);
        assert!(report.dimensions.z > 5.0 && report.dimensions.z < 10.0,
            "Depth should be ~7m, got {}", report.dimensions.z);
    }

    #[test]
    fn house_with_custom_params() {
        let params = HouseParameters {
            main_width: 8.0,
            main_depth: 7.0,
            wing_width: 4.0,
            show_chimney: false,
            show_porch: false,
            show_balcony: false,
            windows_per_floor: 4,
            ..Default::default()
        };
        let mesh = generate_house(&params).unwrap();
        let report = mesh.describe();
        assert!(report.is_manifold);
    }

    #[test]
    #[cfg(feature = "preview")]
    fn house_preview() {
        let mesh = generate_house(&HouseParameters::default()).unwrap();
        eprintln!("=== House mesh ===\n{}", mesh.describe());
        eprintln!("{}", mesh.validate());
        let opts = smesh::smesh::preview::PreviewOptions::default()
            .with_size(1024, 1024)
            .with_wireframe();
        let paths = mesh.save_preview_with_options(&opts, "/tmp").unwrap();
        eprintln!("Saved previews: {:?}", paths);
    }

}
