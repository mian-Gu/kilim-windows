//! Surface contacts for the free desktop sheet. All broad-phase traversal is deterministic.

type V3 = [f32; 3];
const GRID_BUCKETS: usize = 16_384;

#[derive(Clone, Copy, Default)]
struct Aabb {
    lo: V3,
    hi: V3,
}

impl Aabb {
    fn from_motion(start: V3, end: V3) -> Self {
        Self {
            lo: std::array::from_fn(|i| start[i].min(end[i])),
            hi: std::array::from_fn(|i| start[i].max(end[i])),
        }
    }

    fn union(self, other: Self) -> Self {
        Self {
            lo: std::array::from_fn(|i| self.lo[i].min(other.lo[i])),
            hi: std::array::from_fn(|i| self.hi[i].max(other.hi[i])),
        }
    }

    fn expanded(self, padding: f32) -> Self {
        Self {
            lo: self.lo.map(|v| v - padding),
            hi: self.hi.map(|v| v + padding),
        }
    }

    fn overlaps(self, other: Self) -> bool {
        self.lo[0] <= other.hi[0]
            && self.hi[0] >= other.lo[0]
            && self.lo[1] <= other.hi[1]
            && self.hi[1] >= other.lo[1]
            && self.lo[2] <= other.hi[2]
            && self.hi[2] >= other.lo[2]
    }
}

#[derive(Default)]
struct Grid {
    heads: Vec<usize>,
    tails: Vec<usize>,
    query_cursors: Vec<usize>,
    cursors_valid: bool,
    cursor_after: usize,
    entries: Vec<GridEntry>,
    cell: f32,
}

struct GridEntry {
    key: [i32; 3],
    feature: usize,
    next: usize,
}

impl Grid {
    fn clear(&mut self, cell: f32) {
        if self.heads.is_empty() {
            self.heads.resize(GRID_BUCKETS, usize::MAX);
            self.tails.resize(GRID_BUCKETS, usize::MAX);
            self.query_cursors.resize(GRID_BUCKETS, usize::MAX);
        } else {
            self.heads.fill(usize::MAX);
            self.tails.fill(usize::MAX);
        }
        self.entries.clear();
        self.cursors_valid = false;
        self.cell = cell;
    }

    fn hash(key: [i32; 3]) -> usize {
        ((key[0] as u32).wrapping_mul(73_856_093)
            ^ (key[1] as u32).wrapping_mul(19_349_663)
            ^ (key[2] as u32).wrapping_mul(83_492_791)) as usize
            & (GRID_BUCKETS - 1)
    }

    fn bounds(&self, lo: V3, hi: V3) -> ([i32; 3], [i32; 3]) {
        (
            lo.map(|x| (x / self.cell).floor() as i32),
            hi.map(|x| (x / self.cell).floor() as i32),
        )
    }

    fn insert(&mut self, index: usize, lo: V3, hi: V3) {
        // Keep query cursors independent from insertion heads/tails. An insertion after a query
        // invalidates only the cursors, so neither a future query nor appended entries lose data.
        self.cursors_valid = false;
        let (lo, hi) = self.bounds(lo, hi);
        for z in lo[2]..=hi[2] {
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let key = [x, y, z];
                    let bucket = Self::hash(key);
                    let next = self.entries.len();
                    self.entries.push(GridEntry {
                        key,
                        feature: index,
                        next: usize::MAX,
                    });
                    let tail = self.tails[bucket];
                    if tail == usize::MAX {
                        self.heads[bucket] = next;
                    } else {
                        self.entries[tail].next = next;
                    }
                    // Append, rather than prepend, to preserve feature traversal order exactly.
                    self.tails[bucket] = next;
                }
            }
        }
    }

    fn query(&self, lo: V3, hi: V3, output: &mut Vec<usize>, seen: &mut [usize], stamp: usize) {
        output.clear();
        let (lo, hi) = self.bounds(lo, hi);
        for z in lo[2]..=hi[2] {
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let key = [x, y, z];
                    let mut next = self.heads[Self::hash(key)];
                    while next != usize::MAX {
                        let entry = &self.entries[next];
                        if entry.key == key && seen[entry.feature] != stamp {
                            seen[entry.feature] = stamp;
                            output.push(entry.feature);
                        }
                        next = entry.next;
                    }
                }
            }
        }
    }

    fn query_after(
        &mut self,
        lo: V3,
        hi: V3,
        output: &mut Vec<usize>,
        seen: &mut [usize],
        stamp: usize,
        after: usize,
    ) {
        // EE queries increase their first feature monotonically. Bucket entries are appended in
        // feature order, so discarded prefixes never need scanning again. A lower threshold or
        // another insertion resets the cursors without changing the ordinary VF query.
        if !self.cursors_valid || after < self.cursor_after {
            self.query_cursors.copy_from_slice(&self.heads);
            self.cursors_valid = true;
        }
        self.cursor_after = after;
        output.clear();
        let (lo, hi) = self.bounds(lo, hi);
        for z in lo[2]..=hi[2] {
            for y in lo[1]..=hi[1] {
                for x in lo[0]..=hi[0] {
                    let key = [x, y, z];
                    let cursor = &mut self.query_cursors[Self::hash(key)];
                    while *cursor != usize::MAX && self.entries[*cursor].feature <= after {
                        *cursor = self.entries[*cursor].next;
                    }
                    let mut next = *cursor;
                    while next != usize::MAX {
                        let entry = &self.entries[next];
                        if entry.key == key && entry.feature > after && seen[entry.feature] != stamp
                        {
                            seen[entry.feature] = stamp;
                            output.push(entry.feature);
                        }
                        next = entry.next;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ImpactGroup {
    parent: usize,
    count: usize,
    shift: V3,
    pin_count: usize,
    pin_shift: V3,
    stationary: bool,
    minimum_height: f32,
}

/// Temporary contact groups preserve the incoming shape while retaining common motion.
/// They are rebuilt each step; their proposed positions must still pass complete swept CCD.
struct ImpactGroups {
    groups: Vec<ImpactGroup>,
    requested: Vec<V3>,
}

impl ImpactGroups {
    fn new(count: usize) -> Self {
        Self {
            groups: vec![ImpactGroup::default(); count],
            requested: vec![[0.; 3]; count],
        }
    }

    fn reset(&mut self, position: &[V3]) {
        self.requested.copy_from_slice(position);
        for (parent, group) in self.groups.iter_mut().enumerate() {
            *group = ImpactGroup {
                parent,
                ..Default::default()
            };
        }
    }

    fn root(&mut self, mut index: usize) -> usize {
        while self.groups[index].parent != index {
            self.groups[index].parent = self.groups[self.groups[index].parent].parent;
            index = self.groups[index].parent;
        }
        index
    }

    fn merge(&mut self, ids: [usize; 4], pinned: &[bool]) -> bool {
        if ids.iter().all(|&i| pinned[i]) {
            return false;
        }
        let root = self.root(ids[0]);
        let mut changed = false;
        for id in ids {
            let other = self.root(id);
            if root != other {
                self.groups[other].parent = root;
                changed = true;
            }
        }
        changed
    }

    fn apply(&mut self, start: &[V3], position: &mut [V3], pinned: &[bool]) {
        for group in &mut self.groups {
            *group = ImpactGroup {
                parent: group.parent,
                minimum_height: f32::INFINITY,
                ..Default::default()
            };
        }
        for i in 0..position.len() {
            let root = self.root(i);
            let shift = sub(self.requested[i], start[i]);
            let group = &mut self.groups[root];
            group.count += 1;
            group.shift = add(group.shift, shift);
            group.minimum_height = group.minimum_height.min(start[i][2]);
            if pinned[i] {
                group.pin_count += 1;
                group.pin_shift = add(group.pin_shift, shift);
                group.stationary |= self.requested[i] == start[i];
            }
        }
        for i in 0..position.len() {
            let root = self.root(i);
            let group = &self.groups[root];
            if group.count <= 1 {
                continue;
            }
            let shift = if group.stationary {
                // A true fixed pin remains fixed, even when another grab in its group moves.
                [0.; 3]
            } else {
                let mut shift = if group.pin_count > 0 {
                    mul(group.pin_shift, 1. / group.pin_count as f32)
                } else {
                    mul(group.shift, 1. / group.count as f32)
                };
                shift[2] = shift[2].max(-group.minimum_height);
                shift
            };
            position[i] = add(start[i], shift);
        }
    }
}

pub struct SurfaceCollision {
    nx: usize,
    triangles: Vec<[usize; 3]>,
    edges: Vec<[usize; 2]>,
    grid: Grid,
    seen: Vec<usize>,
    candidates: Vec<usize>,
    vertex_faces: Vec<(usize, usize)>,
    edge_pairs: Vec<(usize, usize)>,
    vertex_bounds: Vec<Aabb>,
    triangle_bounds: Vec<Aabb>,
    edge_bounds: Vec<Aabb>,
    impact_groups: ImpactGroups,
    #[cfg(test)]
    unfiltered_counts: [usize; 2],
    #[cfg(test)]
    candidate_rebuilds: usize,
    pub limited_steps: u64,
    pub last_step_fraction: f32,
}

impl SurfaceCollision {
    pub fn new(nx: usize, ny: usize) -> Self {
        assert!(nx >= 2 && ny >= 2);
        let mut triangles = Vec::with_capacity((nx - 1) * (ny - 1) * 2);
        let mut edges = Vec::with_capacity(nx * ny * 3);
        for y in 0..ny {
            for x in 0..nx {
                let a = y * nx + x;
                if x + 1 < nx {
                    edges.push([a, a + 1]);
                }
                if y + 1 < ny {
                    edges.push([a, a + nx]);
                }
                if x + 1 < nx && y + 1 < ny {
                    triangles.extend([[a, a + 1, a + nx], [a + 1, a + nx + 1, a + nx]]);
                    edges.push([a + 1, a + nx]);
                }
            }
        }
        let seen = vec![0; edges.len().max(triangles.len())];
        let triangle_bounds = vec![Aabb::default(); triangles.len()];
        let edge_bounds = vec![Aabb::default(); edges.len()];
        Self {
            nx,
            triangles,
            edges,
            grid: Grid::default(),
            seen,
            candidates: Vec::new(),
            vertex_faces: Vec::new(),
            edge_pairs: Vec::new(),
            vertex_bounds: vec![Aabb::default(); nx * ny],
            triangle_bounds,
            edge_bounds,
            #[cfg(test)]
            unfiltered_counts: [0; 2],
            #[cfg(test)]
            candidate_rebuilds: 0,
            impact_groups: ImpactGroups::new(nx * ny),
            limited_steps: 0,
            last_step_fraction: 1.,
        }
    }

    /// `start` is the last accepted state, captured before moving the mouse grab or any constraint.
    /// Contact projections do not move pinned vertices. Residual impacts use temporary common
    /// motion groups, which can adjust moving grabs but keep stationary pins fixed. Every repaired
    /// path is rechecked; unresolved impacts still limit the entire step. This is for a flat floor.
    pub fn solve(&mut self, start: &[V3], position: &mut [V3], pinned: &[bool], thickness: f32) {
        assert_eq!(start.len(), position.len());
        assert_eq!(start.len(), pinned.len());
        self.last_step_fraction = 1.;
        let thickness = thickness.max(1e-3);
        if start.iter().zip(position.iter()).all(|(a, b)| a == b) {
            return;
        }
        let mut needs_rebuild = false;
        // During dragging, use groups for residual folds instead of repeatedly projecting
        // against an immovable grab. Once released, keep the original thickness relaxation.
        let passes = if pinned.iter().any(|&fixed| fixed) {
            1
        } else {
            4
        };
        for _ in 0..passes {
            self.find_candidates(start, position, thickness);
            let mut changed = false;
            for &(vertex, face) in &self.vertex_faces {
                let t = self.triangles[face];
                let indices = [vertex, t[0], t[1], t[2]];
                let before = indices.map(|i| start[i]);
                let after = indices.map(|i| position[i]);
                if let Some(contact) = vertex_face_contact(before, after, thickness) {
                    changed |= contact.project(indices, position, pinned, thickness);
                }
            }
            for &(first, second) in &self.edge_pairs {
                let a = self.edges[first];
                let b = self.edges[second];
                let indices = [a[0], a[1], b[0], b[1]];
                let before = indices.map(|i| start[i]);
                let after = indices.map(|i| position[i]);
                if let Some(contact) = edge_contact(before, after, thickness) {
                    changed |= contact.project(indices, position, pinned, thickness);
                }
            }
            for (p, &fixed) in position.iter_mut().zip(pinned) {
                if !fixed && p[2] < 0. {
                    p[2] = 0.;
                    changed = true;
                }
            }
            needs_rebuild = changed;
            if !changed {
                break;
            }
        }

        // Projection can change which features meet. Rebuild from the floor-projected result and
        // refuse to accept a step whose complete swept path still crosses another surface.
        if needs_rebuild {
            self.find_candidates(start, position, thickness);
        }
        let epsilon = (thickness * 1e-4).max(1e-4);
        self.impact_groups.reset(position);
        let (mut has_impact, mut time, mut merged) = self.impacts(start, position, pinned, epsilon);
        // A corner fold can leave one tight contact after projection. Clipping the whole sheet
        // repeatedly drives TOI to zero and strands the grab. Resolve only its connected group,
        // preserving common motion; merging is bounded and never substitutes for the CCD check.
        for _ in 0..6 {
            if !has_impact || !merged {
                break;
            }
            self.impact_groups.apply(start, position, pinned);
            self.find_candidates(start, position, thickness);
            (has_impact, time, merged) = self.impacts(start, position, pinned, epsilon);
        }
        if has_impact {
            let maximum_motion = start
                .iter()
                .zip(position.iter())
                .map(|(&a, &b)| length(sub(b, a)))
                .fold(0_f32, f32::max);
            let coordinate = start
                .iter()
                .chain(position.iter())
                .flat_map(|p| p.iter())
                .map(|v| v.abs())
                .fold(1_f32, f32::max);
            let coordinate_ulp = f32::from_bits(coordinate.to_bits() + 1) - coordinate;
            // A relative time factor alone leaves almost no spatial clearance when TOI is tiny.
            // Reserve both a material-space margin and several world-coordinate ULPs before
            // interpolating back to f32 positions; a too-short safe interval accepts start exactly.
            let space_margin = (thickness * 0.01).max(coordinate_ulp * 4.);
            let time_margin = space_margin / maximum_motion.max(f32::MIN_POSITIVE);
            self.last_step_fraction = (time - time_margin).max(0.) * 0.9;
            self.limited_steps += 1;
            for (p, &before) in position.iter_mut().zip(start) {
                *p = lerp(before, *p, self.last_step_fraction);
            }
        }
    }

    fn impacts(
        &mut self,
        start: &[V3],
        position: &[V3],
        pinned: &[bool],
        epsilon: f32,
    ) -> (bool, f32, bool) {
        let mut has_impact = false;
        let mut time = 1_f32;
        let mut merged = false;
        for &(vertex, face) in &self.vertex_faces {
            let t = self.triangles[face];
            let ids = [vertex, t[0], t[1], t[2]];
            if let Some(hit) =
                vertex_face_impact(ids.map(|i| start[i]), ids.map(|i| position[i]), epsilon)
            {
                has_impact = true;
                time = time.min(hit.time);
                merged |= self.impact_groups.merge(ids, pinned);
            }
        }
        for &(first, second) in &self.edge_pairs {
            let a = self.edges[first];
            let b = self.edges[second];
            let ids = [a[0], a[1], b[0], b[1]];
            if let Some(hit) = edge_impact(ids.map(|i| start[i]), ids.map(|i| position[i]), epsilon)
            {
                has_impact = true;
                time = time.min(hit.time);
                merged |= self.impact_groups.merge(ids, pinned);
            }
        }
        (has_impact, time, merged)
    }

    fn near(&self, a: usize, b: usize) -> bool {
        a == b
    }

    fn find_candidates(&mut self, start: &[V3], position: &[V3], thickness: f32) {
        self.vertex_faces.clear();
        self.edge_pairs.clear();
        self.seen.fill(0);
        #[cfg(test)]
        {
            self.unfiltered_counts = [0; 2];
            self.candidate_rebuilds += 1;
        }
        for (index, bound) in self.vertex_bounds.iter_mut().enumerate() {
            *bound = Aabb::from_motion(start[index], position[index]);
        }
        // The topology is regular; one edge's span provides a scale without a view-size assumption.
        let cell = self
            .edges
            .iter()
            .take(self.nx * 3)
            .map(|e| length(sub(start[e[0]], start[e[1]])))
            .fold(thickness * 2., f32::max)
            .max(1.);
        self.grid.clear(cell);
        for (index, &tri) in self.triangles.iter().enumerate() {
            let bound = self.vertex_bounds[tri[0]]
                .union(self.vertex_bounds[tri[1]])
                .union(self.vertex_bounds[tri[2]])
                .expanded(thickness);
            self.triangle_bounds[index] = bound;
            self.grid.insert(index, bound.lo, bound.hi);
        }
        for vertex in 0..position.len() {
            let bound = self.vertex_bounds[vertex];
            self.grid.query(
                bound.lo,
                bound.hi,
                &mut self.candidates,
                &mut self.seen,
                vertex + 1,
            );
            for &face in &self.candidates {
                if !self.triangles[face].iter().any(|&v| self.near(vertex, v)) {
                    #[cfg(test)]
                    {
                        self.unfiltered_counts[0] += 1;
                    }
                    if bound.overlaps(self.triangle_bounds[face]) {
                        let tri = self.triangles[face];
                        let ids = [vertex, tri[0], tri[1], tri[2]];
                        if !swept_separated(
                            ids.map(|i| start[i]),
                            ids.map(|i| position[i]),
                            1,
                            thickness,
                        ) {
                            self.vertex_faces.push((vertex, face));
                        }
                    }
                }
            }
        }
        self.seen.fill(0);
        self.grid.clear(cell);
        for (index, edge) in self.edges.iter().enumerate() {
            let bound = self.vertex_bounds[edge[0]]
                .union(self.vertex_bounds[edge[1]])
                .expanded(thickness);
            self.edge_bounds[index] = bound;
            self.grid.insert(index, bound.lo, bound.hi);
        }
        for (first, edge) in self.edges.iter().enumerate() {
            let bound = self.vertex_bounds[edge[0]].union(self.vertex_bounds[edge[1]]);
            self.grid.query_after(
                bound.lo,
                bound.hi,
                &mut self.candidates,
                &mut self.seen,
                first + 1,
                first,
            );
            for &second in &self.candidates {
                if second > first
                    && !edge
                        .iter()
                        .any(|&a| self.edges[second].iter().any(|&b| self.near(a, b)))
                {
                    #[cfg(test)]
                    {
                        self.unfiltered_counts[1] += 1;
                    }
                    if bound.overlaps(self.edge_bounds[second]) {
                        let other = self.edges[second];
                        let ids = [edge[0], edge[1], other[0], other[1]];
                        if !swept_separated(
                            ids.map(|i| start[i]),
                            ids.map(|i| position[i]),
                            2,
                            thickness,
                        ) {
                            self.edge_pairs.push((first, second));
                        }
                    }
                }
            }
        }
    }
}

/// A separating plane for the convex hull of both endpoints proves that the features remain
/// farther apart than thickness throughout linear motion. The closest-point direction is only
/// a candidate axis: all endpoints must pass, so uncertain/degenerate cases keep full CCD.
fn swept_separated(start: [V3; 4], end: [V3; 4], split: usize, thickness: f32) -> bool {
    let weights = if split == 1 {
        let b = closest_triangle(start[0], [start[1], start[2], start[3]]);
        [1., -b[0], -b[1], -b[2]]
    } else {
        let (s, t) = closest_edges(start);
        [1. - s, s, -(1. - t), -t]
    };
    let axis = weighted(start, weights).map(f64::from);
    let norm = axis.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-12 {
        return false;
    }
    let origin = start[0].map(f64::from);
    let mut min_a = f64::INFINITY;
    let mut max_b = f64::NEG_INFINITY;
    for positions in [start, end] {
        for (i, p) in positions.iter().enumerate() {
            let projection: f64 = (0..3)
                .map(|k| (f64::from(p[k]) - origin[k]) * axis[k])
                .sum();
            if i < split {
                min_a = min_a.min(projection);
            } else {
                max_b = max_b.max(projection);
            }
        }
    }
    min_a - max_b > (f64::from(thickness) + 1e-4) * norm
}

#[derive(Clone, Copy)]
struct Contact {
    weights: [f32; 4],
    normal: V3,
}

impl Contact {
    fn project(
        self,
        ids: [usize; 4],
        position: &mut [V3],
        pinned: &[bool],
        thickness: f32,
    ) -> bool {
        let delta = weighted(ids.map(|i| position[i]), self.weights);
        let gap = dot(delta, self.normal);
        if gap >= thickness - thickness * 1e-4 {
            return false;
        }
        let mass: f32 = (0..4)
            .filter(|&i| !pinned[ids[i]])
            .map(|i| self.weights[i].powi(2))
            .sum();
        if mass <= 1e-8 {
            return false;
        }
        let lambda = (thickness - gap) / mass;
        for i in 0..4 {
            if !pinned[ids[i]] {
                let correction = mul(self.normal, lambda * self.weights[i]);
                position[ids[i]] = add(position[ids[i]], correction);
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
struct Impact {
    time: f32,
    contact: Contact,
}

fn vertex_face_contact(start: [V3; 4], end: [V3; 4], thickness: f32) -> Option<Contact> {
    let epsilon = (thickness * 1e-4).max(1e-4);
    if let Some(impact) = vertex_face_impact(start, end, epsilon) {
        return Some(impact.contact);
    }
    let bary = closest_triangle(end[0], [end[1], end[2], end[3]]);
    let weights = [1., -bary[0], -bary[1], -bary[2]];
    let delta = weighted(end, weights);
    if dot(delta, delta) >= thickness * thickness {
        return None;
    }
    let normal = orient_normal(
        delta,
        weighted(start, weights),
        sub(weighted(end, weights), weighted(start, weights)),
        epsilon,
    )?;
    Some(Contact { weights, normal })
}

fn edge_contact(start: [V3; 4], end: [V3; 4], thickness: f32) -> Option<Contact> {
    let epsilon = (thickness * 1e-4).max(1e-4);
    if let Some(impact) = edge_impact(start, end, epsilon) {
        return Some(impact.contact);
    }
    let (s, t) = closest_edges(end);
    let weights = [1. - s, s, -(1. - t), -t];
    let delta = weighted(end, weights);
    if dot(delta, delta) >= thickness * thickness {
        return None;
    }
    let normal = orient_normal(
        delta,
        weighted(start, weights),
        sub(weighted(end, weights), weighted(start, weights)),
        epsilon,
    )?;
    Some(Contact { weights, normal })
}

fn vertex_face_impact(start: [V3; 4], end: [V3; 4], epsilon: f32) -> Option<Impact> {
    // Keep interpolation local too: an accurate cubic root does not help if interpolating a large
    // world coordinate rounds the hit away from a narrow, slanted triangle.
    let origin = start[0];
    let start = start.map(|p| sub(p, origin));
    let end = end.map(|p| sub(p, origin));
    let roots = coplanar_times(start, end);
    if roots.coplanar {
        let time = conservative_time(start, end, epsilon, |p| {
            let b = closest_triangle(p[0], [p[1], p[2], p[3]]);
            length(weighted(p, [1., -b[0], -b[1], -b[2]]))
        })?;
        let p = interpolate(start, end, time);
        let b = closest_triangle(p[0], [p[1], p[2], p[3]]);
        let weights = [1., -b[0], -b[1], -b[2]];
        let before = interpolate(start, end, (time - 0.001).max(0.));
        let delta = weighted(before, weights);
        let velocity = sub(weighted(end, weights), weighted(start, weights));
        let normal = orient_normal(delta, delta, velocity, epsilon)?;
        return Some(Impact {
            time,
            contact: Contact { weights, normal },
        });
    }
    for &time in &roots.times[..roots.len] {
        let p = interpolate(start, end, time);
        let bary = closest_triangle(p[0], [p[1], p[2], p[3]]);
        let weights = [1., -bary[0], -bary[1], -bary[2]];
        if length(weighted(p, weights)) > epsilon * 4. {
            continue;
        }
        let n = cross(sub(p[2], p[1]), sub(p[3], p[1]));
        let before = weighted(interpolate(start, end, (time - 0.001).max(0.)), weights);
        let velocity = sub(weighted(end, weights), weighted(start, weights));
        if time <= 1e-6 && dot(n, velocity).abs() < epsilon * length(n) {
            continue;
        }
        let normal = orient_normal(n, before, velocity, epsilon)?;
        return Some(Impact {
            time,
            contact: Contact { weights, normal },
        });
    }
    None
}

fn edge_impact(start: [V3; 4], end: [V3; 4], epsilon: f32) -> Option<Impact> {
    let origin = start[0];
    let start = start.map(|p| sub(p, origin));
    let end = end.map(|p| sub(p, origin));
    let roots = coplanar_times(start, end);
    if roots.coplanar {
        let time = conservative_time(start, end, epsilon, |p| {
            let (s, t) = closest_edges(p);
            length(weighted(p, [1. - s, s, -(1. - t), -t]))
        })?;
        let p = interpolate(start, end, time);
        let (s, t) = closest_edges(p);
        let weights = [1. - s, s, -(1. - t), -t];
        let before = weighted(interpolate(start, end, (time - 0.001).max(0.)), weights);
        let velocity = sub(weighted(end, weights), weighted(start, weights));
        let normal = orient_normal(before, before, velocity, epsilon)?;
        return Some(Impact {
            time,
            contact: Contact { weights, normal },
        });
    }
    for &time in &roots.times[..roots.len] {
        let p = interpolate(start, end, time);
        let (s, t) = closest_edges(p);
        let weights = [1. - s, s, -(1. - t), -t];
        if length(weighted(p, weights)) > epsilon * 4. {
            continue;
        }
        let n = cross(sub(p[1], p[0]), sub(p[3], p[2]));
        let before = weighted(interpolate(start, end, (time - 0.001).max(0.)), weights);
        let velocity = sub(weighted(end, weights), weighted(start, weights));
        if time <= 1e-6 && dot(n, velocity).abs() < epsilon * length(n) {
            continue;
        }
        let normal = orient_normal(n, before, velocity, epsilon)?;
        return Some(Impact {
            time,
            contact: Contact { weights, normal },
        });
    }
    None
}

fn orient_normal(candidate: V3, before: V3, velocity: V3, epsilon: f32) -> Option<V3> {
    let mut normal = if length(candidate) > epsilon {
        mul(candidate, 1. / length(candidate))
    } else if length(before) > epsilon {
        mul(before, 1. / length(before))
    } else if length(velocity) > epsilon {
        mul(velocity, -1. / length(velocity))
    } else {
        return None;
    };
    let side = dot(normal, before);
    if side < -epsilon || (side.abs() <= epsilon && dot(normal, velocity) > 0.) {
        normal = mul(normal, -1.);
    }
    Some(normal)
}

/// Conservative advancement is reserved for coplanar/degenerate trajectories, for which the
/// cubic vanishes identically. The distance has a Lipschitz bound given by endpoint speeds.
fn conservative_time(
    start: [V3; 4],
    end: [V3; 4],
    epsilon: f32,
    distance: impl Fn([V3; 4]) -> f32,
) -> Option<f32> {
    // Remove common translation before bounding the Hausdorff speed of the two features. The
    // distance is translation invariant; including a large common displacement can exhaust the
    // iteration budget on nearby features that never approach one another.
    let common = sub(end[0], start[0]);
    let speeds = std::array::from_fn::<_, 4, _>(|i| length(sub(sub(end[i], start[i]), common)));
    let speed = speeds.iter().copied().fold(0_f32, f32::max) * 2.;
    if speed <= epsilon {
        return None;
    }
    let mut time = 0.;
    for _ in 0..48 {
        let d = distance(interpolate(start, end, time));
        if d <= epsilon * 2. {
            return Some(time);
        }
        let advance = (d - epsilon) / speed * 0.9;
        if time + advance > 1. {
            return None;
        }
        time += advance;
    }
    // A hard iteration limit must not silently accept a possible crossing.
    Some(time)
}

struct Roots {
    times: [f32; 4],
    len: usize,
    coplanar: bool,
}

fn coplanar_times(start: [V3; 4], end: [V3; 4]) -> Roots {
    let base: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|axis| f64::from(start[i + 1][axis]) - f64::from(start[0][axis]))
    });
    let velocity: [[f64; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|axis| {
            f64::from(end[i + 1][axis]) - f64::from(end[0][axis]) - base[i][axis]
        })
    });
    let triple = |a: [f64; 3], b: [f64; 3], c: [f64; 3]| {
        a[0] * (b[1] * c[2] - b[2] * c[1])
            + a[1] * (b[2] * c[0] - b[0] * c[2])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    };
    let p = [
        triple(base[0], base[1], base[2]),
        triple(velocity[0], base[1], base[2])
            + triple(base[0], velocity[1], base[2])
            + triple(base[0], base[1], velocity[2]),
        triple(base[0], velocity[1], velocity[2])
            + triple(velocity[0], base[1], velocity[2])
            + triple(velocity[0], velocity[1], base[2]),
        triple(velocity[0], velocity[1], velocity[2]),
    ];
    polynomial_roots(p)
}

fn polynomial_roots(p: [f64; 4]) -> Roots {
    let scale = p.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    let mut result = Roots {
        times: [0.; 4],
        len: 0,
        coplanar: scale < 1e-14,
    };
    if result.coplanar {
        return result;
    }
    let tolerance = scale * 1e-10;
    let eval = |t: f64| ((p[3] * t + p[2]) * t + p[1]) * t + p[0];
    let mut cuts = [0., 1., 1., 1.];
    let mut count = 2;
    let (a, b, c) = (3. * p[3], 2. * p[2], p[1]);
    if a.abs() <= tolerance {
        if b.abs() > tolerance {
            let root = -c / b;
            if root > 0. && root < 1. {
                cuts[count] = root;
                count += 1;
            }
        }
    } else {
        let discriminant = b * b - 4. * a * c;
        if discriminant >= 0. {
            let q = -0.5 * (b + discriminant.sqrt().copysign(b));
            for root in [
                q / a,
                if q.abs() > 1e-30 {
                    c / q
                } else {
                    -b / (2. * a)
                },
            ] {
                if root > 0. && root < 1. && count < 4 {
                    cuts[count] = root;
                    count += 1;
                }
            }
        }
    }
    cuts[..count].sort_by(f64::total_cmp);
    let mut push = |time: f64| {
        let time = time as f32;
        if result.len < 4
            && !result.times[..result.len]
                .iter()
                .any(|v| (*v - time).abs() < 1e-6)
        {
            result.times[result.len] = time;
            result.len += 1;
        }
    };
    for i in 0..count {
        if eval(cuts[i]).abs() <= tolerance {
            push(cuts[i]);
        }
        if i + 1 == count || eval(cuts[i]) * eval(cuts[i + 1]) >= 0. {
            continue;
        }
        let (mut lo, mut hi) = (cuts[i], cuts[i + 1]);
        let negative = eval(lo) < 0.;
        for _ in 0..45 {
            let middle = (lo + hi) * 0.5;
            if (eval(middle) < 0.) == negative {
                lo = middle;
            } else {
                hi = middle;
            }
        }
        push((lo + hi) * 0.5);
    }
    result.times[..result.len].sort_by(f32::total_cmp);
    result
}

fn closest_triangle(p: V3, tri: [V3; 3]) -> [f32; 3] {
    let [a, b, c] = tri;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0. && d2 <= 0. {
        return [1., 0., 0.];
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0. && d4 <= d3 {
        return [0., 1., 0.];
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        let t = d1 / (d1 - d3).max(1e-20);
        return [1. - t, t, 0.];
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0. && d5 <= d6 {
        return [0., 0., 1.];
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        let t = d2 / (d2 - d6).max(1e-20);
        return [1. - t, 0., t];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && d4 - d3 >= 0. && d5 - d6 >= 0. {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6)).max(1e-20);
        return [0., 1. - t, t];
    }
    let sum = va + vb + vc;
    if sum.abs() <= 1e-16 {
        // Degenerate triangles still have edges. Find the closest one instead of dividing by zero.
        let mut best = (f32::INFINITY, [1., 0., 0.]);
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            let e = sub(tri[j], tri[i]);
            let t = (dot(sub(p, tri[i]), e) / dot(e, e).max(1e-20)).clamp(0., 1.);
            let distance = length(sub(p, lerp(tri[i], tri[j], t)));
            if distance < best.0 {
                let mut bary = [0.; 3];
                bary[i] = 1. - t;
                bary[j] = t;
                best = (distance, bary);
            }
        }
        return best.1;
    }
    let v = vb / sum;
    let w = vc / sum;
    [1. - v - w, v, w]
}

fn closest_edges(p: [V3; 4]) -> (f32, f32) {
    let d1 = sub(p[1], p[0]);
    let d2 = sub(p[3], p[2]);
    let r = sub(p[0], p[2]);
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    if a <= 1e-16 && e <= 1e-16 {
        return (0., 0.);
    }
    if a <= 1e-16 {
        return (0., (f / e).clamp(0., 1.));
    }
    let c = dot(d1, r);
    if e <= 1e-16 {
        return ((-c / a).clamp(0., 1.), 0.);
    }
    let b = dot(d1, d2);
    let denominator = a * e - b * b;
    let mut s = if denominator > 1e-12 * a * e {
        ((b * f - c * e) / denominator).clamp(0., 1.)
    } else {
        0.
    };
    let mut t = (b * s + f) / e;
    if t < 0. {
        t = 0.;
        s = (-c / a).clamp(0., 1.);
    } else if t > 1. {
        t = 1.;
        s = ((b - c) / a).clamp(0., 1.);
    }
    (s, t)
}

fn interpolate(start: [V3; 4], end: [V3; 4], t: f32) -> [V3; 4] {
    std::array::from_fn(|i| lerp(start[i], end[i], t))
}
fn weighted(p: [V3; 4], weights: [f32; 4]) -> V3 {
    // Contact weights sum to zero. Subtract the common origin before accumulation so a distant
    // monitor origin does not cancel large absolute coordinates at the CCD contact tolerance.
    std::array::from_fn(|axis| (1..4).map(|i| (p[i][axis] - p[0][axis]) * weights[i]).sum())
}
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn mul(a: V3, b: f32) -> V3 {
    a.map(|x| x * b)
}
fn lerp(a: V3, b: V3, t: f32) -> V3 {
    add(a, mul(sub(b, a), t))
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_groups_preserve_fixed_pins_and_unrelated_motion() {
        let start = [
            [0., 0., 3.],
            [1., 0., 3.],
            [0., 1., 3.],
            [1., 1., 3.],
            [8., 8., 3.],
        ];
        let mut requested = start.map(|p| add(p, [2., 0., 1.]));
        requested[0] = start[0];
        let pinned = [true, true, false, false, false];
        let mut groups = ImpactGroups::new(start.len());
        groups.reset(&requested);
        assert!(groups.merge([0, 1, 2, 3], &pinned));
        groups.apply(&start, &mut requested, &pinned);
        assert_eq!(&requested[..4], &start[..4]);
        assert_eq!(requested[4], add(start[4], [2., 0., 1.]));
    }

    #[test]
    fn impact_groups_preserve_common_grab_motion_and_floor_clearance() {
        let start = [[0., 0., 2.], [1., 0., 2.], [0., 1., 1.], [1., 1., 1.]];
        let pinned = [true, true, false, false];
        let mut requested = start.map(|p| add(p, [3., -2., -0.5]));
        requested[2] = [1., 2., 0.];
        let mut groups = ImpactGroups::new(start.len());
        groups.reset(&requested);
        groups.merge([0, 1, 2, 3], &pinned);
        groups.apply(&start, &mut requested, &pinned);
        assert_eq!(requested, start.map(|p| add(p, [3., -2., -0.5])));

        let pinned = [false; 4];
        requested = start.map(|p| add(p, [3., -2., -5.]));
        groups.reset(&requested);
        groups.merge([0, 1, 2, 3], &pinned);
        groups.apply(&start, &mut requested, &pinned);
        assert_eq!(requested, start.map(|p| add(p, [3., -2., -1.])));
    }

    #[test]
    fn repaired_motion_is_rechecked_against_new_contacts() {
        let mut surface = SurfaceCollision::new(2, 2);
        let start = [[0.25, 0.25, 1.], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let pinned = [true, false, false, false];
        let mut requested = start;
        requested[0][2] = -2.;
        surface.solve(&start, &mut requested, &pinned, 0.01);
        assert!(requested.iter().all(|p| p[2] >= 0.));
        assert!(vertex_face_impact(start, requested, 1e-4).is_none());
        assert!(
            surface.candidate_rebuilds >= 3,
            "group repair must rebuild CCD candidates"
        );

        // A moving group is only a candidate: translating an edge into another edge must
        // produce a fresh impact, even if both previous groups were internally rigid.
        let start = [[-1., 2., 0.], [1., 2., 0.], [-1., 0., 0.], [1., 0., 0.]];
        let end = [
            add(start[0], [0., -3., 0.]),
            add(start[1], [0., -3., 0.]),
            start[2],
            start[3],
        ];
        assert!(edge_impact(start, end, 1e-4).is_some());
    }

    #[test]
    fn grid_preserves_keys_order_and_deduplication_across_reuse() {
        let mut grid = Grid::default();
        grid.clear(1.);
        grid.insert(1, [0.; 3], [1., 0., 0.]);
        grid.insert(
            2,
            [GRID_BUCKETS as f32, 0., 0.],
            [GRID_BUCKETS as f32, 0., 0.],
        );
        grid.insert(3, [0.; 3], [0.; 3]);
        let mut result = Vec::new();
        let mut seen = vec![0; 4];
        grid.query([0.; 3], [1., 0., 0.], &mut result, &mut seen, 1);
        assert_eq!(
            result,
            [1, 3],
            "exact keys exclude hash aliases; multi-cell features appear once"
        );
        let allocation = grid.entries.as_ptr();
        grid.clear(1.);
        grid.insert(2, [0.; 3], [0.; 3]);
        grid.query([0.; 3], [0.; 3], &mut result, &mut seen, 2);
        assert_eq!(result, [2]);
        assert_eq!(grid.entries.as_ptr(), allocation);
    }

    #[test]
    fn polynomial_isolates_three_crossings_and_tangent_root() {
        let roots = polynomial_roots([-0.08, 0.66, -1.5, 1.]);
        assert_eq!(roots.len, 3);
        for (actual, expected) in roots.times.iter().zip([0.2, 0.5, 0.8]) {
            assert!((*actual - expected).abs() < 1e-5);
        }
        let roots = polynomial_roots([0.25, -1., 1., 0.]);
        assert_eq!(roots.len, 1);
        assert!((roots.times[0] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn fast_vertex_face_crossing_preserves_the_incoming_side_and_pins() {
        for sign in [-1., 1.] {
            let start = [
                [0.2, 0.2, 10. * sign],
                [0., 0., 0.],
                [1., 0., 0.],
                [0., 1., 0.],
            ];
            let mut end = start;
            end[0][2] = -10. * sign;
            let hit = vertex_face_impact(start, end, 1e-4).expect("sweep must hit");
            assert!((hit.time - 0.5).abs() < 1e-4);
            assert!(
                hit.contact
                    .project([0, 1, 2, 3], &mut end, &[false, true, true, true], 0.1)
            );
            assert!(end[0][2] * sign >= 0.099);
            assert_eq!(&end[1..], &start[1..]);
        }
    }

    #[test]
    fn fast_edge_crossing_is_detected_in_both_directions() {
        for sign in [-1., 1.] {
            let start = [
                [-1., 0., 5. * sign],
                [1., 0., 5. * sign],
                [0., -1., 0.],
                [0., 1., 0.],
            ];
            let mut end = start;
            end[0][2] *= -1.;
            end[1][2] *= -1.;
            let hit = edge_impact(start, end, 1e-4).expect("moving edges must hit");
            assert!((hit.time - 0.5).abs() < 1e-4);
            hit.contact
                .project([0, 1, 2, 3], &mut end, &[false, false, true, true], 0.1);
            assert!(end[0][2] * sign >= 0.099 && end[1][2] * sign >= 0.099);
            assert_eq!(&end[2..], &start[2..]);
        }
    }

    #[test]
    fn contacts_are_invariant_under_a_large_common_translation() {
        let start = [[0.25, 0.5, 10.], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let offset = [100_000., -100_000., 100_000.];
        let shifted = start.map(|p| add(p, offset));
        let weights = [1., -0.2, -0.3, -0.5];
        let reference = weighted(start, weights);
        let actual = weighted(shifted, weights);
        for axis in 0..3 {
            assert!((actual[axis] - reference[axis]).abs() < 1e-6);
        }

        let mut end = start;
        end[0][2] = -10.;
        let mut shifted_end = end.map(|p| add(p, offset));
        let base_hit = vertex_face_impact(start, end, 1e-4).unwrap();
        let shifted_hit = vertex_face_impact(shifted, shifted_end, 1e-4).unwrap();
        assert!((base_hit.time - shifted_hit.time).abs() < 1e-6);
        assert!(length(sub(base_hit.contact.normal, shifted_hit.contact.normal)) < 1e-6);
        let pinned = [false, true, true, true];
        base_hit
            .contact
            .project([0, 1, 2, 3], &mut end, &pinned, 0.125);
        shifted_hit
            .contact
            .project([0, 1, 2, 3], &mut shifted_end, &pinned, 0.125);
        assert!(length(sub(sub(shifted_end[0], offset), end[0])) < 1e-6);
        assert_eq!(&shifted_end[1..], &shifted[1..]);
    }

    #[test]
    fn narrow_offset_triangle_detects_a_nonmidpoint_hit() {
        let third_ulp = f32::from_bits(32768_f32.to_bits() + 3);
        let start = [
            [32770., 105., 5.],
            [32768., 100., 0.],
            [third_ulp, 120., 0.],
            [third_ulp, 100., 20.],
        ];
        let mut end = start;
        end[0] = [32766., 106., 6.];
        let hit =
            vertex_face_impact(start, end, 2e-4).expect("offset hit must survive interpolation");
        assert!((hit.time - 0.49838915).abs() < 1e-6);
    }

    #[test]
    fn close_coplanar_features_do_not_collide_under_common_translation() {
        let edge = [[-1., 1., 0.], [1., 1., 0.], [-1., 0., 0.], [1., 0., 0.]];
        let moved = edge.map(|p| add(p, [100., 0., 0.]));
        assert!(edge_impact(edge, moved, 1e-4).is_none());
        let vertex_face = [[-1., 0.5, 0.], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let moved = vertex_face.map(|p| add(p, [100., 0., 0.]));
        assert!(vertex_face_impact(vertex_face, moved, 1e-4).is_none());
    }

    #[test]
    fn a_terminal_hit_limits_even_a_fully_pinned_requested_step() {
        let mut surface = SurfaceCollision::new(2, 2);
        let start = [[0.25, 0.25, 1.], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mut end = start;
        end[0][2] = 0.;
        surface.solve(&start, &mut end, &[true; 4], 0.01);
        assert_eq!(surface.limited_steps, 1);
        assert_eq!(
            surface.candidate_rebuilds, 1,
            "unchanged pinned contacts reuse the same candidates for final CCD"
        );
        assert!(surface.last_step_fraction > 0.89 && surface.last_step_fraction < 0.9);
        assert!(end[0][2] > 0.1 && end[0][2] < 0.11);
        assert_eq!(&end[1..], &start[1..]);
    }

    #[test]
    fn a_tiny_time_of_impact_keeps_the_previous_safe_positions_exactly() {
        let mut surface = SurfaceCollision::new(2, 2);
        let start = [[0.25, 0.25, 1e-6], [0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let mut end = start;
        end[0][2] = -1.;
        surface.solve(&start, &mut end, &[true; 4], 0.01);
        assert_eq!(surface.limited_steps, 1);
        assert_eq!(surface.last_step_fraction, 0.);
        assert_eq!(end, start);
    }

    #[test]
    fn one_ulp_clearance_at_a_distant_origin_cannot_round_past_a_contact() {
        let mut surface = SurfaceCollision::new(2, 2);
        let plane = 32768_f32;
        let first_safe_x = f32::from_bits(plane.to_bits() + 1);
        let start = [
            [first_safe_x, 5., 5.],
            [plane, 0., 0.],
            [plane, 20., 0.],
            [plane, 0., 20.],
        ];
        let mut end = start;
        end[0][0] = plane - 2.;
        surface.solve(&start, &mut end, &[true; 4], 0.01);
        assert_eq!(surface.limited_steps, 1);
        assert_eq!(surface.last_step_fraction, 0.);
        assert_eq!(end, start);
        assert!(end[0][0] > plane);
    }

    #[test]
    fn coplanar_parallel_edges_and_degenerate_faces_stay_finite() {
        let start = [[-1., 2., 0.], [1., 2., 0.], [-1., 0., 0.], [1., 0., 0.]];
        let mut end = start;
        end[0][1] = -2.;
        end[1][1] = -2.;
        let hit = edge_impact(start, end, 1e-4).expect("parallel sweep must hit");
        assert!((hit.time - 0.5).abs() < 0.001);
        let bary = closest_triangle([1., 0., 1.], [[0.; 3], [1., 0., 0.], [2., 0., 0.]]);
        assert!(bary.iter().all(|x| x.is_finite()));
        assert!((bary.iter().sum::<f32>() - 1.).abs() < 1e-5);
    }

    #[test]
    fn unobstructed_sheet_translation_accepts_the_whole_step() {
        let mut surface = SurfaceCollision::new(8, 6);
        let start: Vec<V3> = (0..48)
            .map(|i| [(i % 8) as f32 * 10., (i / 8) as f32 * 10., 3.])
            .collect();
        let mut end: Vec<V3> = start.iter().map(|&p| add(p, [7., 2., 1.])).collect();
        let expected = end.clone();
        surface.solve(&start, &mut end, &[false; 48], 1.);
        assert_eq!(end, expected);
        assert_eq!(surface.last_step_fraction, 1.);
        assert_eq!(surface.limited_steps, 0);
        assert_eq!(
            surface.candidate_rebuilds, 1,
            "unchanged geometry reuses the final CCD candidate list"
        );
    }

    #[test]
    fn swept_aabbs_keep_touching_features_and_reject_disjoint_bounds() {
        let bound = Aabb::from_motion([0., 0., 0.], [1., 1., 1.]);
        assert!(bound.overlaps(Aabb::from_motion([1., 0., 0.], [2., 1., 1.])));
        assert!(!bound.overlaps(Aabb::from_motion([1.01, 0., 0.], [2., 1., 1.])));
        let sweep = Aabb::from_motion([-10., 0., 0.], [10., 0., 0.]);
        assert!(sweep.overlaps(Aabb::from_motion([0.; 3], [0.; 3])));
    }

    #[test]
    fn swept_separation_rejects_diagonal_neighbors_but_keeps_crossings_and_contact() {
        // AABBs overlap, but a diagonal plane separates the point and triangle at every time.
        let vf = [[9., 9., 0.], [0., 0., 0.], [10., 0., 0.], [0., 10., 0.]];
        let drift = vf.map(|p| add(p, [0.5, 0., 0.]));
        assert!(swept_separated(vf, drift, 1, 1.));
        let mut crossing = drift;
        crossing[0] = [1., 1., 0.];
        assert!(!swept_separated(vf, crossing, 1, 1.));
        let mut touching = vf;
        touching[0] = [5., 5., 1.];
        assert!(!swept_separated(touching, touching, 1, 1.));
        let ee = [[0., 0., 0.], [10., 10., 0.], [0., 4., 0.], [6., 10., 0.]];
        assert!(swept_separated(ee, ee, 2, 1.));
        let crossed = [ee[0], ee[1], [0., -4., 0.], [6., 2., 0.]];
        assert!(!swept_separated(ee, crossed, 2, 1.));
        let far = |p| add(p, [80000., -40000., 20000.]);
        assert!(swept_separated(vf.map(far), drift.map(far), 1, 1.));
        assert!(!swept_separated(vf.map(far), crossing.map(far), 1, 1.));
        assert!(!swept_separated([[0.; 3]; 4], [[0.; 3]; 4], 2, 1.));
    }

    #[test]
    fn actual_swept_aabbs_filter_bucket_neighbors_and_reuse_storage() {
        let mut surface = SurfaceCollision::new(12, 8);
        let start: Vec<V3> = (0..96)
            .map(|i| [(i % 12) as f32 * 10., (i / 12) as f32 * 10., 3.])
            .collect();
        let end: Vec<V3> = start.iter().map(|&p| add(p, [0.5, 0.5, 0.])).collect();
        surface.find_candidates(&start, &end, 1.);
        let unfiltered = surface.unfiltered_counts;
        let filtered = [surface.vertex_faces.len(), surface.edge_pairs.len()];
        eprintln!(
            "bucket candidates VF={} EE={}; actual AABB VF={} EE={}",
            unfiltered[0], unfiltered[1], filtered[0], filtered[1]
        );
        assert!(filtered[0] * 2 < unfiltered[0]);
        assert!(filtered[1] * 2 < unfiltered[1]);
        let pointers = [
            surface.vertex_bounds.as_ptr(),
            surface.triangle_bounds.as_ptr(),
            surface.edge_bounds.as_ptr(),
        ];
        let expected_vf = surface.vertex_faces.clone();
        let expected_ee = surface.edge_pairs.clone();
        surface.find_candidates(&start, &end, 1.);
        assert_eq!(
            pointers,
            [
                surface.vertex_bounds.as_ptr(),
                surface.triangle_bounds.as_ptr(),
                surface.edge_bounds.as_ptr()
            ]
        );
        assert_eq!(surface.vertex_faces, expected_vf);
        assert_eq!(surface.edge_pairs, expected_ee);
    }

    #[test]
    fn ee_query_cursors_match_the_original_order_with_aliases_and_insertions() {
        let mut grid = Grid::default();
        grid.clear(1.);
        let far = GRID_BUCKETS as f32;
        assert_eq!(
            Grid::hash([0, 0, 0]),
            Grid::hash([GRID_BUCKETS as i32, 0, 0])
        );
        grid.insert(0, [0.; 3], [2., 1., 1.]);
        grid.insert(1, [far, 0., 0.], [far + 2., 1., 1.]);
        grid.insert(2, [1., 0., 0.], [3., 1., 1.]);
        grid.insert(3, [0.; 3], [2., 1., 1.]);
        grid.insert(3, [far, 0., 0.], [far + 2., 1., 1.]);
        grid.insert(4, [0.5; 3], [1.5; 3]);
        let mut normal_seen = [0; 6];
        let mut cursor_seen = [0; 6];
        let mut normal = Vec::new();
        let mut cursor = Vec::new();
        for (i, after) in [0, 1, 2, 3, 1, 4].into_iter().enumerate() {
            let shift = if i % 2 == 0 { 0. } else { far };
            let (lo, hi) = ([shift, 0., 0.], [shift + 2., 1., 1.]);
            grid.query(lo, hi, &mut normal, &mut normal_seen, i + 1);
            normal.retain(|&feature| feature > after);
            grid.query_after(lo, hi, &mut cursor, &mut cursor_seen, i + 1, after);
            assert_eq!(cursor, normal, "query {i}, after {after}");
        }
        // Appending after cursor queries must leave ordinary query heads and future EE lookups
        // intact, even after a bucket cursor has advanced to the end of its linked list.
        grid.insert(5, [0.; 3], [2., 1., 1.]);
        grid.query([0.; 3], [2., 1., 1.], &mut normal, &mut normal_seen, 7);
        assert!(normal.contains(&0) && normal.contains(&5));
        normal.retain(|&feature| feature > 2);
        grid.query_after([0.; 3], [2., 1., 1.], &mut cursor, &mut cursor_seen, 7, 2);
        assert_eq!(cursor, normal);
    }
}
