//! Direct port of Rendering/Lighting.swift. Coordinates: x right, y down, z up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightMode {
    Auto,
    Day,
    Evening,
    Night,
}
impl LightMode {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Day, Self::Evening, Self::Night];
    pub fn key(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Day => "day",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.key() == value)
    }
    pub fn environment(self) -> LightEnv {
        let hour = match self {
            Self::Auto => {
                // SAFETY: GetLocalTime writes a fully sized stack value and changes no system state.
                let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
                f64::from(time.wHour) + f64::from(time.wMinute) / 60.
            }
            Self::Day => 12.5,
            Self::Evening => 18.,
            Self::Night => 23.,
        };
        LightEnv::at_hour(hour)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightEnv {
    pub light: [f32; 4],
    pub tint: [f32; 4],
    pub shadow: [f32; 4],
    pub fold_contrast: f32,
}
impl LightEnv {
    pub const NEUTRAL: Self = Self {
        light: [-0.45, -0.55, 0.75, 1.],
        tint: [1.; 4],
        shadow: [0.5, 0.7, 1.5, 3.],
        fold_contrast: 1.,
    };
    pub fn at_hour(hour: f64) -> Self {
        let elevation = if hour > 6. && hour < 19. {
            (std::f64::consts::PI * (hour - 6.) / 13.).sin()
        } else {
            -0.3
        };
        let smooth = |a: f64, b: f64, x: f64| {
            let t = ((x - a) / (b - a)).clamp(0., 1.);
            t * t * (3. - 2. * t)
        };
        let mix = |a: [f64; 3], b: [f64; 3], t: f64| {
            std::array::from_fn::<_, 3, _>(|i| a[i] + (b[i] - a[i]) * t)
        };
        let day = smooth(-0.02, 0.22, elevation);
        let azimuth = ((hour - 12.5) / 6.5).clamp(-1., 1.);
        let sun = [azimuth * 0.95, -0.35, elevation.max(0.2) * 1.2];
        let direction = mix([-0.35, -0.5, 0.8], sun, day);
        let length = direction.iter().map(|x| x * x).sum::<f64>().sqrt();
        let light = direction.map(|x| x / length);
        let warmth = (1. - (elevation / 0.55).clamp(0., 1.)) * day;
        let day_tint = mix([1., 0.99, 0.96], [1., 0.80, 0.58], warmth);
        let tint = mix([0.50, 0.58, 0.84], day_tint, day);
        let sx = (-light[0] / light[2] * 0.55).clamp(-1.7, 1.7);
        let sy = (-light[1] / light[2] * 0.55).clamp(-1.7, 1.7);
        Self {
            light: [
                light[0] as f32,
                light[1] as f32,
                light[2] as f32,
                (0.3 + 0.7 * day) as f32,
            ],
            tint: [
                tint[0] as f32,
                tint[1] as f32,
                tint[2] as f32,
                (0.45 + 0.55 * day) as f32,
            ],
            shadow: [
                sx as f32,
                sy as f32,
                (sx * 2.6 + 0.4) as f32,
                (sy * 2.6 + 2.) as f32,
            ],
            fold_contrast: 1.,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sun_moves_and_night_is_cool() {
        let morning = LightEnv::at_hour(8.);
        let noon = LightEnv::at_hour(12.5);
        let evening = LightEnv::at_hour(18.);
        let night = LightEnv::at_hour(23.);
        assert!(morning.light[0] < 0. && evening.light[0] > 0.);
        assert!(noon.light[2] > evening.light[2]);
        assert!(night.tint[2] > night.tint[0] && night.light[3] < noon.light[3]);
        for hour in 0..=1440 {
            let e = LightEnv::at_hour(f64::from(hour) / 60.);
            assert!(
                e.light
                    .iter()
                    .chain(e.tint.iter())
                    .chain(e.shadow.iter())
                    .all(|v| v.is_finite())
            );
        }
    }
}
