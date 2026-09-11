//! LANTERNFALL CROSSING — a hand-composed, procedural low-poly RPG diorama.
//! Run: cargo run --example rpg_scene
//! Drag to orbit, scroll to zoom, 1/2/3 for composed views, Space to pause motion.
//! All modeled geometry comes from SMesh. No external models, textures or assets.
use std::{
    error::Error,
    f32::consts::{FRAC_PI_2, PI, TAU},
    path::PathBuf,
};

use bevy::{
    app::AppExit,
    camera::ScalingMode,
    post_process::bloom::Bloom,
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use glam::{vec3, U16Vec3};
use smesh::{
    prelude::*,
    smesh::{
        primitives::{Cube, Cylinder, Icosphere, Primitive, Wedge},
        transform::Pivot,
    },
};

#[derive(Clone, Copy)]
#[repr(usize)]
enum Shape {
    Box,
    Post,
    Disk,
    Rock,
    Boulder,
    Gable,
    Cone,
    Crystal,
    Cliff,
    Turf,
    Ring,
    Banner,
    Cap,
    Cloak,
    Brick,
}
#[derive(Clone, Copy)]
#[repr(usize)]
enum Ink {
    Cliff,
    Slate,
    Stone,
    StoneLight,
    Moss,
    Grass,
    GrassLight,
    Pine,
    PineLight,
    Wood,
    Timber,
    Plank,
    Rope,
    Plaster,
    Roof,
    RoofLight,
    RoofDark,
    Iron,
    Gold,
    Water,
    Foam,
    Magic,
    Portal,
    Amber,
    Red,
    Cloth,
    Skin,
    Smoke,
    Night,
}
const COLORS: [[f32; 3]; 29] = [
    [0.20, 0.27, 0.33],
    [0.30, 0.39, 0.43],
    [0.48, 0.55, 0.53],
    [0.64, 0.67, 0.57],
    [0.32, 0.43, 0.23],
    [0.20, 0.36, 0.29],
    [0.44, 0.55, 0.29],
    [0.10, 0.27, 0.24],
    [0.20, 0.40, 0.31],
    [0.34, 0.19, 0.12],
    [0.17, 0.12, 0.12],
    [0.58, 0.36, 0.19],
    [0.71, 0.57, 0.35],
    [0.78, 0.67, 0.44],
    [0.63, 0.22, 0.14],
    [0.79, 0.34, 0.18],
    [0.43, 0.15, 0.14],
    [0.13, 0.19, 0.23],
    [0.90, 0.61, 0.22],
    [0.06, 0.43, 0.48],
    [0.44, 0.86, 0.81],
    [0.15, 0.83, 0.92],
    [0.028, 0.070, 0.13],
    [1.0, 0.60, 0.16],
    [0.55, 0.13, 0.22],
    [0.24, 0.28, 0.47],
    [0.75, 0.49, 0.30],
    [0.35, 0.41, 0.46],
    [0.035, 0.055, 0.085],
];

struct Model {
    name: &'static str,
    mesh: SMesh,
}
struct Part {
    name: &'static str,
    shape: Shape,
    ink: Ink,
    transform: Transform,
    motion: Option<Motion>,
}
struct Lamp {
    position: Vec3,
    color: Color,
    intensity: f32,
    radius: f32,
}
#[derive(Resource)]
struct Diorama {
    models: Vec<Model>,
    parts: Vec<Part>,
    lamps: Vec<Lamp>,
}

#[derive(Component, Clone, Copy)]
enum Motion {
    Hover { origin: Vec3, phase: f32 },
    Smoke { origin: Vec3, phase: f32, size: f32 },
    Spark { origin: Vec3, phase: f32 },
    Fall { origin: Vec3, phase: f32 },
}
#[derive(Resource, Default)]
struct Clock {
    time: f32,
    paused: bool,
}

fn cylinder(segments: usize) -> SMeshResult<SMesh> {
    Ok(Cylinder {
        segments,
        height: 1.0,
        radius: 1.0,
    }
    .generate()?
    .0)
}

// Extruded ring profiles keep the cliff and crystal silhouettes coarse and authored.
fn cliff(turf: bool) -> SMeshResult<SMesh> {
    let (mut mesh, data) = Cylinder {
        segments: 14,
        height: 0.18,
        radius: 1.0,
    }
    .generate()?;
    for (_, p) in mesh.positions.iter_mut() {
        let a = p.z.atan2(p.x);
        let radius = 1.0 + 0.035 * (3.0 * a + 0.4).sin() + 0.025 * (5.0 * a).cos();
        p.x *= radius;
        p.z *= radius;
        p.y -= 0.09;
    }
    if !turf {
        let mut bottom = data.bottom_face;
        for (drop, scale, drift) in [
            (0.55, 1.025, 0.01),
            (1.10, 0.87, -0.06),
            (1.15, 0.58, 0.10),
            (0.35, 0.30, -0.02),
        ] {
            bottom = mesh.extrude(bottom)?;
            mesh.scale(bottom, Vec3::splat(scale), Pivot::SelectionCog)?;
            mesh.translate(bottom, vec3(drift, -drop, 0.02))?;
        }
    }
    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn ring() -> SMeshResult<SMesh> {
    let mut mesh = SMesh::new();
    let mut rings = Vec::new();
    for (radius, y) in [(1.0, -0.5), (1.0, 0.5), (0.76, 0.5), (0.76, -0.5)] {
        rings.push(
            (0..12)
                .map(|i| {
                    let a = i as f32 * TAU / 12.0;
                    mesh.add_vertex(vec3(radius * a.cos(), y, radius * a.sin()))
                })
                .collect::<Vec<_>>(),
        );
    }
    for r in 0..4 {
        for i in 0..12 {
            let j = (i + 1) % 12;
            mesh.make_quad(
                rings[r][i],
                rings[(r + 1) % 4][i],
                rings[(r + 1) % 4][j],
                rings[r][j],
            )?;
        }
    }
    mesh.recalculate_normals()?;
    Ok(mesh)
}

fn models() -> SMeshResult<Vec<Model>> {
    let cube = Cube {
        subdivision: U16Vec3::ONE,
    }
    .generate()?
    .0;
    let mut cone = cylinder(7)?;
    for (_, p) in cone.positions.iter_mut() {
        if p.y > 0.0 {
            p.x *= 0.035;
            p.z *= 0.035;
        }
        p.y += 0.5;
    }
    let (mut crystal, data) = Cylinder {
        segments: 6,
        height: 0.72,
        radius: 0.34,
    }
    .generate()?;
    for (face, height) in [(data.top_face, 0.40), (data.bottom_face, -0.20)] {
        let point = crystal.extrude(face)?;
        crystal.translate(point, Vec3::Y * height)?;
        crystal.scale(point, Vec3::splat(0.025), Pivot::SelectionCog)?;
    }
    let mut banner = Cube {
        subdivision: U16Vec3::new(3, 4, 1),
    }
    .generate()?
    .0;
    for (_, p) in banner.positions.iter_mut() {
        p.z = p.z * 0.025 + (p.x * 5.0 + p.y * 3.0).sin() * (0.5 - p.y) * 0.13;
        if p.y < -0.49 {
            p.y -= 0.25 * (1.0 - 2.0 * p.x.abs());
        }
    }
    let mut cap = cylinder(8)?;
    for (_, p) in cap.positions.iter_mut() {
        if p.y > 0.0 {
            p.x *= 0.40;
            p.z *= 0.40;
        }
        p.y = (p.y + 0.5) * 0.35;
    }
    let mut cloak = cube.clone();
    for (_, p) in cloak.positions.iter_mut() {
        p.x *= if p.y > 0.0 { 0.55 } else { 1.15 };
        p.z = p.z * 0.28 - (0.5 - p.y) * 0.30;
    }
    let mut brick = cube.clone();
    let top = brick.faces_facing(Vec3::Y, 0.1)[0];
    let inset = brick.inset(top, 0.12)?;
    brick.translate(inset, Vec3::Y * 0.045)?;
    let bottom = brick.faces_facing(Vec3::NEG_Y, 0.1)[0];
    let inset = brick.inset(bottom, 0.10)?;
    brick.translate(inset, Vec3::NEG_Y * 0.035)?;
    let mut result = vec![
        Model {
            name: "cube",
            mesh: cube,
        },
        Model {
            name: "octagonal timber",
            mesh: cylinder(8)?,
        },
        Model {
            name: "twelve-sided disk",
            mesh: cylinder(12)?,
        },
        Model {
            name: "angular rock",
            mesh: Icosphere { subdivisions: 0 }.generate()?.0,
        },
        Model {
            name: "faceted boulder",
            mesh: Icosphere { subdivisions: 1 }.generate()?.0,
        },
        Model {
            name: "gable prism",
            mesh: Wedge {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            }
            .generate()?
            .0,
        },
        Model {
            name: "seven-sided foliage",
            mesh: cone,
        },
        Model {
            name: "extruded crystal",
            mesh: crystal,
        },
        Model {
            name: "extruded cliff",
            mesh: cliff(false)?,
        },
        Model {
            name: "grass cap",
            mesh: cliff(true)?,
        },
        Model {
            name: "closed hollow ring",
            mesh: ring()?,
        },
        Model {
            name: "folded swallowtail cloth",
            mesh: banner,
        },
        Model {
            name: "mushroom cap",
            mesh: cap,
        },
        Model {
            name: "traveler cloak",
            mesh: cloak,
        },
        Model {
            name: "inset masonry",
            mesh: brick,
        },
    ];
    for model in &mut result {
        model.mesh.recalculate_normals()?;
    }
    Ok(result)
}

impl Diorama {
    fn put(
        &mut self,
        name: &'static str,
        shape: Shape,
        ink: Ink,
        p: Vec3,
        size: Vec3,
        rotation: Quat,
    ) -> usize {
        let index = self.parts.len();
        self.parts.push(Part {
            name,
            shape,
            ink,
            transform: Transform::from_translation(p)
                .with_scale(size)
                .with_rotation(rotation),
            motion: None,
        });
        index
    }
    fn prop(&mut self, name: &'static str, shape: Shape, ink: Ink, p: Vec3, size: Vec3) -> usize {
        self.put(name, shape, ink, p, size, Quat::IDENTITY)
    }
    fn block(&mut self, ink: Ink, p: Vec3, size: Vec3) {
        self.prop("joinery", Shape::Box, ink, p, size);
    }
    fn beam(&mut self, ink: Ink, a: Vec3, b: Vec3, radius: f32) {
        self.put(
            "beam / rope",
            Shape::Post,
            ink,
            (a + b) * 0.5,
            vec3(radius, a.distance(b), radius),
            Quat::from_rotation_arc(Vec3::Y, (b - a).normalize()),
        );
    }
    fn stone(&mut self, p: Vec3, size: Vec3, yaw: f32, light: bool) {
        self.put(
            "cut stone",
            Shape::Brick,
            if light { Ink::StoneLight } else { Ink::Stone },
            p,
            size,
            Quat::from_rotation_y(yaw),
        );
    }
    fn lantern(&mut self, p: Vec3, scale: f32) {
        self.prop(
            "lantern glow",
            Shape::Post,
            Ink::Amber,
            p,
            vec3(0.105, 0.25, 0.105) * scale,
        );
        for y in [-0.16, 0.16] {
            self.prop(
                "lantern frame",
                Shape::Post,
                Ink::Iron,
                p + Vec3::Y * y * scale,
                vec3(0.15, 0.055, 0.15) * scale,
            );
        }
        for (x, z) in [(-0.09, -0.09), (-0.09, 0.09), (0.09, -0.09), (0.09, 0.09)] {
            self.beam(
                Ink::Iron,
                p + vec3(x, -0.16, z) * scale,
                p + vec3(x, 0.16, z) * scale,
                0.015 * scale,
            );
        }
        self.prop(
            "lantern roof",
            Shape::Cone,
            Ink::Iron,
            p + Vec3::Y * 0.19 * scale,
            vec3(0.17, 0.10, 0.17) * scale,
        );
    }
    fn light(&mut self, p: Vec3, color: [f32; 3], intensity: f32) {
        self.lamps.push(Lamp {
            position: p,
            color: Color::srgb(color[0], color[1], color[2]),
            intensity,
            radius: 0.25,
        });
    }
}

fn landscape(s: &mut Diorama, rng: &mut fastrand::Rng) {
    for (center, scale) in [
        (vec3(-4.1, 0.0, -0.1), vec3(4.25, 1.0, 5.0)),
        (vec3(4.15, 0.65, -0.65), vec3(3.15, 1.0, 4.65)),
    ] {
        s.prop(
            "floating escarpment",
            Shape::Cliff,
            Ink::Cliff,
            center,
            scale,
        );
        s.prop(
            "heather-covered rim",
            Shape::Turf,
            Ink::Grass,
            center + Vec3::Y * 0.025,
            scale * vec3(1.015, 1.0, 1.015),
        );
        for i in 0..18 {
            let a = i as f32 * TAU / 18.0;
            let p = center + vec3(a.cos() * scale.x * 0.94, -0.22, a.sin() * scale.z * 0.94);
            let size = vec3(
                0.35 + rng.f32() * 0.32,
                0.30 + rng.f32() * 0.45,
                0.35 + rng.f32() * 0.30,
            );
            s.put(
                "exposed rim rock",
                Shape::Rock,
                if i % 3 == 0 { Ink::Moss } else { Ink::Slate },
                p,
                size,
                Quat::from_rotation_y(a),
            );
            if i % 2 == 0 {
                s.put(
                    "basalt ribs",
                    Shape::Rock,
                    Ink::Slate,
                    p - Vec3::Y * 1.10,
                    size * vec3(0.9, 2.8, 0.9),
                    Quat::from_rotation_z(0.18 * a.sin()),
                );
            }
        }
    }
    // A spring threads the cleft; the bridge and waterfall share the same channel.
    s.prop(
        "upper spring",
        Shape::Disk,
        Ink::Water,
        vec3(0.45, -0.15, -2.8),
        vec3(1.3, 0.08, 1.65),
    );
    s.block(Ink::Water, vec3(0.52, -0.14, -0.1), vec3(1.22, 0.08, 5.4));
    for i in 0..11 {
        let z = -3.2 + i as f32 * 0.51;
        s.block(
            Ink::Foam,
            vec3(0.45 + 0.15 * (i as f32).sin(), -0.09, z),
            vec3(0.35, 0.014, 0.035),
        );
    }
    // Layered ribbons retain a polygonal silhouette rather than a transparent plane.
    for i in 0..7 {
        let x = 0.03 + i as f32 * 0.16;
        let y = -1.80 + 0.06 * (i as f32).sin();
        s.put(
            "falling spring water",
            Shape::Box,
            if i % 3 == 0 { Ink::Foam } else { Ink::Water },
            vec3(x, y, 2.65),
            vec3(0.165, 3.4, 0.16),
            Quat::from_rotation_x(-0.025),
        );
        let origin = vec3(x, -0.25, 2.77);
        let id = s.prop(
            "moving waterfall streak",
            Shape::Box,
            Ink::Foam,
            origin,
            vec3(0.055, 0.35, 0.025),
        );
        s.parts[id].motion = Some(Motion::Fall {
            origin,
            phase: i as f32 * 0.15,
        });
    }
    for i in 0..9 {
        let a = i as f32 * TAU / 9.0;
        s.prop(
            "waterfall spray",
            Shape::Rock,
            Ink::Foam,
            vec3(0.5 + a.cos() * 0.53, -3.47, a.sin() * 0.27 + 2.78),
            vec3(0.32, 0.12, 0.23),
        );
    }
    s.prop(
        "receiving pool",
        Shape::Disk,
        Ink::Water,
        vec3(0.53, -3.68, 3.09),
        vec3(1.42, 0.065, 0.97),
    );
    for radius in [0.56, 0.92, 1.24] {
        for i in 0..14 {
            if i % 5 == 0 {
                continue;
            }
            let a = i as f32 * TAU / 14.0;
            let b = (i + 1) as f32 * TAU / 14.0;
            s.beam(
                Ink::Foam,
                vec3(
                    0.53 + radius * a.cos(),
                    -3.639,
                    3.09 + radius * 0.61 * a.sin(),
                ),
                vec3(
                    0.53 + radius * b.cos(),
                    -3.639,
                    3.09 + radius * 0.61 * b.sin(),
                ),
                0.012,
            );
        }
    }
    for i in 0..8 {
        let a = i as f32 * TAU / 8.0;
        s.prop(
            "stones around the plunge pool",
            Shape::Rock,
            Ink::Slate,
            vec3(0.53 + 1.43 * a.cos(), -3.65, 3.09 + 0.98 * a.sin()),
            vec3(0.17, 0.12, 0.15),
        );
    }
    s.prop(
        "shadow beneath the islands",
        Shape::Disk,
        Ink::Night,
        vec3(0.0, -3.85, 0.0),
        vec3(10.5, 0.22, 7.3),
    );
    s.block(Ink::Night, vec3(0.0, -4.04, 0.0), vec3(200.0, 0.10, 200.0));
    // Irregular stepping stones lead the eye from the well through the crossing.
    for i in 0..15 {
        let t = i as f32 / 14.0;
        let p = vec3(
            -5.5 + 4.0 * t,
            0.075,
            3.45 - 2.2 * t + 0.28 * (t * PI).sin(),
        );
        s.stone(
            p,
            vec3(0.44 + rng.f32() * 0.2, 0.11, 0.32 + rng.f32() * 0.2),
            rng.f32() * 1.2,
            false,
        );
    }
    for i in 0..7 {
        let t = i as f32 / 6.0;
        s.stone(
            vec3(2.65 + 1.2 * t, 0.73, 1.1 - 2.35 * t),
            vec3(0.50, 0.12, 0.40),
            rng.f32(),
            i % 3 == 0,
        );
    }
}

fn inn(s: &mut Diorama, rng: &mut fastrand::Rng) {
    let c = vec3(-4.45, 0.0, -1.75);
    s.stone(c + vec3(0.0, 0.12, 0.0), vec3(3.65, 0.26, 3.20), 0.0, false);
    s.block(
        Ink::Plaster,
        c + vec3(0.0, 1.52, 0.0),
        vec3(3.30, 2.65, 2.90),
    );
    for y in [0.40, 1.50, 2.83] {
        s.block(Ink::Timber, c + vec3(0.0, y, 0.0), vec3(3.48, 0.13, 3.08));
    }
    for x in [-1.66, 0.0, 1.66] {
        for z in [-1.47, 1.47] {
            s.block(Ink::Wood, c + vec3(x, 1.58, z), vec3(0.13, 2.6, 0.13));
        }
    }
    for z in [-0.65, 0.65] {
        s.block(Ink::Wood, c + vec3(1.68, 1.63, z), vec3(0.12, 2.5, 0.12));
    }
    for side in [-1.0, 1.0] {
        s.beam(
            Ink::Timber,
            c + vec3(side * 1.6, 1.6, 1.50),
            c + vec3(side * 0.9, 2.74, 1.50),
            0.065,
        );
        s.beam(
            Ink::Timber,
            c + vec3(1.72, 1.6, side * 1.40),
            c + vec3(1.72, 2.72, side * 0.72),
            0.065,
        );
    }
    // Gable fill and roof; individually laid overlapping shingles, not a texture.
    s.prop(
        "plaster gable",
        Shape::Gable,
        Ink::Plaster,
        c + vec3(0.0, 2.82, 0.0),
        vec3(3.84, 1.86, 3.80),
    );
    let slope = (1.90_f32 / 2.025).atan();
    for side in [-1.0, 1.0] {
        s.put(
            "roof underlay",
            Shape::Box,
            Ink::RoofDark,
            c + vec3(side * 1.0125, 3.80, 0.0),
            vec3(2.80, 0.09, 3.92),
            Quat::from_rotation_z(-side * slope),
        );
        for row in 0..6 {
            let t = (row as f32 + 0.5) / 6.0;
            for col in 0..8 {
                let z = -1.74 + col as f32 * 0.495 + if row % 2 == 0 { 0.045 } else { 0.0 };
                let ink = match rng.usize(0..5) {
                    0 => Ink::RoofLight,
                    1 => Ink::RoofDark,
                    _ => Ink::Roof,
                };
                s.put(
                    "individual fired-clay shingle",
                    Shape::Box,
                    ink,
                    c + vec3(side * t * 2.025, 2.92 + 1.90 * (1.0 - t), z),
                    vec3(0.50, 0.065, 0.50),
                    Quat::from_rotation_z(-side * slope),
                );
            }
        }
        s.beam(
            Ink::Timber,
            c + vec3(side * 2.10, 2.84, 1.98),
            c + vec3(0.0, 4.80, 1.98),
            0.065,
        );
        s.beam(
            Ink::Timber,
            c + vec3(side * 2.10, 2.84, -1.94),
            c + vec3(0.0, 4.80, -1.94),
            0.065,
        );
    }
    s.beam(
        Ink::RoofDark,
        c + vec3(0.0, 4.80, -2.05),
        c + vec3(0.0, 4.80, 2.03),
        0.105,
    );
    for z in [-2.08, 2.08] {
        s.beam(
            Ink::Gold,
            c + vec3(0.0, 4.78, z),
            c + vec3(0.0, 5.10, z + z.signum() * 0.18),
            0.045,
        );
    }
    // Small attic lantern window beneath the ridge.
    s.put(
        "attic glow",
        Shape::Disk,
        Ink::Amber,
        c + vec3(0.0, 3.62, 1.928),
        vec3(0.30, 0.025, 0.30),
        Quat::from_rotation_x(FRAC_PI_2),
    );
    s.put(
        "attic window frame",
        Shape::Ring,
        Ink::Timber,
        c + vec3(0.0, 3.62, 1.95),
        vec3(0.35, 0.075, 0.35),
        Quat::from_rotation_x(FRAC_PI_2),
    );
    s.block(
        Ink::Timber,
        c + vec3(0.0, 3.62, 1.993),
        vec3(0.045, 0.60, 0.045),
    );
    s.block(
        Ink::Timber,
        c + vec3(0.0, 3.62, 1.993),
        vec3(0.60, 0.045, 0.045),
    );
    s.beam(
        Ink::Timber,
        c + vec3(0.0, 4.00, 1.95),
        c + vec3(0.0, 4.62, 1.95),
        0.046,
    );
    s.beam(
        Ink::Timber,
        c + vec3(-1.84, 2.88, 1.95),
        c + vec3(1.84, 2.88, 1.95),
        0.057,
    );
    for side in [-1.0, 1.0] {
        s.beam(
            Ink::Timber,
            c + vec3(side * 1.35, 2.93, 1.95),
            c + vec3(side * 0.44, 3.72, 1.95),
            0.044,
        );
        s.block(
            Ink::Wood,
            c + vec3(side * 0.94, 3.18, 1.95),
            vec3(0.055, 0.55, 0.06),
        );
    }
    // Door, iron hinges, latch and two warm leaded windows.
    s.block(
        Ink::Timber,
        c + vec3(0.0, 0.97, 1.51),
        vec3(0.86, 1.60, 0.12),
    );
    for i in 0..5 {
        s.block(
            Ink::Wood,
            c + vec3(-0.32 + i as f32 * 0.16, 0.98, 1.59),
            vec3(0.135, 1.46, 0.045),
        );
    }
    for y in [0.55, 1.27] {
        s.block(Ink::Iron, c + vec3(0.0, y, 1.63), vec3(0.77, 0.055, 0.03));
    }
    s.prop(
        "door latch",
        Shape::Rock,
        Ink::Gold,
        c + vec3(0.24, 0.98, 1.66),
        Vec3::splat(0.055),
    );
    for x in [-1.10, 1.10] {
        s.block(Ink::Timber, c + vec3(x, 2.12, 1.51), vec3(0.81, 0.92, 0.12));
        s.block(
            Ink::Amber,
            c + vec3(x, 2.12, 1.585),
            vec3(0.62, 0.73, 0.035),
        );
        for dx in [-0.30, 0.0, 0.30] {
            s.block(
                Ink::Wood,
                c + vec3(x + dx, 2.12, 1.62),
                vec3(0.043, 0.78, 0.045),
            );
        }
        s.block(Ink::Wood, c + vec3(x, 2.12, 1.62), vec3(0.65, 0.05, 0.045));
        s.block(Ink::Plank, c + vec3(x, 1.65, 1.68), vec3(0.87, 0.09, 0.25));
        for dx in [-0.12, 0.12] {
            s.prop(
                "window-box herbs",
                Shape::Rock,
                Ink::GrassLight,
                c + vec3(x + dx, 1.81, 1.73),
                vec3(0.15, 0.17, 0.12),
            );
        }
    }
    // Side bay window faces the crossing.
    s.block(
        Ink::Timber,
        c + vec3(1.81, 1.95, -0.1),
        vec3(0.30, 1.02, 1.35),
    );
    s.block(
        Ink::Amber,
        c + vec3(1.98, 1.95, -0.1),
        vec3(0.035, 0.79, 1.12),
    );
    for z in [-0.56, -0.1, 0.36] {
        s.block(Ink::Wood, c + vec3(2.01, 1.96, z), vec3(0.05, 0.85, 0.045));
    }
    s.block(
        Ink::Wood,
        c + vec3(2.01, 1.95, -0.1),
        vec3(0.05, 0.045, 1.15),
    );
    // Front porch boards, shelter and knee braces.
    for i in 0..12 {
        s.block(
            Ink::Plank,
            c + vec3(-1.8 + i as f32 * 0.32, 0.23, 2.03),
            vec3(0.30, 0.10, 1.04),
        );
    }
    for x in [-1.75, 1.75] {
        s.beam(
            Ink::Wood,
            c + vec3(x, 0.2, 2.48),
            c + vec3(x, 1.86, 2.48),
            0.07,
        );
        s.beam(
            Ink::Wood,
            c + vec3(x, 1.45, 2.48),
            c + vec3(x * 0.77, 1.84, 2.48),
            0.04,
        );
    }
    s.put(
        "porch lean-to roof",
        Shape::Box,
        Ink::RoofDark,
        c + vec3(0.0, 1.93, 2.05),
        vec3(3.85, 0.09, 1.3),
        Quat::from_rotation_x(0.22),
    );
    for i in 0..12 {
        s.put(
            "porch shingles",
            Shape::Box,
            if i % 3 == 0 {
                Ink::RoofLight
            } else {
                Ink::Roof
            },
            c + vec3(-1.77 + i as f32 * 0.32, 1.99, 2.05),
            vec3(0.30, 0.055, 1.3),
            Quat::from_rotation_x(0.22),
        );
    }
    for i in 0..3 {
        s.stone(
            c + vec3(0.0, 0.07 + i as f32 * 0.055, 2.93 - i as f32 * 0.20),
            vec3(1.12, 0.11, 0.35),
            0.0,
            i % 2 == 0,
        );
    }
    // Hanging inn sign: a crescent and a moth, built from small geometric pieces.
    s.beam(
        Ink::Iron,
        c + vec3(1.72, 2.64, 1.65),
        c + vec3(2.60, 2.64, 1.65),
        0.035,
    );
    for x in [2.03, 2.44] {
        s.beam(
            Ink::Iron,
            c + vec3(x, 2.64, 1.65),
            c + vec3(x, 2.33, 1.65),
            0.012,
        );
    }
    s.block(
        Ink::Timber,
        c + vec3(2.24, 2.04, 1.65),
        vec3(0.79, 0.58, 0.095),
    );
    for i in 0..7 {
        let a = 0.60 + i as f32 * 0.70;
        s.prop(
            "crescent on inn sign",
            Shape::Rock,
            Ink::Gold,
            c + vec3(2.24 + 0.17 * a.cos(), 2.05 + 0.17 * a.sin(), 1.71),
            vec3(0.045, 0.060, 0.018),
        );
    }
    s.lantern(c + vec3(-1.75, 1.48, 2.48), 1.0);
    s.light(c + vec3(0.0, 1.55, 2.0), [1.0, 0.52, 0.19], 420.0);
    // Crooked masonry chimney and slowly drifting smoke.
    for row in 0..7 {
        s.stone(
            c + vec3(-1.15 + row as f32 * 0.015, 3.55 + row as f32 * 0.28, -0.72),
            vec3(0.60, 0.26, 0.57),
            0.0,
            row % 3 == 0,
        );
    }
    s.block(
        Ink::Iron,
        c + vec3(-1.05, 5.46, -0.72),
        vec3(0.77, 0.13, 0.73),
    );
    for i in 0..5 {
        let origin = c + vec3(-1.05, 5.60, -0.72);
        let id = s.prop(
            "chimney smoke",
            Shape::Boulder,
            Ink::Smoke,
            origin,
            Vec3::splat(0.19),
        );
        s.parts[id].motion = Some(Motion::Smoke {
            origin,
            phase: i as f32 / 5.0,
            size: 0.22,
        });
    }
}

fn moon_gate(s: &mut Diorama, rng: &mut fastrand::Rng) {
    let c = vec3(4.20, 0.65, -2.15);
    for (y, r) in [(0.08, 1.48), (0.22, 1.27), (0.34, 1.07)] {
        s.prop(
            "moon gate dais",
            Shape::Disk,
            Ink::Slate,
            c + vec3(0.0, y, 0.75),
            vec3(r, 0.14, r * 0.75),
        );
    }
    for side in [-1.0, 1.0] {
        s.stone(
            c + vec3(side * 1.65, 0.16, 0.0),
            vec3(0.93, 0.32, 0.92),
            0.0,
            true,
        );
        for row in 0..5 {
            s.stone(
                c + vec3(side * 1.65, 0.52 + row as f32 * 0.41, 0.0),
                vec3(0.60, 0.39, 0.70),
                if row % 2 == 0 { 0.035 * side } else { 0.0 },
                row % 3 == 0,
            );
        }
        s.stone(
            c + vec3(side * 1.65, 2.32, 0.0),
            vec3(0.80, 0.23, 0.83),
            0.0,
            true,
        );
    }
    let spring = c + vec3(0.0, 2.32, 0.0);
    for i in 0..11 {
        let a = (i as f32 + 0.5) * PI / 11.0;
        s.put(
            "arch voussoir",
            Shape::Brick,
            if i % 3 == 0 {
                Ink::StoneLight
            } else {
                Ink::Stone
            },
            spring + vec3(a.cos() * 1.65, a.sin() * 1.65, 0.0),
            vec3(0.46, 0.55, 0.75),
            Quat::from_rotation_z(a - FRAC_PI_2),
        );
    }
    s.put(
        "moon keystone",
        Shape::Brick,
        Ink::StoneLight,
        spring + vec3(0.0, 1.79, 0.05),
        vec3(0.51, 0.71, 0.88),
        Quat::IDENTITY,
    );
    // A dark portal remains readable beneath glowing runes, not a blown-out white disk.
    let portal = c + vec3(0.0, 2.17, -0.11);
    s.put(
        "still surface of the moon gate",
        Shape::Disk,
        Ink::Portal,
        portal,
        vec3(1.40, 0.055, 1.65),
        Quat::from_rotation_x(FRAC_PI_2),
    );
    for i in 0..16 {
        let a = i as f32 * TAU / 16.0;
        let p = portal + vec3(1.24 * a.cos(), 1.44 * a.sin(), 0.07);
        s.put(
            "broken runic halo",
            Shape::Box,
            Ink::Magic,
            p,
            vec3(0.17, 0.035, 0.035),
            Quat::from_rotation_z(a + FRAC_PI_2),
        );
        if i % 2 == 0 {
            s.put(
                "outer rune ticks",
                Shape::Box,
                Ink::Gold,
                p + vec3(0.085 * a.cos(), 0.10 * a.sin(), 0.005),
                vec3(0.08, 0.028, 0.025),
                Quat::from_rotation_z(a),
            );
        }
    }
    // Broken concentric sigils and a crescent, with empty space around the relic.
    for i in 0..24 {
        if i % 6 == 0 {
            continue;
        }
        let a = i as f32 * TAU / 24.0;
        let b = (i + 1) as f32 * TAU / 24.0;
        s.beam(
            Ink::Magic,
            portal + vec3(0.84 * a.cos(), 0.94 * a.sin(), 0.08),
            portal + vec3(0.84 * b.cos(), 0.94 * b.sin(), 0.08),
            0.009,
        );
    }
    let moon = portal + vec3(-0.09, 0.25, 0.10);
    for i in 0..15 {
        let a = 0.72 + i as f32 * 0.32;
        let b = a + 0.32;
        s.beam(
            Ink::Foam,
            moon + vec3(0.34 * a.cos(), 0.34 * a.sin(), 0.0),
            moon + vec3(0.34 * b.cos(), 0.34 * b.sin(), 0.0),
            0.025,
        );
    }
    for p in [
        vec3(0.42, 0.60, 0.10),
        vec3(-0.46, -0.26, 0.10),
        vec3(0.40, -0.32, 0.10),
    ] {
        s.prop(
            "stars beyond the gate",
            Shape::Rock,
            Ink::Magic,
            portal + p,
            vec3(0.055, 0.10, 0.025),
        );
    }
    for side in [-1.0, 1.0] {
        s.beam(
            Ink::Gold,
            portal + vec3(0.0, 1.12, 0.09),
            portal + vec3(side * 0.15, 0.94, 0.09),
            0.014,
        );
        s.beam(
            Ink::Gold,
            portal + vec3(side * 0.15, 0.94, 0.09),
            portal + vec3(0.0, 0.76, 0.09),
            0.014,
        );
    }
    s.prop(
        "relic pedestal",
        Shape::Post,
        Ink::Stone,
        c + vec3(0.0, 0.64, 0.85),
        vec3(0.36, 0.50, 0.36),
    );
    s.prop(
        "relic bowl",
        Shape::Ring,
        Ink::Gold,
        c + vec3(0.0, 0.94, 0.85),
        vec3(0.43, 0.12, 0.43),
    );
    let origin = c + vec3(0.0, 1.59, 0.85);
    let id = s.prop(
        "levitating moon shard",
        Shape::Crystal,
        Ink::Magic,
        origin,
        Vec3::splat(0.64),
    );
    s.parts[id].motion = Some(Motion::Hover { origin, phase: 0.0 });
    s.light(c + vec3(0.0, 2.3, 0.9), [0.10, 0.75, 1.0], 500.0);
    // Fallen lintels and lichen reinforce the ancient, partly reclaimed masonry.
    for i in 0..9 {
        let p = c + vec3(-2.0 + rng.f32() * 3.9, 0.18, 0.95 + rng.f32() * 0.7);
        if p.x > 3.45 && p.x < 5.05 {
            continue;
        }
        s.put(
            "fallen carved masonry",
            Shape::Brick,
            Ink::Slate,
            p,
            vec3(0.30 + rng.f32() * 0.4, 0.26, 0.35),
            Quat::from_euler(EulerRot::XYZ, 0.12, rng.f32() * TAU, 0.13),
        );
        if i % 2 == 0 {
            s.prop(
                "lichen",
                Shape::Rock,
                Ink::Moss,
                p + Vec3::Y * 0.14,
                vec3(0.2, 0.08, 0.18),
            );
        }
    }
    // An ivy strand follows one pier, with individually angled diamond leaves.
    for i in 0..12 {
        let p = c + vec3(
            -1.9 + 0.12 * (i as f32 * 0.8).sin(),
            0.30 + i as f32 * 0.25,
            0.44,
        );
        s.put(
            "gate ivy",
            Shape::Rock,
            if i % 2 == 0 {
                Ink::Moss
            } else {
                Ink::PineLight
            },
            p,
            vec3(0.15, 0.07, 0.13),
            Quat::from_rotation_z(i as f32),
        );
    }
}

fn watchtower(s: &mut Diorama) {
    let c = vec3(5.15, 0.65, -3.68);
    for row in 0..8 {
        for i in 0..10 {
            // A ragged missing quadrant reveals the interior and broken timber floor.
            if row > 3 && (2..=4).contains(&i) {
                continue;
            }
            if row > 5 && i == 5 {
                continue;
            }
            let a = (i as f32 + if row % 2 == 0 { 0.0 } else { 0.5 }) * TAU / 10.0;
            s.put(
                "watchtower masonry",
                Shape::Brick,
                if (i + row) % 5 == 0 {
                    Ink::StoneLight
                } else {
                    Ink::Slate
                },
                c + vec3(0.95 * a.cos(), 0.25 + row as f32 * 0.48, 0.95 * a.sin()),
                vec3(0.59, 0.45, 0.37),
                Quat::from_rotation_y(-a - FRAC_PI_2),
            );
        }
    }
    for i in [0, 1, 6, 7, 8, 9] {
        let a = i as f32 * TAU / 10.0;
        s.stone(
            c + vec3(0.95 * a.cos(), 4.25, 0.95 * a.sin()),
            vec3(0.53, 0.42, 0.44),
            -a - FRAC_PI_2,
            false,
        );
    }
    for i in 0..5 {
        s.block(
            Ink::Wood,
            c + vec3(-0.62 + i as f32 * 0.30, 2.46, 0.05),
            vec3(0.27, 0.08, 1.18),
        );
    }
    s.beam(
        Ink::Timber,
        c + vec3(-0.65, 2.36, -0.8),
        c + vec3(-0.65, 2.36, 0.85),
        0.07,
    );
    s.beam(
        Ink::Iron,
        c + vec3(1.0, 3.75, 0.0),
        c + vec3(1.0, 3.75, 1.2),
        0.03,
    );
    s.put(
        "torn watch banner",
        Shape::Banner,
        Ink::Cloth,
        c + vec3(1.02, 2.95, 0.63),
        vec3(0.78, 1.45, 1.0),
        Quat::from_rotation_y(FRAC_PI_2),
    );
    s.put(
        "banner heraldic diamond",
        Shape::Rock,
        Ink::Gold,
        c + vec3(1.07, 3.05, 0.62),
        vec3(0.035, 0.24, 0.15),
        Quat::IDENTITY,
    );
    // A ladder leans against the breached wall.
    for z in [-0.22, 0.22] {
        s.beam(
            Ink::Wood,
            c + vec3(1.37, 0.05, z + 0.65),
            c + vec3(0.91, 2.35, z + 0.65),
            0.033,
        );
    }
    for i in 0..8 {
        let t = i as f32 / 7.0;
        s.beam(
            Ink::Plank,
            c + vec3(1.37 - 0.46 * t, 0.05 + 2.3 * t, 0.43),
            c + vec3(1.37 - 0.46 * t, 0.05 + 2.3 * t, 0.87),
            0.023,
        );
    }
}

fn bridge_height(t: f32) -> f32 {
    0.22 + 0.57 * t - 0.38 * (t * PI).sin()
}
fn bridge(s: &mut Diorama, rng: &mut fastrand::Rng) {
    for i in 0..18 {
        let t = i as f32 / 17.0;
        let p = vec3(-1.75 + 4.50 * t, bridge_height(t), 1.10);
        s.put(
            "worn bridge plank",
            Shape::Box,
            if i % 4 == 0 { Ink::Wood } else { Ink::Plank },
            p,
            vec3(0.242, 0.12, 1.44 + rng.f32() * 0.15),
            Quat::from_rotation_y((rng.f32() - 0.5) * 0.045),
        );
        for z in [-0.62, 0.62] {
            s.prop(
                "iron nail",
                Shape::Post,
                Ink::Iron,
                p + vec3(0.0, 0.073, z),
                vec3(0.016, 0.015, 0.016),
            );
        }
    }
    for z in [0.34, 1.86] {
        for i in 0..17 {
            let t = i as f32 / 17.0;
            let u = (i + 1) as f32 / 17.0;
            for lift in [-0.13, 0.95] {
                s.beam(
                    if lift < 0.0 { Ink::Timber } else { Ink::Rope },
                    vec3(-1.75 + 4.5 * t, bridge_height(t) + lift, z),
                    vec3(-1.75 + 4.5 * u, bridge_height(u) + lift, z),
                    if lift < 0.0 { 0.07 } else { 0.026 },
                );
            }
        }
        for i in [0, 5, 11, 17] {
            let t = i as f32 / 17.0;
            let p = vec3(-1.75 + 4.5 * t, bridge_height(t), z);
            s.beam(
                if i == 0 || i == 17 {
                    Ink::Wood
                } else {
                    Ink::Rope
                },
                p - Vec3::Y * 0.10,
                p + Vec3::Y * 1.16,
                if i == 0 || i == 17 { 0.085 } else { 0.018 },
            );
            if i == 0 || i == 17 {
                for dy in [0.12, 0.19, 0.91, 0.98] {
                    s.prop(
                        "rope binding",
                        Shape::Ring,
                        Ink::Rope,
                        p + Vec3::Y * dy,
                        vec3(0.105, 0.035, 0.105),
                    );
                }
                s.prop(
                    "post finial",
                    Shape::Cone,
                    Ink::Timber,
                    p + Vec3::Y * 1.17,
                    vec3(0.12, 0.17, 0.12),
                );
            }
        }
    }
    for (x, y, z) in [(-1.75, 1.25, 0.34), (2.75, 1.82, 1.86)] {
        s.beam(
            Ink::Iron,
            vec3(x, y + 0.30, z),
            vec3(x, y + 0.30, z + 0.37),
            0.022,
        );
        s.lantern(vec3(x, y, z + 0.37), 0.8);
    }
    s.light(vec3(-1.60, 1.05, 1.0), [1.0, 0.53, 0.19], 140.0);
}

fn pine(s: &mut Diorama, p: Vec3, scale: f32, yaw: f32) {
    s.beam(
        Ink::Wood,
        p,
        p + vec3(0.09, 3.55, 0.04) * scale,
        0.13 * scale,
    );
    for (y, r, h) in [(0.75, 1.15, 1.75), (1.58, 0.94, 1.65), (2.42, 0.67, 1.50)] {
        s.put(
            "faceted fir boughs",
            Shape::Cone,
            if y > 2.0 { Ink::PineLight } else { Ink::Pine },
            p + vec3(0.02, y, 0.0) * scale,
            vec3(r, h, r) * scale,
            Quat::from_rotation_y(yaw + y * 0.9),
        );
    }
    for i in 0..3 {
        let a = yaw + i as f32 * 2.1;
        s.beam(
            Ink::Wood,
            p + Vec3::Y * 1.10 * scale,
            p + vec3(a.cos() * 0.68, 1.03, a.sin() * 0.68) * scale,
            0.043 * scale,
        );
    }
}

fn vegetation(s: &mut Diorama, rng: &mut fastrand::Rng) {
    for (p, size, yaw) in [
        (vec3(-6.75, 0.0, -3.08), 1.24, 0.1),
        (vec3(-7.12, 0.0, 0.62), 0.87, 0.6),
        (vec3(-7.04, 0.0, 2.1), 0.59, 1.4),
        (vec3(-2.10, 0.0, -3.69), 0.78, 0.2),
        (vec3(6.68, 0.65, 0.60), 0.78, 0.5),
        (vec3(6.34, 0.65, 2.20), 0.65, 1.8),
        (vec3(2.85, 0.65, -4.03), 0.77, 0.4),
    ] {
        pine(s, p, size, yaw);
    }
    // A wind-twisted tree carries a votive lantern at the near cliff edge.
    let root = vec3(5.45, 0.65, 2.78);
    let joints = [
        root,
        root + vec3(-0.12, 0.82, 0.04),
        root + vec3(0.13, 1.65, 0.09),
        root + vec3(-0.20, 2.36, 0.04),
    ];
    for (i, pair) in joints.windows(2).enumerate() {
        s.beam(Ink::Timber, pair[0], pair[1], 0.13 - i as f32 * 0.025);
    }
    for (a, b, c) in [
        (
            joints[1],
            root + vec3(-0.72, 1.47, 0.20),
            root + vec3(-1.03, 1.84, 0.13),
        ),
        (
            joints[2],
            root + vec3(0.72, 2.02, -0.08),
            root + vec3(1.00, 2.27, -0.12),
        ),
        (
            joints[2],
            root + vec3(-0.39, 2.10, -0.60),
            root + vec3(-0.52, 2.47, -0.78),
        ),
    ] {
        s.beam(Ink::Wood, a, b, 0.06);
        s.beam(Ink::Wood, b, c, 0.032);
    }
    s.beam(
        Ink::Rope,
        root + vec3(-0.76, 1.65, 0.18),
        root + vec3(-0.76, 1.24, 0.18),
        0.012,
    );
    s.lantern(root + vec3(-0.76, 1.06, 0.18), 0.65);
    // Restrict detail scatter to the banks; keep architecture, bridge and paths clear.
    for i in 0..90 {
        let east = i % 2 == 0;
        let center = if east {
            vec3(4.15, 0.65, -0.65)
        } else {
            vec3(-4.1, 0.0, -0.1)
        };
        let a = rng.f32() * TAU;
        let r = 0.72 + rng.f32() * 0.22;
        let p = center
            + vec3(
                a.cos() * if east { 3.0 } else { 4.0 } * r,
                0.04,
                a.sin() * if east { 4.4 } else { 4.7 } * r,
            );
        if (p - vec3(-4.45, p.y, -1.75)).xz().length() < 2.25
            || (p - vec3(4.2, p.y, -2.15)).xz().length() < 1.85
        {
            continue;
        }
        for j in 0..3 {
            let rot = Quat::from_rotation_y(a + j as f32 * 1.9)
                * Quat::from_rotation_z(0.16 * (j as f32 - 1.0));
            s.put(
                "heather blade",
                Shape::Gable,
                if i % 3 == 0 {
                    Ink::GrassLight
                } else {
                    Ink::Moss
                },
                p + vec3(j as f32 * 0.045, 0.0, 0.0),
                vec3(0.07, 0.16 + rng.f32() * 0.18, 0.035),
                rot,
            );
        }
        if i % 6 == 0 {
            let h = 0.14 + rng.f32() * 0.12;
            s.prop(
                "mushroom stem",
                Shape::Post,
                Ink::Plaster,
                p + Vec3::Y * h * 0.5,
                vec3(0.035, h, 0.035),
            );
            s.prop(
                "redcap mushroom",
                Shape::Cap,
                Ink::Red,
                p + Vec3::Y * h,
                vec3(0.14, 0.20, 0.14),
            );
            s.prop(
                "mushroom spot",
                Shape::Rock,
                Ink::Plaster,
                p + vec3(0.03, h + 0.065, 0.01),
                Vec3::splat(0.025),
            );
        }
    }
}

fn barrel(s: &mut Diorama, p: Vec3, size: f32) {
    s.prop(
        "oak barrel",
        Shape::Post,
        Ink::Wood,
        p + Vec3::Y * 0.30 * size,
        vec3(0.26, 0.59, 0.26) * size,
    );
    for y in [0.10, 0.49] {
        s.prop(
            "barrel hoop",
            Shape::Ring,
            Ink::Iron,
            p + Vec3::Y * y * size,
            vec3(0.275, 0.055, 0.275) * size,
        );
    }
    s.prop(
        "barrel lid",
        Shape::Disk,
        Ink::Plank,
        p + Vec3::Y * 0.60 * size,
        vec3(0.245, 0.035, 0.245) * size,
    );
    for x in [-0.12, 0.0, 0.12] {
        s.block(
            Ink::Wood,
            p + vec3(x, 0.625, 0.0) * size,
            vec3(0.012, 0.008, 0.37) * size,
        );
    }
}

fn crate_prop(s: &mut Diorama, p: Vec3, scale: f32) {
    s.block(
        Ink::Wood,
        p + Vec3::Y * 0.25 * scale,
        Vec3::splat(0.50 * scale),
    );
    for y in [0.04, 0.45] {
        s.block(
            Ink::Plank,
            p + Vec3::Y * y * scale,
            vec3(0.55, 0.065, 0.55) * scale,
        );
    }
    for x in [-0.23, 0.23] {
        s.block(
            Ink::Plank,
            p + vec3(x, 0.25, 0.27) * scale,
            vec3(0.065, 0.49, 0.045) * scale,
        );
    }
    s.beam(
        Ink::Plank,
        p + vec3(-0.20, 0.06, 0.30) * scale,
        p + vec3(0.20, 0.44, 0.30) * scale,
        0.030 * scale,
    );
}

fn well_and_market(s: &mut Diorama) {
    let c = vec3(-4.75, 0.0, 2.72);
    s.prop(
        "well foundation",
        Shape::Disk,
        Ink::Slate,
        c + Vec3::Y * 0.06,
        vec3(0.85, 0.12, 0.85),
    );
    s.prop(
        "hollow well",
        Shape::Ring,
        Ink::Stone,
        c + Vec3::Y * 0.34,
        vec3(0.69, 0.50, 0.69),
    );
    s.prop(
        "well lip",
        Shape::Ring,
        Ink::StoneLight,
        c + Vec3::Y * 0.62,
        vec3(0.75, 0.12, 0.75),
    );
    s.prop(
        "water in the well",
        Shape::Disk,
        Ink::Water,
        c + Vec3::Y * 0.40,
        vec3(0.52, 0.025, 0.52),
    );
    for x in [-0.81, 0.81] {
        s.beam(
            Ink::Wood,
            c + vec3(x, 0.0, 0.0),
            c + vec3(x, 1.83, 0.0),
            0.065,
        );
    }
    s.beam(
        Ink::Wood,
        c + vec3(-0.88, 1.36, 0.0),
        c + vec3(0.94, 1.36, 0.0),
        0.07,
    );
    s.beam(
        Ink::Rope,
        c + vec3(0.11, 1.36, 0.0),
        c + vec3(0.11, 0.54, 0.0),
        0.017,
    );
    s.prop(
        "bucket",
        Shape::Ring,
        Ink::Wood,
        c + vec3(0.11, 0.61, 0.0),
        vec3(0.15, 0.24, 0.15),
    );
    s.prop(
        "well canopy",
        Shape::Gable,
        Ink::Timber,
        c + vec3(0.0, 1.86, 0.0),
        vec3(2.05, 0.50, 1.52),
    );
    for side in [-1.0, 1.0] {
        s.put(
            "well roof",
            Shape::Box,
            Ink::Slate,
            c + vec3(side * 0.51, 2.13, 0.0),
            vec3(1.15, 0.05, 1.62),
            Quat::from_rotation_z(-side * 0.46),
        );
    }
    let stall = vec3(-6.21, 0.0, 1.24);
    s.block(
        Ink::Wood,
        stall + vec3(0.0, 0.70, 0.0),
        vec3(1.40, 0.12, 0.77),
    );
    for x in [-0.57, 0.57] {
        for z in [-0.27, 0.27] {
            s.block(Ink::Wood, stall + vec3(x, 0.34, z), vec3(0.08, 0.68, 0.08));
        }
    }
    for x in [-0.72, 0.72] {
        s.beam(
            Ink::Wood,
            stall + vec3(x, 0.0, -0.3),
            stall + vec3(x, 1.80, -0.3),
            0.04,
        );
    }
    for i in 0..6 {
        let x = -0.70 + i as f32 * 0.28;
        s.put(
            "striped market awning",
            Shape::Box,
            if i % 2 == 0 { Ink::Cloth } else { Ink::Plaster },
            stall + vec3(x, 1.67, 0.0),
            vec3(0.28, 0.038, 1.10),
            Quat::from_rotation_x(0.18),
        );
        s.prop(
            "awning scallop",
            Shape::Gable,
            if i % 2 == 0 { Ink::Cloth } else { Ink::Plaster },
            stall + vec3(x, 1.51, 0.54),
            vec3(0.27, 0.13, 0.035),
        );
    }
    for i in 0..9 {
        s.prop(
            "market apples",
            Shape::Rock,
            if i % 3 == 0 { Ink::Gold } else { Ink::Red },
            stall
                + vec3(
                    -0.45 + (i % 3) as f32 * 0.18,
                    0.82,
                    (i / 3) as f32 * 0.16 - 0.2,
                ),
            Vec3::splat(0.10),
        );
    }
    for (p, size) in [
        (vec3(-2.63, 0.0, -0.45), 1.0),
        (vec3(-2.16, 0.0, -0.78), 0.80),
        (vec3(-6.11, 0.0, 1.05), 0.9),
    ] {
        barrel(s, p, size);
    }
    crate_prop(s, vec3(-2.25, 0.0, 0.0), 0.9);
    crate_prop(s, vec3(-2.25, 0.45, 0.0), 0.68);
    // An abandoned handcart: open slatted sides, spoked wheels and long handles.
    let w = vec3(3.67, 0.65, 2.76);
    s.block(
        Ink::Timber,
        w + vec3(0.0, 0.47, 0.0),
        vec3(1.15, 0.12, 0.82),
    );
    for side in [-1.0, 1.0] {
        for y in [0.67, 0.86, 1.05] {
            s.block(
                Ink::Plank,
                w + vec3(0.0, y, side * 0.40),
                vec3(1.20, 0.13, 0.055),
            );
        }
        for x in [-0.54, 0.54] {
            s.block(
                Ink::Wood,
                w + vec3(x, 0.76, side * 0.4),
                vec3(0.075, 0.73, 0.075),
            );
        }
        let wheel = w + vec3(0.12, 0.40, side * 0.58);
        s.put(
            "cart wheel rim",
            Shape::Ring,
            Ink::Timber,
            wheel,
            vec3(0.38, 0.09, 0.38),
            Quat::from_rotation_x(FRAC_PI_2),
        );
        for i in 0..6 {
            let a = i as f32 * PI / 3.0;
            s.beam(
                Ink::Plank,
                wheel,
                wheel + vec3(a.cos() * 0.32, a.sin() * 0.32, 0.0),
                0.025,
            );
        }
        s.put(
            "iron hub",
            Shape::Post,
            Ink::Iron,
            wheel,
            vec3(0.085, 0.17, 0.085),
            Quat::from_rotation_x(FRAC_PI_2),
        );
        s.beam(
            Ink::Wood,
            w + vec3(-0.53, 0.47, side * 0.28),
            w + vec3(-1.55, 0.27, side * 0.28),
            0.045,
        );
    }
    barrel(s, w + vec3(0.21, 0.55, 0.0), 0.75);
    s.prop(
        "travel sack",
        Shape::Boulder,
        Ink::Plaster,
        w + vec3(-0.32, 0.77, 0.0),
        vec3(0.27, 0.32, 0.25),
    );
}

fn discoveries(s: &mut Diorama) {
    // Two tankards and a map: someone has just left the table for the crossing.
    let table = vec3(-3.04, 0.0, 1.58);
    s.prop(
        "tavern table top",
        Shape::Disk,
        Ink::Plank,
        table + Vec3::Y * 0.68,
        vec3(0.43, 0.07, 0.43),
    );
    s.prop(
        "table pedestal",
        Shape::Post,
        Ink::Wood,
        table + Vec3::Y * 0.33,
        vec3(0.10, 0.63, 0.10),
    );
    for x in [-0.63, 0.63] {
        s.prop(
            "three-legged stool",
            Shape::Disk,
            Ink::Wood,
            table + vec3(x, 0.35, 0.0),
            vec3(0.20, 0.07, 0.20),
        );
        for i in 0..3 {
            let a = i as f32 * TAU / 3.0;
            s.beam(
                Ink::Wood,
                table + vec3(x + 0.16 * a.cos(), 0.02, 0.16 * a.sin()),
                table + vec3(x + 0.11 * a.cos(), 0.33, 0.11 * a.sin()),
                0.024,
            );
        }
    }
    s.put(
        "unfolded map",
        Shape::Box,
        Ink::Plaster,
        table + vec3(0.03, 0.724, 0.0),
        vec3(0.29, 0.008, 0.22),
        Quat::from_rotation_y(0.30),
    );
    for x in [-0.22, 0.23] {
        s.prop(
            "tankard",
            Shape::Ring,
            Ink::Iron,
            table + vec3(x, 0.79, 0.12),
            vec3(0.065, 0.13, 0.065),
        );
        s.put(
            "tankard handle",
            Shape::Ring,
            Ink::Iron,
            table + vec3(x + 0.075, 0.79, 0.12),
            vec3(0.047, 0.018, 0.047),
            Quat::from_rotation_x(FRAC_PI_2),
        );
    }
    // Noticeboard with pinned quests; its roof echoes the inn without competing.
    let board = vec3(-1.83, 0.0, 2.55);
    for x in [-0.28, 0.28] {
        s.beam(
            Ink::Wood,
            board + vec3(x, 0.0, 0.0),
            board + vec3(x, 1.25, 0.0),
            0.04,
        );
    }
    s.block(Ink::Timber, board + Vec3::Y * 0.96, vec3(0.78, 0.57, 0.085));
    for (x, y, z, angle) in [
        (-0.19, 1.02, 0.051, -0.11),
        (0.08, 1.07, 0.057, 0.12),
        (0.20, 0.83, 0.06, -0.10),
    ] {
        s.put(
            "pinned parchment",
            Shape::Box,
            Ink::Plaster,
            board + vec3(x, y, z),
            vec3(0.21, 0.27, 0.01),
            Quat::from_rotation_z(angle),
        );
        s.prop(
            "notice pin",
            Shape::Rock,
            Ink::Gold,
            board + vec3(x, y + 0.10, z + 0.012),
            Vec3::splat(0.013),
        );
    }
    s.prop(
        "noticeboard cap",
        Shape::Gable,
        Ink::Roof,
        board + Vec3::Y * 1.29,
        vec3(0.99, 0.22, 0.36),
    );
    // A half-open cache in the ruins rewards looking behind the gate's right pier.
    let chest = vec3(5.52, 0.65, -0.33);
    s.block(Ink::Wood, chest + Vec3::Y * 0.19, vec3(0.66, 0.37, 0.43));
    s.block(Ink::Iron, chest + Vec3::Y * 0.38, vec3(0.59, 0.025, 0.36));
    s.put(
        "raised chest lid",
        Shape::Box,
        Ink::Plank,
        chest + vec3(0.0, 0.57, -0.20),
        vec3(0.69, 0.075, 0.46),
        Quat::from_rotation_x(-0.94),
    );
    for x in [-0.25, 0.25] {
        s.block(
            Ink::Gold,
            chest + vec3(x, 0.20, 0.23),
            vec3(0.045, 0.37, 0.025),
        );
    }
    s.block(
        Ink::Gold,
        chest + vec3(0.0, 0.32, 0.24),
        vec3(0.07, 0.10, 0.03),
    );
    for i in 0..7 {
        s.prop(
            "coins in the cache",
            Shape::Disk,
            Ink::Gold,
            chest
                + vec3(
                    -0.19 + (i % 3) as f32 * 0.16,
                    0.41 + (i / 3) as f32 * 0.02,
                    -0.06 + (i / 3) as f32 * 0.09,
                ),
            vec3(0.065, 0.018, 0.065),
        );
    }
    // The inn's black cat sits on a barrel, watching the red-cloaked traveler.
    let cat = vec3(-2.63, 0.62, -0.45);
    s.prop(
        "inn cat",
        Shape::Rock,
        Ink::Iron,
        cat + vec3(0.0, 0.12, 0.0),
        vec3(0.14, 0.18, 0.16),
    );
    s.prop(
        "cat head",
        Shape::Rock,
        Ink::Iron,
        cat + vec3(0.0, 0.29, 0.06),
        vec3(0.115, 0.11, 0.10),
    );
    for x in [-0.075, 0.075] {
        s.prop(
            "cat ears",
            Shape::Cone,
            Ink::Iron,
            cat + vec3(x, 0.36, 0.04),
            vec3(0.05, 0.09, 0.045),
        );
        s.prop(
            "cat eyes",
            Shape::Rock,
            Ink::Gold,
            cat + vec3(x * 0.6, 0.30, 0.147),
            vec3(0.016, 0.013, 0.008),
        );
    }
    s.beam(
        Ink::Iron,
        cat + vec3(0.08, 0.06, -0.07),
        cat + vec3(0.21, 0.10, -0.17),
        0.025,
    );
    s.beam(
        Ink::Iron,
        cat + vec3(0.21, 0.10, -0.17),
        cat + vec3(0.25, 0.23, -0.12),
        0.020,
    );
}

fn traveler(s: &mut Diorama, p: Vec3, scale: f32, yaw: f32, cloak_ink: Ink) {
    let rot = Quat::from_rotation_y(yaw);
    let at = |v: Vec3| p + rot * (v * scale);
    let mut part = |name, shape, ink, v, size| {
        s.put(name, shape, ink, at(v), size * scale, rot);
    };
    part(
        "traveler boots",
        Shape::Box,
        Ink::Timber,
        vec3(-0.12, 0.11, 0.06),
        vec3(0.16, 0.22, 0.30),
    );
    part(
        "traveler boots",
        Shape::Box,
        Ink::Timber,
        vec3(0.12, 0.12, -0.10),
        vec3(0.16, 0.24, 0.30),
    );
    part(
        "traveler tunic",
        Shape::Post,
        Ink::Cloth,
        vec3(0.0, 0.54, 0.0),
        vec3(0.24, 0.68, 0.19),
    );
    part(
        "travel-worn cloak",
        Shape::Cloak,
        cloak_ink,
        vec3(0.0, 0.62, -0.06),
        vec3(0.72, 0.85, 1.0),
    );
    part(
        "hood",
        Shape::Boulder,
        cloak_ink,
        vec3(0.0, 1.13, 0.0),
        vec3(0.25, 0.28, 0.235),
    );
    part(
        "face within hood",
        Shape::Rock,
        Ink::Skin,
        vec3(0.0, 1.115, 0.18),
        vec3(0.14, 0.16, 0.065),
    );
    part(
        "leather backpack",
        Shape::Box,
        Ink::Wood,
        vec3(0.0, 0.77, -0.28),
        vec3(0.37, 0.40, 0.25),
    );
    part(
        "bedroll",
        Shape::Post,
        Ink::Plaster,
        vec3(0.0, 1.02, -0.30),
        vec3(0.17, 0.35, 0.17),
    );
    s.beam(
        Ink::Timber,
        at(vec3(0.34, 0.05, 0.19)),
        at(vec3(0.34, 1.61, 0.19)),
        0.025 * scale,
    );
    s.prop(
        "staff moonstone",
        Shape::Crystal,
        Ink::Magic,
        at(vec3(0.34, 1.67, 0.19)),
        Vec3::splat(0.19 * scale),
    );
    s.beam(
        Ink::Cloth,
        at(vec3(0.17, 0.85, 0.0)),
        at(vec3(0.34, 0.68, 0.19)),
        0.085 * scale,
    );
    s.prop(
        "hand on staff",
        Shape::Rock,
        Ink::Skin,
        at(vec3(0.34, 0.68, 0.19)),
        Vec3::splat(0.065 * scale),
    );
    s.beam(
        Ink::Gold,
        at(vec3(-0.10, 0.93, 0.18)),
        at(vec3(0.10, 0.93, 0.18)),
        0.015 * scale,
    );
}

fn camp_and_magic(s: &mut Diorama, rng: &mut fastrand::Rng) {
    let c = vec3(3.13, 0.65, 0.18);
    for i in 0..9 {
        let a = i as f32 * TAU / 9.0;
        s.prop(
            "campfire stones",
            Shape::Rock,
            Ink::Slate,
            c + vec3(0.43 * a.cos(), 0.09, 0.43 * a.sin()),
            vec3(0.15, 0.13, 0.15),
        );
    }
    for angle in [0.0_f32, 1.2, 2.4] {
        let d = vec3(angle.cos(), 0.0, angle.sin()) * 0.38;
        s.beam(
            Ink::Timber,
            c - d + Vec3::Y * 0.11,
            c + d + Vec3::Y * 0.11,
            0.075,
        );
    }
    for i in 0..4 {
        let p = c + vec3(
            (i as f32 * 1.8).cos() * 0.13,
            0.27,
            (i as f32 * 1.8).sin() * 0.13,
        );
        s.put(
            "low-poly flame",
            Shape::Crystal,
            if i % 2 == 0 { Ink::Amber } else { Ink::Gold },
            p,
            vec3(0.23, 0.45, 0.23),
            Quat::from_rotation_z(i as f32 * 0.13),
        );
    }
    s.light(c + Vec3::Y * 0.5, [1.0, 0.32, 0.08], 170.0);
    s.block(
        Ink::Cloth,
        c + vec3(-0.15, 0.045, -0.98),
        vec3(0.58, 0.06, 1.02),
    );
    s.prop(
        "rolled blanket",
        Shape::Post,
        Ink::Red,
        c + vec3(-0.15, 0.12, -1.43),
        vec3(0.14, 0.58, 0.14),
    );
    s.prop(
        "camp kettle",
        Shape::Post,
        Ink::Iron,
        c + vec3(0.70, 0.16, 0.07),
        vec3(0.18, 0.25, 0.18),
    );
    for side in [-1.0, 1.0] {
        s.beam(
            Ink::Wood,
            c + vec3(side * 0.58, 0.03, 0.0),
            c + vec3(0.0, 0.97, 0.0),
            0.025,
        );
    }
    for i in 0..16 {
        let origin = if i < 7 {
            c + vec3(0.0, 0.6, 0.0)
        } else {
            vec3(
                1.0 + rng.f32() * 4.0,
                0.6 + rng.f32() * 1.2,
                -1.3 + rng.f32() * 3.8,
            )
        };
        let id = s.prop(
            "wandering ember / firefly",
            Shape::Rock,
            if i < 7 { Ink::Amber } else { Ink::Magic },
            origin,
            Vec3::splat(if i < 7 { 0.025 } else { 0.018 }),
        );
        s.parts[id].motion = Some(Motion::Spark {
            origin,
            phase: i as f32 * 0.61,
        });
    }
    // Crystal seams below the grass hint that the floating rock has a source of magic.
    for i in 0..8 {
        let p = vec3(
            1.50 + i as f32 * 0.50,
            -1.50 + 0.20 * (i as f32).sin(),
            2.10 + i as f32 * 0.11,
        );
        s.put(
            "crystal seam in the cliff",
            Shape::Crystal,
            if i % 3 == 0 { Ink::Magic } else { Ink::Water },
            p,
            vec3(0.45, 0.58 + rng.f32() * 0.65, 0.45),
            Quat::from_euler(EulerRot::XYZ, 0.3, 0.1, i as f32 * 0.16 - 0.5),
        );
    }
    traveler(
        s,
        vec3(0.60, bridge_height(2.35 / 4.5) + 0.065, 1.10),
        0.83,
        FRAC_PI_2,
        Ink::Red,
    );
    traveler(s, vec3(-3.8, 0.30, 0.58), 0.66, -0.5, Ink::Cloth);
}

fn build_scene() -> SMeshResult<Diorama> {
    let mut scene = Diorama {
        models: models()?,
        parts: Vec::new(),
        lamps: Vec::new(),
    };
    let mut rng = fastrand::Rng::with_seed(0x1A47_E2FA_11);
    landscape(&mut scene, &mut rng);
    inn(&mut scene, &mut rng);
    moon_gate(&mut scene, &mut rng);
    watchtower(&mut scene);
    bridge(&mut scene, &mut rng);
    vegetation(&mut scene, &mut rng);
    well_and_market(&mut scene);
    discoveries(&mut scene);
    camp_and_magic(&mut scene, &mut rng);
    Ok(scene)
}

fn inspect_scene(scene: &Diorama) -> Result<(), Box<dyn Error>> {
    let mut polygons = 0;
    for model in &scene.models {
        let report = model.mesh.describe();
        if !report.is_closed || !report.is_manifold || report.connected_components != 1 {
            return Err(format!("{}: {report}", model.name).into());
        }
        let validation = model.mesh.validate();
        for issue in validation.issues {
            if matches!(
                issue,
                MeshIssue::BrokenConnectivity { .. }
                    | MeshIssue::DegenerateFace { .. }
                    | MeshIssue::ZeroLengthEdge { .. }
                    | MeshIssue::IsolatedVertex { .. }
            ) {
                return Err(format!("{}: {issue}", model.name).into());
            }
        }
        println!(
            "  {:27} {:4} faces / closed manifold",
            model.name, report.face_count
        );
    }
    for part in &scene.parts {
        if !part.transform.translation.is_finite()
            || !part.transform.scale.is_finite()
            || part.transform.scale.min_element() <= 0.0
        {
            return Err(format!("Invalid transform: {}", part.name).into());
        }
        polygons += scene.models[part.shape as usize].mesh.faces().len();
    }
    println!("Lanternfall: {} instanced SMesh models, {} placed pieces, {polygons} source polygons, {} local lights.",scene.models.len(),scene.parts.len(),scene.lamps.len());
    Ok(())
}

fn material(ink: usize) -> StandardMaterial {
    let [r, g, b] = COLORS[ink];
    let emissive = if ink == Ink::Magic as usize {
        LinearRgba::rgb(0.15, 2.5, 3.0)
    } else if ink == Ink::Amber as usize {
        LinearRgba::rgb(3.2, 1.05, 0.16)
    } else if ink == Ink::Foam as usize {
        LinearRgba::rgb(0.06, 0.20, 0.18)
    } else if ink == Ink::Portal as usize {
        LinearRgba::rgb(0.005, 0.026, 0.052)
    } else {
        LinearRgba::BLACK
    };
    StandardMaterial {
        base_color: Color::srgb(r, g, b),
        emissive,
        perceptual_roughness: if ink == Ink::Water as usize {
            0.28
        } else {
            0.86
        },
        metallic: if ink == Ink::Iron as usize || ink == Ink::Gold as usize {
            0.45
        } else {
            0.0
        },
        ..default()
    }
}

const VIEWS: [(f32, f32, f32, Vec3); 3] = [
    (0.58, 0.49, 1.08, vec3(-0.3, 1.2, 0.0)),
    (-0.48, 0.43, 1.00, vec3(-0.3, 1.4, -0.1)),
    (0.25, 0.62, 0.70, vec3(3.0, 2.0, -0.6)),
];

fn setup(
    mut commands: Commands,
    scene: Res<Diorama>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh_handles: Vec<_> = scene
        .models
        .iter()
        .map(|m| meshes.add(Mesh::from(m.mesh.clone())))
        .collect();
    let mat_handles: Vec<_> = (0..COLORS.len())
        .map(|i| materials.add(material(i)))
        .collect();
    for part in &scene.parts {
        let mut entity = commands.spawn((
            Name::new(part.name),
            Mesh3d(mesh_handles[part.shape as usize].clone()),
            MeshMaterial3d(mat_handles[part.ink as usize].clone()),
            part.transform,
        ));
        if let Some(motion) = part.motion {
            entity.insert(motion);
        }
    }
    for lamp in &scene.lamps {
        commands.spawn((
            PointLight {
                color: lamp.color,
                intensity: lamp.intensity,
                range: 7.0,
                radius: lamp.radius,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(lamp.position),
        ));
    }
    for (p, illuminance, color, shadows) in [
        (
            vec3(-6.0, 10.0, 7.0),
            4400.0,
            Color::srgb(1.0, 0.81, 0.62),
            true,
        ),
        (
            vec3(5.0, 6.0, -8.0),
            2700.0,
            Color::srgb(0.49, 0.72, 1.0),
            false,
        ),
    ] {
        commands.spawn((
            DirectionalLight {
                illuminance,
                color,
                shadow_maps_enabled: shadows,
                ..default()
            },
            Transform::from_translation(p).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }
    let (yaw, pitch, radius, focus) = VIEWS[0];
    commands.spawn((
        Camera3d::default(),
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 15.6,
            },
            near: 0.1,
            far: 160.0,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(15.0, 12.0, 20.0).looking_at(focus, Vec3::Y),
        PanOrbitCamera {
            yaw: Some(yaw),
            pitch: Some(pitch),
            radius: Some(radius),
            focus,
            zoom_lower_limit: 0.30,
            zoom_upper_limit: Some(2.5),
            ..default()
        },
        Bloom {
            intensity: 0.11,
            ..Bloom::NATURAL
        },
        Msaa::Off,
        // Spatial AA keeps this orthographic view stationary. TAA's per-frame
        // projection jitter can read as camera shake, especially on fine joinery.
        bevy::anti_alias::smaa::Smaa::default(),
        bevy::pbr::ScreenSpaceAmbientOcclusion::default(),
    ));
    commands.spawn((
        Text::new("L A N T E R N F A L L\nC R O S S I N G"),
        TextFont::from_font_size(30.0),
        TextColor(Color::srgb(0.90, 0.79, 0.57)),
        Node {
            position_type: PositionType::Absolute,
            left: px(30),
            bottom: px(55),
            ..default()
        },
    ));
    commands.spawn((
        Text::new("THE LAST WARM LIGHT BEFORE THE MOON GATE"),
        TextFont::from_font_size(12.0),
        TextColor(Color::srgb(0.59, 0.68, 0.70)),
        Node {
            position_type: PositionType::Absolute,
            left: px(32),
            bottom: px(31),
            ..default()
        },
    ));
    commands.spawn((
        Text::new("DRAG  orbit     SCROLL  zoom     1 / 2 / 3  views     SPACE  pause"),
        TextFont::from_font_size(13.0),
        TextColor(Color::srgb(0.59, 0.68, 0.70)),
        Node {
            position_type: PositionType::Absolute,
            right: px(25),
            top: px(22),
            ..default()
        },
    ));
}

fn animate(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut clock: ResMut<Clock>,
    mut moving: Query<(&Motion, &mut Transform)>,
    mut camera: Query<&mut PanOrbitCamera>,
) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if !clock.paused {
        clock.time += time.delta_secs();
    }
    for (key, view) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
    ] {
        if keys.just_pressed(key) {
            let (yaw, pitch, radius, focus) = VIEWS[view];
            for mut orbit in &mut camera {
                orbit.target_yaw = yaw;
                orbit.target_pitch = pitch;
                orbit.target_radius = radius;
                orbit.target_focus = focus;
            }
        }
    }
    let t = clock.time;
    for (motion, mut transform) in &mut moving {
        match *motion {
            Motion::Hover { origin, phase } => {
                transform.translation = origin + Vec3::Y * (t * 1.3 + phase).sin() * 0.09;
                transform.rotation = Quat::from_rotation_y(t * 0.28);
            }
            Motion::Smoke {
                origin,
                phase,
                size,
            } => {
                let u = (t * 0.13 + phase).fract();
                transform.translation = origin + vec3(-u * 0.75, u * 1.55, u * 0.15);
                transform.scale = Vec3::splat(size * (0.60 + u * 1.6) * (1.0 - u).min(0.4) * 2.5);
            }
            Motion::Spark { origin, phase } => {
                transform.translation = origin
                    + vec3(
                        (t * 0.45 + phase).sin() * 0.17,
                        (t * 0.8 + phase).sin() * 0.22,
                        (t * 0.6 + phase).cos() * 0.16,
                    );
            }
            Motion::Fall { origin, phase } => {
                transform.translation = origin - Vec3::Y * (t * 0.55 + phase).fract() * 3.1;
            }
        }
    }
}

// Optional visual inspection only: captures the actual Bevy render, not exported
// scene geometry. Unset during normal use. Example: SMESH_PREVIEW_DIR=/tmp/lanternfall
#[derive(Resource)]
struct VisualCheck {
    dir: Option<PathBuf>,
    frame: u32,
}
fn visual_check(
    mut commands: Commands,
    mut check: ResMut<VisualCheck>,
    mut camera: Query<&mut PanOrbitCamera>,
    mut exit: MessageWriter<AppExit>,
) {
    if check.dir.is_none() {
        return;
    }
    check.frame += 1;
    for (i, frame) in [180, 330, 480].into_iter().enumerate() {
        if check.frame == frame - 50 {
            let (yaw, pitch, radius, focus) = VIEWS[i];
            for mut orbit in &mut camera {
                orbit.target_yaw = yaw;
                orbit.target_pitch = pitch;
                orbit.target_radius = radius;
                orbit.target_focus = focus;
            }
        }
        if check.frame == frame {
            if let Some(dir) = &check.dir {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(dir.join(format!("view-{}.png", i + 1))));
            }
        }
    }
    if check.frame == 550 {
        exit.write(AppExit::Success);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let scene = build_scene()?;
    inspect_scene(&scene)?;
    let dir = std::env::var_os("SMESH_PREVIEW_DIR").map(PathBuf::from);
    if let Some(path) = &dir {
        std::fs::create_dir_all(path)?;
    }
    App::new()
        .insert_resource(scene)
        .insert_resource(Clock::default())
        .insert_resource(VisualCheck { dir, frame: 0 })
        .insert_resource(ClearColor(Color::srgb(0.035, 0.055, 0.085)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.64, 0.76, 0.88),
            brightness: 230.0,
            ..default()
        })
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Lanternfall Crossing | SMesh low-poly RPG scene".into(),
                    resolution: (1500, 1050).into(),
                    ..default()
                }),
                ..default()
            }),
            EguiPlugin::default(),
            PanOrbitCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (animate, visual_check).chain())
        .run();
    Ok(())
}
