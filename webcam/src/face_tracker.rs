struct Kalman1D {
    x: f32,           // Position
    v: f32,           // Velocity
    p: [[f32; 2]; 2], // Covariance
    q: f32,           // Process noise
    r: f32,           // Measurement noise
}

impl Kalman1D {
    fn new(q: f32, r: f32) -> Self {
        Kalman1D {
            x: 0.0,
            v: 0.0,
            p: [[1000.0, 0.0], [0.0, 1000.0]],
            q,
            r,
        }
    }

    fn predict(&mut self, dt: f32) {
        // state: x += v*dt
        self.x += self.v * dt;
        // covariance: P = F P Fᵀ + Q
        let p = self.p;
        self.p[0][0] = p[0][0] + dt * (p[1][0] + p[0][1]) + dt * dt * p[1][1] + self.q;
        self.p[0][1] = p[0][1] + dt * p[1][1];
        self.p[1][0] = p[1][0] + dt * p[1][1];
        self.p[1][1] = p[1][1] + self.q;
    }

    fn update(&mut self, z: f32) {
        // innovation
        let y = z - self.x;
        let s = self.p[0][0] + self.r;
        let k0 = self.p[0][0] / s; // gain for position
        let k1 = self.p[1][0] / s; // gain for velocity
        self.x += k0 * y;
        self.v += k1 * y;
        let p = self.p;
        self.p[0][0] = (1.0 - k0) * p[0][0];
        self.p[0][1] = (1.0 - k0) * p[0][1];
        self.p[1][0] = p[1][0] - k1 * p[0][0];
        self.p[1][1] = p[1][1] - k1 * p[0][1];
    }

    fn gate_distance(&self) -> f32 {
        // uncertainty-scaled gate radius; widens when target is lost
        (self.p[0][0] + self.r).sqrt()
    }
}

pub struct FaceTracker {
    kx: Kalman1D,
    ky: Kalman1D,
    last_w: f32,
    last_h: f32,
    misses: u32,
    initialized: bool,
}

impl FaceTracker {
    pub fn new() -> Self {
        FaceTracker {
            kx: Kalman1D::new(50.0, 25.0),
            ky: Kalman1D::new(50.0, 25.0),
            last_w: 0.0,
            last_h: 0.0,
            misses: 0,
            initialized: false,
        }
    }

    // detections: Vec of (cx, cy, w, h). Returns chosen center if locked.
    pub fn step(
        &mut self,
        detections: &[(f32, f32, f32, f32)],
        dt: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        if !self.initialized {
            // initial lock: largest + central — score however you like
            let best = detections
                .iter()
                .max_by(|a, b| (a.2 * a.3).partial_cmp(&(b.2 * b.3)).unwrap())?;
            self.kx.x = best.0;
            self.ky.x = best.1;
            self.last_w = best.2;
            self.last_h = best.3;
            self.initialized = true;
            self.misses = 0;
            return Some(*best);
        }

        self.kx.predict(dt);
        self.ky.predict(dt);

        // gate against predicted position
        let gate = 3.0 * (self.kx.gate_distance() + self.ky.gate_distance());
        let pred = (self.kx.x, self.ky.x);

        let matched = detections
            .iter()
            .map(|d| {
                let dx = d.0 - pred.0;
                let dy = d.1 - pred.1;
                (d, (dx * dx + dy * dy).sqrt())
            })
            .filter(|(_, dist)| *dist < gate)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        match matched {
            Some((d, _)) => {
                self.kx.update(d.0);
                self.ky.update(d.1);
                self.last_w = d.2;
                self.last_h = d.3;
                self.misses = 0;
                Some((self.kx.x, self.ky.x, self.last_w, self.last_h))
            }
            None => {
                // no match: coast on prediction, widen gate via growing covariance
                self.misses += 1;
                if self.misses > 30 {
                    self.initialized = false; // lost — re-select next frame
                    None
                } else {
                    Some((self.kx.x, self.ky.x, self.last_w, self.last_h))
                }
            }
        }
    }
}
