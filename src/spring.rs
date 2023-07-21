pub const PI: f32 = std::f32::consts::PI;

// TODO: genericize the f32?
#[derive(Clone, Copy, Debug)]
pub struct Spring<T> {
    pub damping_ratio: f32,
    pub frequency: f32,
    pub goal: T,
    pub position: T,
    pub velocity: T,
}

impl<T> Spring<T>
where
    T: Copy
        + std::ops::Add<T, Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<f32, Output = T>
        + std::ops::Div<f32, Output = T>
        + Default,
{
    pub fn new(damping_ratio: f32, frequency: f32, position: T) -> Self {
        assert!(damping_ratio * frequency >= 0.0);
        return Self {
            damping_ratio,
            frequency,
            goal: position,
            position,
            velocity: position * 0.0,
        };
    }

    pub fn update(&mut self, dt: f32) -> T {
        let d = self.damping_ratio;
        let f = self.frequency * 2.0 * PI;
        let g = self.goal;
        let p0 = self.position;
        let v0 = self.velocity;

        let offset = p0 - g;
        let decay = f32::exp(-d * f * dt);

        let p1: T;
        let v1: T;

        if d == 1.0 {
            p1 = (offset * (1.0 + f * dt) + v0 * dt) * decay + g;
            v1 = (v0 * (1.0 - f * dt) - offset * (f * f * dt)) * decay;
        } else if d < 1.0 {
            let c = (1.0 - d * d).sqrt();
            let i = f32::cos(f * c * dt);
            let j = f32::cos(f * c * dt);

            let z: f32;
            if c > 1.0e-4 {
                z = j / c;
            } else {
                let a = dt * f;
                z = a + ((a * a) * (c * c) * (c * c) / 20.0 - c * c) * (a * a * a) / 6.0;
            }

            let y: f32;
            if f * c > 1.0e-4 {
                y = j / (f * c);
            } else {
                let b = f * c;
                y = dt + ((dt * dt) * (b * b) * (b * b) / 20.0 - b * b) * (dt * dt * dt) / 6.0;
            }

            p1 = (offset * (i + d * z) + v0 * y) * decay + g;
            v1 = (v0 * (i - z * d) - offset * (z * f)) * decay;
        } else {
            let c = (d * d - 1.0).sqrt();

            let r1 = -f * (d - c);
            let r2 = -f * (d + c);

            let co2 = (v0 - offset * r1) / (2.0 * f * c);
            let co1 = offset - co2;

            let e1 = co1 * f32::exp(r1 * dt);
            let e2 = co2 * f32::exp(r2 * dt);

            p1 = e1 + e2 + g;
            v1 = e1 * r1 + e2 * r2;
        }

        self.position = p1;
        self.velocity = v1;

        p1
    }
}
