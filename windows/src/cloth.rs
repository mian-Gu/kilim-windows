//! Port of the original Verlet / position-based cloth and spiral roll model.
use crate::collision::SurfaceCollision;
use std::f32::consts::PI;

pub const NX: usize = 66;
pub const NY: usize = 45;
pub const N: usize = NX * NY;
const K: f32 = 0.0009;
type V3 = [f32; 3];

#[derive(Clone, Copy)]
struct Constraint {
    a: usize,
    b: usize,
    rest: f32,
    strength: f32,
    color: usize,
}
#[derive(Clone, Copy)]
struct Pin {
    index: usize,
    offset: [f32; 2],
    /// How strongly this vertex follows the grab lift: 1 at the grab centre, 0 at the rim.
    weight: f32,
    height_offset: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Vertex {
    pub position: V3,
    pub uv: [f32; 2],
    pub normal: V3,
    pub ao: f32,
}

pub struct Cloth {
    pub position: Vec<V3>,
    old: Vec<V3>,
    collision_start: Vec<V3>,
    collision_candidate: Vec<V3>,
    surface_collision: SurfaceCollision,
    screen: Vec<[f32; 2]>,
    floor: Vec<f32>,
    pinned: Vec<bool>,
    /// Rest layout of a free-hanging sheet. Pulling it back towards these positions is what makes
    /// the cloth slide home again after it has been dragged away.
    rest: Vec<V3>,
    /// Strength of that pull per step; zero for the ordinary rug, which simply lies where dropped.
    elastic: f32,
    grab_origin: [f32; 2],
    grab_lift: f32,
    grab_base_height: f32,
    /// Full-screen sheet mode: the desktop icon bumps are ignored so the cloth stays flat.
    flat_floor: bool,
    /// Flatter lighting for a desktop-sized sheet: folds keep a high ambient floor instead of
    /// turning into grey patches when their normals face away from the light.
    pub soft_light: bool,
    constraints: Vec<Constraint>,
    pins: Vec<Pin>,
    pointer: [f32; 2],
    pub bumps: Vec<[f32; 2]>,
    pub view: [f32; 2],
    point_scale: f32,
    pub width: f32,
    pub iterations: usize,
    pub self_thickness: f32,
    pub self_strength: f32,
    height: f32,
    pub settled: u32,
    pub max_speed: f32,
    pub slide_level: f32,
    pub impact_sum: f32,
    slide_accum: f32,
    pub rolling: bool,
    roll_progress: f32,
    roll_goal: f32,
    roll_blend: f32,
    roll_dir: f32,
    roll_center: [f32; 2],
    roll_ex: [f32; 2],
    roll_ey: [f32; 2],
    roll_start: Vec<V3>,
    head: Vec<i32>,
    next: Vec<i32>,
}

impl Cloth {
    pub fn new(view: [f32; 2]) -> Self {
        let mut result = Self {
            position: vec![[0.; 3]; N],
            old: vec![[0.; 3]; N],
            collision_start: vec![[0.; 3]; N],
            collision_candidate: vec![[0.; 3]; N],
            surface_collision: SurfaceCollision::new(NX, NY),
            screen: vec![[0.; 2]; N],
            floor: vec![0.; N],
            pinned: vec![false; N],
            rest: vec![[0.; 3]; N],
            elastic: 0.,
            grab_origin: [0.; 2],
            grab_lift: 0.,
            grab_base_height: 0.,
            flat_floor: false,
            soft_light: false,
            constraints: Vec::new(),
            pins: Vec::new(),
            pointer: [0.; 2],
            bumps: Vec::new(),
            view,
            point_scale: 1.,
            width: 600.,
            iterations: 22,
            self_thickness: 10.,
            self_strength: 0.7,
            height: 400.,
            settled: 0,
            max_speed: 0.,
            slide_level: 0.,
            impact_sum: 0.,
            slide_accum: 0.,
            rolling: false,
            roll_progress: 0.,
            roll_goal: 0.,
            roll_blend: 1.,
            roll_dir: 1.,
            roll_center: [0.; 2],
            roll_ex: [1., 0.],
            roll_ey: [0., 1.],
            roll_start: Vec::new(),
            head: vec![-1; 4096],
            next: vec![-1; N],
        };
        result.reset(1.);
        result
    }

    pub fn reset(&mut self, size: f32) {
        self.reset_in_rect(size, [0., 0.], self.view);
    }

    pub fn reset_in_rect(&mut self, size: f32, origin: [f32; 2], screen_size: [f32; 2]) {
        let width = (((screen_size[0] / self.point_scale) * 0.36).clamp(420., 760.)
            * size
            * self.point_scale)
            .min(screen_size[0] * 0.92);
        self.reset_pose(
            [
                origin[0] + screen_size[0] * 0.4,
                origin[1] + screen_size[1] * 0.52,
            ],
            width,
            130.,
            -0.12,
        );
    }

    pub fn reset_pose(&mut self, center: [f32; 2], width: f32, drop: f32, angle: f32) {
        self.reset_flat(center, width, width * 1032. / 1536., drop, angle);
    }

    /// Full-screen sheet: covers `view` plus `bleed` on every side, so the border stays outside
    /// the visible area. The sheet is free to slide, and a spring towards the rest layout pulls it
    /// back over the desktop once the grab is released.
    pub fn reset_fullscreen(&mut self, view: [f32; 2], bleed: f32) {
        self.view = view;
        self.reset_flat(
            [view[0] * 0.5, view[1] * 0.5],
            view[0] + bleed * 2.,
            view[1] + bleed * 2.,
            0.,
            0.,
        );
        self.flat_floor = true;
        self.soft_light = true;
        self.elastic = 0.10;
        self.self_thickness = 2. * self.point_scale;
        for k in 0..N {
            self.position[k][2] = 0.;
            self.old[k][2] = 0.;
            self.rest[k] = self.position[k];
        }
        self.refresh_projection();
    }

    fn reset_flat(&mut self, center: [f32; 2], width: f32, height: f32, drop: f32, angle: f32) {
        self.width = width;
        self.height = height;
        self.flat_floor = false;
        self.soft_light = false;
        self.elastic = 0.;
        self.self_thickness = 10. * self.point_scale;
        let mut random = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            & 0xffff;
        for j in 0..NY {
            for i in 0..NX {
                let x = (i as f32 / (NX - 1) as f32 - 0.5) * self.width;
                let y = (j as f32 / (NY - 1) as f32 - 0.5) * self.height;
                let p = [
                    center[0] + x * angle.cos() - y * angle.sin(),
                    center[1] + x * angle.sin() + y * angle.cos(),
                    (drop + random_unit(&mut random) * 0.6) * self.point_scale,
                ];
                let k = j * NX + i;
                self.position[k] = p;
                self.old[k] = p;
                self.pinned[k] = false;
                self.floor[k] = 0.;
            }
        }
        self.pins.clear();
        self.rolling = false;
        self.roll_progress = 0.;
        self.roll_goal = 0.;
        self.max_speed = 0.;
        self.slide_level = 0.;
        self.impact_sum = 0.;
        self.settled = 0;
        self.build_constraints();
        self.refresh_projection();
    }

    pub fn translate(&mut self, delta: [f32; 2]) {
        for i in 0..N {
            for (axis, value) in delta.iter().enumerate() {
                self.position[i][axis] += value;
                self.old[i][axis] += value;
            }
        }
        for point in &mut self.rest {
            for (axis, value) in delta.iter().enumerate() {
                point[axis] += value;
            }
        }
        for (axis, value) in delta.iter().enumerate() {
            self.roll_center[axis] += value;
        }
        for p in &mut self.roll_start {
            p[0] += delta[0];
            p[1] += delta[1];
        }
        self.refresh_projection();
    }

    pub fn scale(&mut self, factor: f32) {
        if self.rolling || self.grabbing() || !factor.is_finite() || factor <= 0. || self.elastic > 0.
        {
            return;
        }
        let center: [f32; 2] = std::array::from_fn(|axis| {
            self.position.iter().map(|p| p[axis]).sum::<f32>() / N as f32
        });
        for i in 0..N {
            for (axis, c) in center.iter().enumerate() {
                self.position[i][axis] = c + (self.position[i][axis] - c) * factor;
                self.old[i][axis] = c + (self.old[i][axis] - c) * factor;
            }
        }
        self.width *= factor;
        self.height *= factor;
        self.build_constraints();
        self.settled = 0;
        self.refresh_projection();
    }
    pub fn point_scale(&self) -> f32 {
        self.point_scale
    }
    pub fn perspective(&self) -> f32 {
        K / self.point_scale
    }
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }
    pub fn set_point_scale(&mut self, scale: f32) {
        assert!(scale.is_finite() && scale > 0.);
        if self.point_scale == scale {
            return;
        }
        let factor = scale / self.point_scale;
        for i in 0..N {
            for axis in 0..3 {
                self.position[i][axis] *= factor;
                self.old[i][axis] *= factor;
                self.rest[i][axis] *= factor;
            }
            self.floor[i] *= factor;
        }
        for pin in &mut self.pins {
            pin.offset[0] *= factor;
            pin.offset[1] *= factor;
            pin.height_offset *= factor;
        }
        self.pointer = self.pointer.map(|value| value * factor);
        self.grab_origin = self.grab_origin.map(|value| value * factor);
        self.grab_lift *= factor;
        self.grab_base_height *= factor;
        self.roll_center = self.roll_center.map(|value| value * factor);
        for point in &mut self.roll_start {
            for value in point {
                *value *= factor;
            }
        }
        self.width *= factor;
        self.height *= factor;
        self.self_thickness *= factor;
        self.point_scale = scale;
        self.build_constraints();
        self.refresh_projection();
    }

    fn build_constraints(&mut self) {
        self.constraints.clear();
        let dx = self.width / (NX - 1) as f32;
        let dy = self.height / (NY - 1) as f32;
        for j in 0..NY {
            for i in 0..NX {
                for (i1, j1, i2, j2, strength, color) in [
                    (i, j, i + 1, j, 1., i & 1),
                    (i, j, i, j + 1, 1., 2 + (j & 1)),
                    (i, j, i + 1, j + 1, 0.55, 4 + (i & 1) + 2 * (j & 1)),
                    (i + 1, j, i, j + 1, 0.55, 8 + (i & 1) + 2 * (j & 1)),
                    (i, j, i + 2, j, 0.14, 12 + (i & 3)),
                    (i, j, i, j + 2, 0.14, 16 + (j & 3)),
                ] {
                    if i1 < NX && i2 < NX && j1 < NY && j2 < NY {
                        self.constraints.push(Constraint {
                            a: j1 * NX + i1,
                            b: j2 * NX + i2,
                            rest: ((i2 as f32 - i1 as f32) * dx)
                                .hypot((j2 as f32 - j1 as f32) * dy),
                            strength,
                            color,
                        });
                    }
                }
            }
        }
        self.constraints.sort_by_key(|c| c.color);
    }

    pub fn height_at(&self, x: f32, y: f32) -> f32 {
        self.bumps
            .iter()
            .map(|b| {
                let d = (x - b[0]).hypot(y - b[1]) / self.point_scale;
                if d >= 48. {
                    0.
                } else {
                    let u = ((d - 14.) / 34.).clamp(0., 1.);
                    20. * (1. - u * u * (3. - 2. * u))
                }
            })
            .sum::<f32>()
            .min(110.)
            * self.point_scale
    }

    pub fn step(&mut self) {
        if self.rolling {
            self.step_roll();
            self.refresh_projection();
            return;
        }
        self.max_speed = 0.;
        self.slide_accum = 0.;
        self.impact_sum = 0.;
        let unit = self.point_scale;
        if self.flat_floor {
            self.collision_start.copy_from_slice(&self.position);
        }
        for pin in &self.pins {
            let k = pin.index;
            let target = if self.flat_floor {
                self.inverse_project(self.pointer, self.position[k][2])
            } else {
                self.pointer
            };
            let mut mx = (target[0] + pin.offset[0] - self.position[k][0]) * 0.7;
            let mut my = (target[1] + pin.offset[1] - self.position[k][1]) * 0.7;
            let length = mx.hypot(my);
            if length > 22. * unit {
                mx *= 22. * unit / length;
                my *= 22. * unit / length;
            }
            self.position[k][0] += mx;
            self.position[k][1] += my;
            let lift = if self.flat_floor {
                self.grab_lift + pin.height_offset
            } else {
                self.grab_lift * pin.weight
            };
            self.position[k][2] += (lift - self.position[k][2]) * 0.2;
            self.old[k] = self.position[k];
        }
        let mut max_speed: f32 = 0.;
        for i in 0..N {
            if self.pinned[i] {
                continue;
            }
            let p = self.position[i];
            let mut velocity = [
                (p[0] - self.old[i][0]) * 0.965,
                (p[1] - self.old[i][1]) * 0.965,
                (p[2] - self.old[i][2]) * 0.9,
            ];
            if self.elastic > 0. {
                // Free hanging sheet: pull it back towards the layout that covers the desktop.
                velocity[0] += (self.rest[i][0] - p[0]) * self.elastic;
                velocity[1] += (self.rest[i][1] - p[1]) * self.elastic;
            }
            let speed = velocity[0].hypot(velocity[1]);
            if p[2] - self.floor[i] < 0.8 * unit {
                if speed > 0.25 * unit {
                    self.slide_accum += speed / unit;
                }
                let factor = if speed <= 0.85 * unit {
                    0.
                } else {
                    (speed - 0.85 * unit) / speed * 0.93
                };
                velocity[0] *= factor;
                velocity[1] *= factor;
            }
            let speed = velocity[0].hypot(velocity[1]);
            if speed > 14. * unit {
                velocity[0] *= 14. * unit / speed;
                velocity[1] *= 14. * unit / speed;
            }
            max_speed = max_speed.max(speed.min(14. * unit)).max(velocity[2].abs());
            self.old[i] = p;
            for (axis, v) in velocity.iter().enumerate() {
                self.position[i][axis] += v;
            }
            self.position[i][2] -= 0.3 * unit;
        }
        self.compute_floor();
        self.clamp_floor();
        self.self_collide();
        for iteration in 0..self.iterations {
            let count = self.constraints.len();
            for k in 0..count {
                let c = self.constraints[if iteration & 1 == 0 { k } else { count - 1 - k }];
                let delta = sub(self.position[c.b], self.position[c.a]);
                let length = (dot(delta, delta) + 1e-12).sqrt();
                let wa = if self.pinned[c.a] { 0. } else { 1. };
                let wb = if self.pinned[c.b] { 0. } else { 1. };
                if wa + wb == 0. {
                    continue;
                }
                let factor = c.strength * (1. - c.rest / length) / (wa + wb);
                for (axis, d) in delta.iter().enumerate() {
                    self.position[c.a][axis] += d * factor * wa;
                    self.position[c.b][axis] -= d * factor * wb;
                }
            }
            if iteration % 4 == 3 {
                // Interleave the layer separation with the constraint passes: a single pass before
                // and after the loop lets a sharply folded sheet walk through itself at coarse
                // grid spacing, which shows up as triangles poking through the surface.
                self.clamp_floor();
                self.self_collide();
            }
        }
        self.self_collide();
        self.compute_floor();
        self.clamp_floor();
        for p in &mut self.position {
            p[0] = p[0].clamp(-200. * unit, self.view[0] + 200. * unit);
            p[1] = p[1].clamp(-200. * unit, self.view[1] + 200. * unit);
        }
        self.collide_surface();
        self.settled = if max_speed < 0.15 * unit && self.pins.is_empty() {
            self.settled.saturating_add(1)
        } else {
            0
        };
        self.max_speed = max_speed / unit;
        self.slide_level = self.slide_accum / N as f32;
        self.refresh_projection();
    }

    fn compute_floor(&mut self) {
        if self.flat_floor {
            self.floor.fill(0.);
            return;
        }
        for i in 0..N {
            self.floor[i] = self.height_at(self.position[i][0], self.position[i][1]);
        }
    }
    fn clamp_floor(&mut self) {
        for i in 0..N {
            if self.position[i][2] < self.floor[i] {
                let velocity = self.old[i][2] - self.position[i][2];
                let falling = velocity > 0.6 * self.point_scale;
                if falling {
                    self.impact_sum += velocity / self.point_scale - 0.6;
                }
                self.position[i][2] = self.floor[i];
                if falling || self.old[i][2] < self.floor[i] {
                    self.old[i][2] = self.floor[i];
                }
            }
        }
    }
    fn hash(x: i32, y: i32, z: i32) -> usize {
        ((x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ z.wrapping_mul(83492791)) & 4095)
            as usize
    }
    fn self_collide(&mut self) {
        if self.flat_floor {
            return;
        }
        // Bucket by the larger of the collision thickness and the vertex spacing: a bucket smaller
        // than one grid cell hides neighbouring layers from the 3x3x3 search, which is how a folded
        // sheet ends up passing through itself.
        let cell = (self.width / (NX - 1) as f32)
            .max(self.height / (NY - 1) as f32)
            .max(self.self_thickness)
            .max(1.);
        self.head.fill(-1);
        for i in 0..N {
            let p = self.position[i];
            let hash = Self::hash(
                (p[0] / cell).floor() as i32,
                (p[1] / cell).floor() as i32,
                (p[2] / cell).floor() as i32,
            );
            self.next[i] = self.head[hash];
            self.head[hash] = i as i32;
        }
        for i in 0..N {
            let p = self.position[i];
            let key = p.map(|v| (v / cell).floor() as i32);
            for dz in -1..=1 {
                for dy_offset in -1..=1 {
                    for dx_offset in -1..=1 {
                        let mut next = self.head
                            [Self::hash(key[0] + dx_offset, key[1] + dy_offset, key[2] + dz)];
                        while next >= 0 {
                            let j = next as usize;
                            next = self.next[j];
                            if j <= i {
                                continue;
                            }
                            // The immediate ring is exempt: those two vertices are already held at
                            // their rest spacing by the length springs, so a collision term there
                            // just fights them. The desktop sheet also exempts only that ring — its
                            // fold's two halves sit two cells apart, and the rug's ±2 window silently
                            // excused the crease, letting one layer push through the other.
                            if (i % NX).abs_diff(j % NX) <= 2 && (i / NX).abs_diff(j / NX) <= 2 {
                                continue;
                            }
                            let delta = sub(self.position[i], self.position[j]);
                            let length_sq = dot(delta, delta);
                            if length_sq >= self.self_thickness * self.self_thickness {
                                continue;
                            }
                            let wi = if self.pinned[i] { 0. } else { 1. };
                            let wj = if self.pinned[j] { 0. } else { 1. };
                            if wi + wj == 0. {
                                continue;
                            }
                            let length = length_sq.sqrt();
                            let normal = if length < 1e-4 * self.point_scale {
                                [0., 0., 1.]
                            } else {
                                delta.map(|v| v / length)
                            };
                            let correction =
                                (self.self_thickness - length) * self.self_strength / (wi + wj);
                            for (axis, v) in normal.iter().enumerate() {
                                self.position[i][axis] += v * correction * wi;
                                self.position[j][axis] -= v * correction * wj;
                            }
                        }
                    }
                }
            }
        }
    }

    fn collide_surface(&mut self) {
        if !self.flat_floor {
            return;
        }
        self.collision_candidate.copy_from_slice(&self.position);
        self.surface_collision.solve(
            &self.collision_start,
            &mut self.position,
            &self.pinned,
            self.self_thickness,
        );
        for i in 0..N {
            for axis in 0..3 {
                let velocity = if self.pinned[i] {
                    0.
                } else {
                    (self.collision_candidate[i][axis] - self.old[i][axis])
                        * self.surface_collision.last_step_fraction
                };
                self.old[i][axis] = self.position[i][axis] - velocity;
            }
        }
    }

    fn inverse_project(&self, p: [f32; 2], height: f32) -> [f32; 2] {
        let k = K * height / self.point_scale;
        [
            (p[0] + self.view[0] * 0.5 * k) / (1. + k),
            (p[1] + self.view[1] * 0.5 * k) / (1. + k),
        ]
    }
    fn refresh_projection(&mut self) {
        for (i, p) in self.position.iter().enumerate() {
            self.screen[i] = [
                p[0] + (p[0] - self.view[0] * 0.5) * p[2] * K / self.point_scale,
                p[1] + (p[1] - self.view[1] * 0.5) * p[2] * K / self.point_scale,
            ];
        }
    }
    pub fn hit_node(&self, p: [f32; 2]) -> Option<usize> {
        let mut best = None;
        let mut best_z = f32::NEG_INFINITY;
        for j in 0..NY - 1 {
            for i in 0..NX - 1 {
                let a = j * NX + i;
                let b = a + 1;
                let c = a + NX;
                let d = c + 1;
                let min_x = self.screen[a][0]
                    .min(self.screen[b][0])
                    .min(self.screen[c][0])
                    .min(self.screen[d][0]);
                let max_x = self.screen[a][0]
                    .max(self.screen[b][0])
                    .max(self.screen[c][0])
                    .max(self.screen[d][0]);
                let min_y = self.screen[a][1]
                    .min(self.screen[b][1])
                    .min(self.screen[c][1])
                    .min(self.screen[d][1]);
                let max_y = self.screen[a][1]
                    .max(self.screen[b][1])
                    .max(self.screen[c][1])
                    .max(self.screen[d][1]);
                if p[0] < min_x || p[0] > max_x || p[1] < min_y || p[1] > max_y {
                    continue;
                }
                for [a, b, c] in [[a, b, c], [b, d, c]] {
                    let sign = |a: usize, b: usize| {
                        (p[0] - self.screen[b][0]) * (self.screen[a][1] - self.screen[b][1])
                            - (self.screen[a][0] - self.screen[b][0]) * (p[1] - self.screen[b][1])
                    };
                    let signs = [sign(a, b), sign(b, c), sign(c, a)];
                    if signs.iter().any(|s| *s < 0.) && signs.iter().any(|s| *s > 0.) {
                        continue;
                    }
                    let area = signs.iter().sum::<f32>();
                    if area.abs() < 1e-6 {
                        continue;
                    }
                    let z = (signs[1] * self.position[a][2]
                        + signs[2] * self.position[b][2]
                        + signs[0] * self.position[c][2])
                        / area;
                    if z > best_z {
                        best_z = z;
                        best = [a, b, c].into_iter().min_by(|a, b| {
                            let distance = |k: usize| {
                                (p[0] - self.screen[k][0]).powi(2)
                                    + (p[1] - self.screen[k][1]).powi(2)
                            };
                            distance(*a).total_cmp(&distance(*b))
                        });
                    }
                }
            }
        }
        best
    }
    pub fn begin_grab(&mut self, p: [f32; 2]) -> bool {
        if self.rolling {
            return false;
        }
        let Some(center) = self.hit_node(p) else {
            return false;
        };
        self.end_grab();
        let c = self.position[center];
        self.pointer = if self.flat_floor {
            p
        } else {
            self.inverse_project(p, 28. * self.point_scale)
        };
        self.grab_origin = p;
        self.grab_base_height = if self.flat_floor {
            c[2].max(28. * self.point_scale)
        } else {
            28. * self.point_scale
        };
        self.grab_lift = self.grab_base_height;
        // Grab a handful, not a needle: the radius follows the grid spacing so a coarse
        // full-screen sheet pins a patch instead of a single vertex that spikes when dragged.
        let cell = (self.width / (NX - 1) as f32).max(self.height / (NY - 1) as f32);
        let radius = (26. * self.point_scale).max(cell * 1.7);
        for i in 0..N {
            let material_distance = ((i % NX).abs_diff(center % NX) as f32 * self.width
                / (NX - 1) as f32)
                .hypot((i / NX).abs_diff(center / NX) as f32 * self.height / (NY - 1) as f32);
            let d = (self.position[i][0] - c[0]).hypot(self.position[i][1] - c[1]);
            if (!self.flat_floor || material_distance < radius)
                && d < radius
                && (self.position[i][2] - c[2]).abs() < 25. * self.point_scale
            {
                let target = if self.flat_floor {
                    self.inverse_project(p, self.position[i][2])
                } else {
                    self.pointer
                };
                self.pins.push(Pin {
                    index: i,
                    offset: [
                        self.position[i][0] - target[0],
                        self.position[i][1] - target[1],
                    ],
                    weight: (1. - 0.6 * d / radius).max(0.15),
                    height_offset: self.position[i][2] - c[2],
                });
                self.pinned[i] = true;
            }
        }
        self.settled = 0;
        true
    }
    pub fn move_grab(&mut self, p: [f32; 2]) {
        let p = [p[0].clamp(0., self.view[0]), p[1].clamp(0., self.view[1])];
        self.pointer = if self.flat_floor {
            p
        } else {
            self.inverse_project(p, 28. * self.point_scale)
        };
        if self.elastic > 0. {
            let travel = (p[0] - self.grab_origin[0]).hypot(p[1] - self.grab_origin[1]);
            self.grab_lift = (self.grab_base_height + travel * 0.40)
                .min(self.grab_base_height.max(self.view[1] * 0.35));
        }
        self.settled = 0;
    }
    pub fn end_grab(&mut self) {
        for i in 0..N {
            for axis in 0..2 {
                self.old[i][axis] =
                    self.position[i][axis] - (self.position[i][axis] - self.old[i][axis]) * 0.1;
            }
            self.pinned[i] = false;
        }
        self.pins.clear();
        self.settled = 0;
    }
    pub fn grabbing(&self) -> bool {
        !self.pins.is_empty()
    }

    pub fn toggle_roll(&mut self, right: bool) {
        if self.rolling {
            self.roll_goal = 1. - self.roll_goal;
            self.roll_blend = 1.;
        } else {
            self.end_grab();
            self.roll_center = [0.; 2];
            let mut ex = [0.; 2];
            for p in &self.position {
                self.roll_center[0] += p[0] / N as f32;
                self.roll_center[1] += p[1] / N as f32;
            }
            for j in 0..NY {
                for (axis, value) in ex.iter_mut().enumerate() {
                    *value += (self.position[j * NX + NX - 1][axis] - self.position[j * NX][axis])
                        / NY as f32;
                }
            }
            let length = ex[0].hypot(ex[1]);
            self.roll_ex = if length > 20. * self.point_scale {
                [ex[0] / length, ex[1] / length]
            } else {
                [1., 0.]
            };
            if self.roll_ex[0] < 0. {
                self.roll_ex = self.roll_ex.map(|v| -v);
            }
            self.roll_ey = [-self.roll_ex[1], self.roll_ex[0]];
            self.roll_dir = if right { 1. } else { -1. };
            self.roll_start.clone_from(&self.position);
            self.roll_progress = 0.;
            self.roll_goal = 1.;
            self.roll_blend = 0.;
            self.rolling = true;
        }
        self.settled = 0;
    }
    fn step_roll(&mut self) {
        let moving = self.roll_progress != self.roll_goal || self.roll_blend < 1.;
        if self.roll_progress < self.roll_goal {
            self.roll_progress = self.roll_goal.min(self.roll_progress + 1. / 96.);
        } else {
            self.roll_progress = self.roll_goal.max(self.roll_progress - 1. / 78.);
        }
        self.roll_blend = (self.roll_blend + 1. / 27.).min(1.);
        let ease = |x: f32| x * x * (3. - 2. * x);
        let sf = self.width * ease(self.roll_progress);
        let blend = ease(self.roll_blend);
        let thickness = 9. * self.point_scale;
        let core = 5. * self.point_scale;
        let theta =
            |s: f32| (-core + (core * core + thickness * s / PI).sqrt()) / (thickness / (2. * PI));
        for j in 0..NY {
            for i in 0..NX {
                let u = (i as f32 / (NX - 1) as f32 - 0.5) * self.width;
                let v = (j as f32 / (NY - 1) as f32 - 0.5) * self.height;
                let a = self.roll_dir * u;
                let s = a + self.width * 0.5;
                let mut along = a;
                let mut z = 0.;
                if s <= sf {
                    let th = theta(s);
                    let end = theta(sf);
                    let radius = core + thickness * th / (2. * PI);
                    let alpha = -PI * 0.5 - (end - th);
                    along = -self.width * 0.5 + sf + radius * alpha.cos();
                    z = core + thickness * end / (2. * PI) + radius * alpha.sin();
                }
                let x = self.roll_center[0]
                    + self.roll_ey[0] * v
                    + self.roll_ex[0] * self.roll_dir * along;
                let y = self.roll_center[1]
                    + self.roll_ey[1] * v
                    + self.roll_ex[1] * self.roll_dir * along;
                if s > sf {
                    z = self.height_at(x, y);
                }
                let k = j * NX + i;
                let target = [x, y, z];
                for (axis, value) in target.iter().enumerate() {
                    self.position[k][axis] =
                        self.roll_start[k][axis] + (value - self.roll_start[k][axis]) * blend;
                }
                self.old[k] = self.position[k];
                self.floor[k] = 0.;
            }
        }
        self.settled = if moving {
            0
        } else {
            self.settled.saturating_add(1)
        };
        self.max_speed = 0.;
        self.impact_sum = 0.;
        self.slide_level = if moving { 1.6 } else { 0. };
        if self.roll_progress == 0. && self.roll_goal == 0. && self.roll_blend >= 1. {
            self.rolling = false;
            self.settled = 0;
            for i in 0..N {
                self.position[i][2] = self.position[i][2].max(0.) + 0.2 * self.point_scale;
                self.old[i] = self.position[i];
            }
        }
    }
    pub fn vertices(&self) -> Vec<Vertex> {
        let mut vertices = vec![Vertex::default(); N];
        self.fill_vertices(&mut vertices);
        vertices
    }
    pub fn fill_vertices(&self, out: &mut [Vertex]) {
        assert_eq!(out.len(), N);
        for (k, vertex) in out.iter_mut().enumerate() {
            let i = k % NX;
            let j = k / NX;
            let a = self.position[j * NX + (i + 1).min(NX - 1)];
            let b = self.position[j * NX + i.saturating_sub(1)];
            let c = self.position[(j + 1).min(NY - 1) * NX + i];
            let d = self.position[j.saturating_sub(1) * NX + i];
            let mut t1 = sub(a, b);
            let mut t2 = sub(c, d);
            t1[2] *= 2.6;
            t2[2] *= 2.6;
            let normal = [
                t1[1] * t2[2] - t1[2] * t2[1],
                t1[2] * t2[0] - t1[0] * t2[2],
                t1[0] * t2[1] - t1[1] * t2[0],
            ];
            let length = dot(normal, normal).sqrt();
            *vertex = Vertex {
                position: self.position[k],
                uv: [i as f32 / (NX - 1) as f32, j as f32 / (NY - 1) as f32],
                normal: if length > 1e-6 {
                    normal.map(|v| v / length)
                } else {
                    [0., 0., 1.]
                },
                ao: 1.
                    - (((a[2] + b[2] + c[2] + d[2]) * 0.25 - self.position[k][2]) * 0.06
                        / self.point_scale)
                        .clamp(0., 0.5),
            };
        }
    }
    pub fn indices() -> Vec<u16> {
        let mut result = Vec::with_capacity((NX - 1) * (NY - 1) * 6);
        for j in 0..NY - 1 {
            for i in 0..NX - 1 {
                let a = (j * NX + i) as u16;
                let b = a + 1;
                let c = a + NX as u16;
                let d = c + 1;
                result.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        result
    }
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn random_unit(state: &mut u64) -> f32 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    ((z >> 11) as f64 / (1u64 << 53) as f64) as f32
}
#[cfg(test)]
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_testing_uses_the_surface_depth_under_the_pointer() {
        let mut c = Cloth::new([1920., 1080.]);
        c.reset_fullscreen(c.view, 64.);
        for j in 0..NY {
            let upper = j < NY / 2;
            let row = if upper { j } else { j - NY / 2 };
            let z = if upper {
                if j == 1 { 300. } else { 0. }
            } else {
                50.
            };
            for i in 0..NX {
                let xy = c.inverse_project([i as f32 * 30., row as f32 * 30.], z);
                c.position[j * NX + i] = [xy[0], xy[1], z];
            }
        }
        c.refresh_projection();
        let hit = c
            .hit_node([401., 2.])
            .expect("overlapping surfaces must be hit");
        assert!(
            hit / NX >= NY / 2,
            "at the pointer the flat 50px layer is above the sloped 20px layer"
        );
    }

    #[test]
    fn fullscreen_dpi_change_preserves_home_and_active_grab() {
        let view = [1600., 900.];
        let mut normal = Cloth::new(view);
        let mut scaled = Cloth::new(view);
        let node = (NY / 2) * NX + NX / 2;
        let mut offset = [0.; 2];
        for c in [&mut normal, &mut scaled] {
            c.reset_fullscreen(view, 64.);
            offset = [c.screen[node][0] - 800., c.screen[node][1] - 450.];
            assert!(c.begin_grab([800., 450.]));
            c.move_grab([890., 490.]);
            for _ in 0..20 {
                c.step();
            }
        }
        scaled.set_point_scale(1.5);
        scaled.view = view.map(|v| v * 1.5);
        for (a, b) in normal.position.iter().zip(&scaled.position) {
            for axis in 0..3 {
                assert!(
                    (a[axis] - b[axis] / 1.5).abs() < 0.001,
                    "DPI change must preserve the immediate logical pose"
                );
            }
        }
        normal.move_grab([910., 500.]);
        scaled.move_grab([1365., 750.]);
        assert!((normal.grab_lift - scaled.grab_lift / 1.5).abs() < 0.001);
        for (a, b) in normal.rest.iter().zip(&scaled.rest) {
            for axis in 0..3 {
                assert!((a[axis] - b[axis] / 1.5).abs() < 0.001);
            }
        }
        for _ in 0..20 {
            normal.step();
            scaled.step();
        }
        for c in [&normal, &scaled] {
            assert!(c.grabbing(), "DPI change must preserve the active grip");
            let error = (c.screen[node][0] / c.point_scale - 910. - offset[0])
                .hypot(c.screen[node][1] / c.point_scale - 500. - offset[1]);
            assert!(
                error < 3.,
                "logical grab point drifted {error}px after DPI change"
            );
            assert_eq!(
                crossing_pairs(c),
                0,
                "DPI change must preserve a valid surface"
            );
            assert_finite(c);
        }
    }

    #[test]
    fn a_raised_fullscreen_grip_does_not_stretch_its_fixed_edges() {
        let mut c = Cloth::new([1920., 1080.]);
        c.reset_fullscreen(c.view, 64.);
        assert!(c.begin_grab([960., 540.]));
        c.move_grab([350., 240.]);
        for _ in 0..90 {
            c.step();
        }
        let ratio = c
            .constraints
            .iter()
            .filter(|edge| c.pinned[edge.a] && c.pinned[edge.b] && edge.strength == 1.)
            .map(|edge| {
                let d = sub(c.position[edge.a], c.position[edge.b]);
                dot(d, d).sqrt() / edge.rest
            })
            .fold(1_f32, f32::max);
        assert!(
            ratio < 1.15,
            "fixed grip edges stretch to {ratio:.2} times their material length"
        );
    }

    fn fold_report(c: &Cloth) -> (f32, usize) {
        let mut closest = f32::MAX;
        let mut packed = 0;
        for i in 0..N {
            for j in i + 1..N {
                if (i % NX).abs_diff(j % NX) <= 2 && (i / NX).abs_diff(j / NX) <= 2 {
                    continue;
                }
                let d = (c.position[i][0] - c.position[j][0])
                    .hypot(c.position[i][1] - c.position[j][1])
                    .hypot(c.position[i][2] - c.position[j][2]);
                closest = closest.min(d);
                if d < c.self_thickness * 0.5 {
                    packed += 1;
                }
            }
        }
        (closest, packed)
    }

    /// Triangle-level self-intersection count for a folded sheet. Vertex separation alone cannot
    /// prove anything: two layers can keep their vertices apart while large triangles in between
    /// slice straight through each other, which is what shows up as shards and punched-through
    /// wedges on screen.
    fn crossing_pairs(c: &Cloth) -> usize {
        let cell = 48_f32;
        let mut buckets: std::collections::HashMap<(i32, i32, i32), Vec<usize>> =
            std::collections::HashMap::new();
        let mut tris: Vec<[usize; 3]> = Vec::new();
        for j in 0..NY - 1 {
            for i in 0..NX - 1 {
                let a = j * NX + i;
                tris.push([a, a + 1, a + NX]);
                tris.push([a + 1, a + NX + 1, a + NX]);
            }
        }
        for (index, tri) in tris.iter().enumerate() {
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for v in tri {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(c.position[*v][axis]);
                    hi[axis] = hi[axis].max(c.position[*v][axis]);
                }
            }
            for x in (lo[0] / cell).floor() as i32..=(hi[0] / cell).floor() as i32 {
                for y in (lo[1] / cell).floor() as i32..=(hi[1] / cell).floor() as i32 {
                    for z in (lo[2] / cell).floor() as i32..=(hi[2] / cell).floor() as i32 {
                        buckets.entry((x, y, z)).or_default().push(index);
                    }
                }
            }
        }
        let mut count = 0;
        let mut seen = std::collections::HashSet::new();
        for list in buckets.values() {
            for a in 0..list.len() {
                for b in a + 1..list.len() {
                    let (first, second) = (list[a].min(list[b]), list[a].max(list[b]));
                    if !seen.insert((first, second)) {
                        continue;
                    }
                    let t1 = tris[first];
                    let t2 = tris[second];
                    if triangles_cross(c, t1, t2) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn triangles_cross(c: &Cloth, t1: [usize; 3], t2: [usize; 3]) -> bool {
        for a in 0..3 {
            let p = c.position[t1[a]];
            let q = c.position[t1[(a + 1) % 3]];
            if segment_hits_triangle(p, q, t2.map(|v| c.position[v])) {
                return true;
            }
        }
        for a in 0..3 {
            let p = c.position[t2[a]];
            let q = c.position[t2[(a + 1) % 3]];
            if segment_hits_triangle(p, q, t1.map(|v| c.position[v])) {
                return true;
            }
        }
        coplanar_triangles_overlap(t1.map(|v| c.position[v]), t2.map(|v| c.position[v]))
    }

    fn coplanar_triangles_overlap(a: [V3; 3], b: [V3; 3]) -> bool {
        let normal = cross(sub(a[1], a[0]), sub(a[2], a[0]));
        let magnitude = dot(normal, normal).sqrt();
        if magnitude < 1e-5
            || b.iter()
                .any(|p| dot(normal, sub(*p, a[0])).abs() > magnitude * 1e-4)
        {
            return false;
        }
        let dropped = (0..3)
            .max_by(|i, j| normal[*i].abs().total_cmp(&normal[*j].abs()))
            .unwrap();
        let axes: Vec<_> = (0..3).filter(|axis| *axis != dropped).collect();
        let project = |p: V3| [p[axes[0]], p[axes[1]]];
        let a = a.map(project);
        let b = b.map(project);
        let orient = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        let epsilon = magnitude * 1e-5;
        let inside = |p: [f32; 2], tri: [[f32; 2]; 3]| {
            let s = [
                orient(tri[0], tri[1], p),
                orient(tri[1], tri[2], p),
                orient(tri[2], tri[0], p),
            ];
            s.iter().all(|s| *s > epsilon) || s.iter().all(|s| *s < -epsilon)
        };
        for i in 0..3 {
            if inside(a[i], b) || inside(b[i], a) {
                return true;
            }
            for j in 0..3 {
                let ab = [
                    orient(a[i], a[(i + 1) % 3], b[j]),
                    orient(a[i], a[(i + 1) % 3], b[(j + 1) % 3]),
                ];
                let ba = [
                    orient(b[j], b[(j + 1) % 3], a[i]),
                    orient(b[j], b[(j + 1) % 3], a[(i + 1) % 3]),
                ];
                if ab[0] * ab[1] < -epsilon * epsilon && ba[0] * ba[1] < -epsilon * epsilon {
                    return true;
                }
            }
        }
        let centroid = |t: [[f32; 2]; 3]| {
            [
                (t[0][0] + t[1][0] + t[2][0]) / 3.,
                (t[0][1] + t[1][1] + t[2][1]) / 3.,
            ]
        };
        inside(centroid(a), b) || inside(centroid(b), a)
    }

    #[test]
    fn intersection_check_distinguishes_coplanar_overlap_from_shared_edges() {
        let a = [[0., 0., 0.], [10., 0., 0.], [0., 10., 0.]];
        assert!(coplanar_triangles_overlap(
            a,
            [[1., 1., 0.], [8., 1., 0.], [1., 8., 0.]]
        ));
        assert!(coplanar_triangles_overlap(a, a));
        assert!(!coplanar_triangles_overlap(
            a,
            [[10., 0., 0.], [10., 10., 0.], [0., 10., 0.]]
        ));
        assert!(!coplanar_triangles_overlap(a, a.map(|p| [p[0], p[1], 1.])));
    }

    #[test]
    fn intersection_check_accepts_a_bent_shared_edge() {
        let mut c = Cloth::new([1920., 1080.]);
        c.position[0] = [723.74194, 540.0234, 0.0006926535];
        c.position[1] = [755.2474, 540.04407, 0.04266048];
        c.position[2] = [723.7525, 567.4779, 0.0013241249];
        c.position[3] = [755.32764, 567.4984, 0.1454956];
        assert!(
            !triangles_cross(&c, [0, 1, 2], [1, 3, 2]),
            "adjacent faces touching only along their shared edge do not intersect"
        );
        c.position[4] = [738., 550., -10.];
        c.position[5] = [738., 550., 10.];
        assert!(segment_hits_triangle(
            c.position[4],
            c.position[5],
            [c.position[0], c.position[1], c.position[2]]
        ));
    }

    fn segment_hits_triangle(p: V3, q: V3, tri: [V3; 3]) -> bool {
        // Independent f64 predicate: f32 cancellation on a shared edge can invent an interior hit.
        let sub = |a: [f64; 3], b: [f64; 3]| std::array::from_fn(|i| a[i] - b[i]);
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let p = p.map(f64::from);
        let q = q.map(f64::from);
        let tri = tri.map(|p| p.map(f64::from));
        let e1 = sub(tri[1], tri[0]);
        let e2 = sub(tri[2], tri[0]);
        let dir = sub(q, p);
        let h = cross(dir, e2);
        let det = dot(e1, h);
        let tolerance = 1e-10 * (dot(e1, e1) * dot(e2, e2) * dot(dir, dir)).sqrt();
        if det.abs() <= tolerance {
            return false;
        }
        let inv = 1. / det;
        let s = sub(p, tri[0]);
        let u = dot(s, h) * inv;
        if !(0. ..=1.).contains(&u) {
            return false;
        }
        let qv = cross(s, e1);
        let v = dot(dir, qv) * inv;
        if v < 0. || u + v > 1. {
            return false;
        }
        let t = dot(e2, qv) * inv;
        t > 1e-4 && t < 1. - 1e-4
    }

    #[test]
    fn desktop_fold_keeps_its_layers_apart() {
        // Regression for the punched-through patch users saw at the fold tip: the sheet is
        // grabbed off-centre and dragged hard, which is the worst case for self-intersection.
        let view = [1920., 1080.];
        let mut c = Cloth::new(view);
        c.reset_fullscreen(view, 64.);
        for _ in 0..30 {
            c.step();
        }
        let grab = [view[0] * 0.42, view[1] * 0.55];
        assert!(c.begin_grab(grab));
        for step in 0..140 {
            let t = step as f32 / 139.;
            c.move_grab([grab[0] - 520. * t, grab[1] - 300. * t]);
            c.step();
        }
        assert_eq!(crossing_pairs(&c), 0, "folded sheet intersects itself");
        let (closest, packed) = fold_report(&c);
        assert!(
            closest >= c.self_thickness * 0.5,
            "layers collapse onto each other: closest {closest}, thickness {}",
            c.self_thickness
        );
        assert_eq!(packed, 0, "layers are packed tighter than half a thickness");
        c.end_grab();
        for _ in 0..300 {
            c.step();
        }
        assert_eq!(crossing_pairs(&c), 0, "settled sheet intersects itself");
        assert_finite(&c);
    }

    #[test]
    fn fullscreen_drag_paths_keep_layers_apart_without_stalling() {
        let cases = [
            ("left", [1920., 1080.], 1., [0.42, 0.55], [-520., -300.]),
            ("right", [1920., 1080.], 1., [0.58, 0.45], [520., 300.]),
            ("wide", [3440., 1440.], 1., [0.5, 0.5], [-700., 380.]),
            ("scaled", [2560., 1600.], 1.5, [0.42, 0.55], [-780., -450.]),
        ];
        for (name, view, scale, origin, travel) in cases {
            let mut c = Cloth::new(view);
            c.set_point_scale(scale);
            c.reset_fullscreen(view, 64. * scale);
            let grab = [view[0] * origin[0], view[1] * origin[1]];
            let node = c.hit_node(grab).expect("initial grab must hit");
            let before = c.screen[node];
            assert!(c.begin_grab(grab), "{name}: initial grab must hit");
            for step in 0..140 {
                let t = step as f32 / 139.;
                c.move_grab([grab[0] + travel[0] * t, grab[1] + travel[1] * t]);
                c.step();
                assert_finite(&c);
                assert_eq!(crossing_pairs(&c), 0, "{name}: crossing at step {step}");
            }
            let distance = travel[0].hypot(travel[1]);
            let progress = ((c.screen[node][0] - before[0]) * travel[0]
                + (c.screen[node][1] - before[1]) * travel[1])
                / distance;
            assert!(
                progress > distance * 0.75,
                "{name}: grab stalled at {progress}/{distance}"
            );
        }
    }

    #[test]
    fn grabbing_a_fold_selects_only_the_top_material_patch() {
        let view = [1920., 1080.];
        let mut c = Cloth::new(view);
        c.reset_fullscreen(view, 64.);
        for position in c.position.iter_mut().skip((NY / 2) * NX) {
            position[1] = view[1] - position[1];
            position[2] = 12.;
        }
        c.refresh_projection();
        assert!(c.begin_grab([view[0] * 0.4, view[1] * 0.25]));
        assert!(c.pins.len() >= 4, "the grab should hold a cloth patch");
        assert!(
            c.pins.iter().all(|pin| pin.index / NX >= NY / 2),
            "grabbing one fold must not pin the layer underneath"
        );
    }

    #[test]
    fn raised_fullscreen_grab_tracks_the_pointer_in_screen_space() {
        let mut c = Cloth::new([1920., 1080.]);
        c.reset_fullscreen(c.view, 64.);
        let start = [800., 550.];
        let end = [400., 300.];
        let node = c.hit_node(start).unwrap();
        let offset = [c.screen[node][0] - start[0], c.screen[node][1] - start[1]];
        assert!(c.begin_grab(start));
        for step in 0..140 {
            let t = step as f32 / 139.;
            c.move_grab([
                start[0] + (end[0] - start[0]) * t,
                start[1] + (end[1] - start[1]) * t,
            ]);
            c.step();
        }
        let error =
            (c.screen[node][0] - end[0] - offset[0]).hypot(c.screen[node][1] - end[1] - offset[1]);
        assert!(
            error < 20.,
            "raised fabric drifted {error}px from its grab point"
        );
    }

    #[test]
    fn fullscreen_corner_drag_keeps_following_across_the_desktop() {
        for (view, start, end) in [
            ([2560., 1600.], [2480., 1440.], [200., 240.]),
            ([4480., 1600.], [4400., 1440.], [2120., 240.]),
        ] {
            let mut c = Cloth::new(view);
            c.set_point_scale(1.25);
            c.reset_fullscreen(view, 80.);
            let node = c.hit_node(start).unwrap();
            let offset = [c.screen[node][0] - start[0], c.screen[node][1] - start[1]];
            assert!(c.begin_grab(start));
            for step in 0..240 {
                let t = (step as f32 / 160.).min(1.);
                let pointer = std::array::from_fn(|a| start[a] + (end[a] - start[a]) * t);
                c.move_grab(pointer);
                c.step();
                let error = (c.screen[node][0] - pointer[0] - offset[0])
                    .hypot(c.screen[node][1] - pointer[1] - offset[1]);
                assert!(
                    error < 80. * c.point_scale(),
                    "corner grab stopped following: view={view:?}, step={step}, error={error}"
                );
                assert_finite(&c);
                assert_eq!(crossing_pairs(&c), 0, "corner drag crossed at step {step}");
            }
            c.end_grab();
            for step in 0..360 {
                c.step();
                assert_finite(&c);
                if step % 30 == 0 {
                    assert_eq!(
                        crossing_pairs(&c),
                        0,
                        "corner release crossed at step {step}"
                    );
                }
            }
            assert!(c.position.iter().all(|p| p[2] < 2. * c.point_scale()));
        }
    }

    #[test]
    fn fullscreen_fast_reversals_do_not_tunnel_or_freeze() {
        let view = [1920., 1080.];
        let mut c = Cloth::new(view);
        c.reset_fullscreen(view, 64.);
        let grab = [960., 540.];
        let node = c.hit_node(grab).unwrap();
        assert!(c.begin_grab(grab));
        for step in 0..90 {
            let target = if (step / 30) % 2 == 0 {
                [350., 240.]
            } else {
                [1560., 840.]
            };
            c.move_grab(target);
            c.step();
            assert_finite(&c);
            assert_eq!(
                crossing_pairs(&c),
                0,
                "fast reversal crosses the surface at step {step}"
            );
        }
        assert!(
            c.screen[node][0] < grab[0] - 200.,
            "collision handling must not freeze dragging"
        );
        c.end_grab();
        for _ in 0..300 {
            c.step();
        }
        assert_eq!(
            crossing_pairs(&c),
            0,
            "release must not leave intersections"
        );
        assert_finite(&c);
    }

    #[test]
    fn fullscreen_sheet_spans_the_view_and_returns_home_after_a_drag() {
        let view = [1600., 900.];
        let bleed = 60.;
        let mut c = Cloth::new(view);
        c.bumps = vec![[300., 300.], [340., 320.]];
        c.reset_fullscreen(view, bleed);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for p in &c.position {
            min_x = min_x.min(p[0]);
            max_x = max_x.max(p[0]);
            min_y = min_y.min(p[1]);
            max_y = max_y.max(p[1]);
        }
        assert!(min_x <= -bleed + 1. && max_x >= view[0] + bleed - 1., "covers width");
        assert!(min_y <= -bleed + 1. && max_y >= view[1] + bleed - 1., "covers height");
        for _ in 0..60 {
            c.step();
        }
        assert!(
            c.position.iter().all(|p| p[2].abs() < 0.001),
            "the sheet must rest on a flat plane and ignore desktop icon bumps"
        );
        let middle = (NY / 2) * NX + NX / 2;
        assert!(c.begin_grab([view[0] * 0.5, view[1] * 0.5]));
        let grabbed = (0..N).filter(|k| c.pinned[*k]).count();
        assert!(grabbed >= 4, "a coarse sheet must grab a patch: {grabbed}");
        let before = c.position[middle][0];
        c.move_grab([view[0] * 0.5 + 420., view[1] * 0.5 - 300.]);
        assert!(c.grab_lift <= view[1] * 0.35, "drag lift must stay bounded");
        for _ in 0..90 {
            c.step();
        }
        assert!(c.position[middle][2] > 20., "dragging lifts the grabbed cloth");
        assert!(
            c.position[middle][0] > before + 60.,
            "dragging pulls the sheet away from where it lay"
        );
        c.end_grab();
        for _ in 0..300 {
            c.step();
        }
        let worst = c
            .position
            .iter()
            .zip(c.rest.iter())
            .map(|(p, r)| (p[0] - r[0]).hypot(p[1] - r[1]))
            .fold(0f32, f32::max);
        assert!(worst < 3., "the sheet must slide home again, off by {worst}");
        assert!(
            c.position.iter().all(|p| p[2].abs() < 1.),
            "and lie flat once it is home"
        );
        assert_finite(&c);
    }
    #[test]
    fn high_dpi_preserves_logical_physics_and_audio_units() {
        let mut normal = Cloth::new([1440., 900.]);
        let mut scaled = Cloth::new([1440., 900.]);
        for cloth in [&mut normal, &mut scaled] {
            for index in 0..N {
                cloth.position[index][2] = 130.;
                cloth.old[index][2] = 130.;
            }
        }
        scaled.set_point_scale(1.5);
        scaled.view = [2160., 1350.];
        for _ in 0..45 {
            normal.step();
            scaled.step();
        }
        for (a, b) in normal.position.iter().zip(scaled.position.iter()) {
            for axis in 0..3 {
                assert!(
                    (a[axis] - b[axis] / 1.5).abs() < 0.15,
                    "axis {axis}: {a:?} {b:?}"
                );
            }
        }
        normal.bumps = vec![[576., 468.]];
        scaled.bumps = vec![[864., 702.]];
        assert!((normal.height_at(576., 468.) - scaled.height_at(864., 702.) / 1.5).abs() < 0.001);
        assert!((normal.perspective() - scaled.perspective() * 1.5).abs() < 1e-8);
        assert!((normal.max_speed - scaled.max_speed).abs() < 0.01);
    }
    #[test]
    fn scaling_keeps_folds_and_translation_keeps_roll_state() {
        let mut cloth = Cloth::new([1440., 900.]);
        for _ in 0..160 {
            cloth.step();
        }
        assert!(cloth.begin_grab([576., 468.]));
        cloth.move_grab([760., 480.]);
        for _ in 0..80 {
            cloth.step();
        }
        cloth.end_grab();
        let z: Vec<_> = cloth.position.iter().map(|p| p[2]).collect();
        let width = cloth.width;
        let center_before = cloth.position.iter().map(|p| p[0]).sum::<f32>() / N as f32;
        cloth.scale(1.45);
        let center_after = cloth.position.iter().map(|p| p[0]).sum::<f32>() / N as f32;
        assert!((center_before - center_after).abs() < 0.01);
        assert!((cloth.width - width * 1.45).abs() < 0.01);
        assert!(cloth.position.iter().zip(z.iter()).all(|(p, z)| p[2] == *z));
        cloth.toggle_roll(true);
        for _ in 0..30 {
            cloth.step();
        }
        let progress = cloth.roll_progress;
        let first = cloth.position[0];
        cloth.translate([200., -40.]);
        assert!(cloth.rolling);
        assert_eq!(cloth.roll_progress, progress);
        assert!((cloth.position[0][0] - first[0] - 200.).abs() < 0.001);
        assert!((cloth.position[0][1] - first[1] + 40.).abs() < 0.001);
        cloth.step();
        assert_finite(&cloth);
        cloth.reset(1.);
        assert!(cloth.floor.iter().all(|height| *height == 0.));
    }
    fn assert_finite(c: &Cloth) {
        assert!(c.vertices().iter().all(|v| {
            v.position
                .iter()
                .chain(v.normal.iter())
                .all(|x| x.is_finite())
        }));
    }
    #[test]
    fn dropped_rug_lands_and_sleeps() {
        let mut c = Cloth::new([1440., 900.]);
        for _ in 0..600 {
            c.step();
        }
        assert_finite(&c);
        assert!(c.position.iter().all(|p| p[2].abs() < 0.1));
        assert!(c.settled > 60);
        assert!(c.hit_node([576., 468.]).is_some());
        assert!(c.hit_node([-100., -100.]).is_none());
    }
    #[test]
    fn grab_follows_pointer_then_releases() {
        let mut c = Cloth::new([1440., 900.]);
        for _ in 0..150 {
            c.step();
        }
        let before = c.position[N / 2][0];
        assert!(c.begin_grab([576., 468.]));
        c.move_grab([730., 500.]);
        for _ in 0..120 {
            c.step();
        }
        assert!(c.position[N / 2][0] > before + 50.);
        assert_finite(&c);
        c.end_grab();
        assert!(!c.grabbing());
        assert!(c.pinned.iter().all(|x| !x));
    }
    #[test]
    fn roll_unroll_preserves_finite_mesh() {
        let mut c = Cloth::new([1440., 900.]);
        for _ in 0..150 {
            c.step();
        }
        c.toggle_roll(true);
        for _ in 0..120 {
            c.step();
        }
        assert!(c.rolling);
        assert_eq!(c.roll_progress, 1.);
        assert_finite(&c);
        c.toggle_roll(true);
        for _ in 0..110 {
            c.step();
        }
        assert!(!c.rolling);
        assert_finite(&c);
        assert!(c.position.iter().all(|p| p[2] >= 0.));
    }
    #[test]
    fn desktop_bumps_support_rug_without_penetration() {
        let mut c = Cloth::new([1440., 900.]);
        c.bumps.push([576., 468.]);
        for _ in 0..300 {
            c.step();
        }
        assert_finite(&c);
        assert_eq!(c.height_at(576., 468.), 20.);
        assert_eq!(c.height_at(700., 468.), 0.);
        assert!(c.position.iter().any(|p| p[2] > 15.));
        assert!(
            c.position
                .iter()
                .all(|p| p[2] + 0.01 >= c.height_at(p[0], p[1]))
        );
    }

    #[test]
    fn initial_rug_uses_primary_monitor_inside_virtual_desktop() {
        let mut c = Cloth::new([4480., 1600.]);
        c.reset_in_rect(1., [1920., 0.], [2560., 1600.]);
        let center = c.position.iter().map(|p| p[0]).sum::<f32>() / N as f32;
        assert!((center - 2944.).abs() < 0.1);
        assert!(c.position.iter().all(|p| p[0] >= 1920. && p[0] < 4480.));
        assert_eq!(c.view, [4480., 1600.]);
        assert_finite(&c);
    }
}
