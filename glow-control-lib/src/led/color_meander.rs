use crate::led::led_color::LedColor;
use rand::Rng;
use std::f64::consts::PI;

pub enum MeanderStyle {
    Sphere,
    Cylinder,
    Surface,
}

pub struct ColorMeander {
    step_length: f64,
    noise_level: f64,
    xyz: (f64, f64, f64),
    dir: (f64, f64, f64),
    style: MeanderStyle,
}

impl ColorMeander {
    pub fn new(style: MeanderStyle, speed: f64, noise: f64, start: (f64, f64, f64)) -> Self {
        let mut rng = rand::thread_rng();
        let dir = (
            rng.gen_range(-0.5..0.5),
            rng.gen_range(-0.5..0.5),
            rng.gen_range(-0.5..0.5) - start.2,
        );
        ColorMeander {
            step_length: speed,
            noise_level: noise,
            xyz: start,
            dir,
            style,
        }
    }

    fn normalize(&self, vec: (f64, f64, f64)) -> (f64, f64, f64) {
        let nrm = (vec.0 * vec.0 + vec.1 * vec.1 + vec.2 * vec.2).sqrt();
        if nrm == 0.0 {
            (0.0, 0.0, 0.0)
        } else {
            (vec.0 / nrm, vec.1 / nrm, vec.2 / nrm)
        }
    }

    fn xyz_to_hsl(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        match self.style {
            MeanderStyle::Cylinder => {
                let h = y.atan2(x) / (2.0 * PI) + 0.5;
                let s = (x * x + y * y).sqrt().min(1.0);
                let l = z;
                (h, s, l)
            }
            _ => {
                let h = y.atan2(x) / (2.0 * PI) + 0.5;
                let l = z.asin() * 2.0 / PI;
                let r = (x * x + y * y).sqrt();
                let r0 = (1.0 - z * z).sqrt();
                let s = if r0 > 0.0 { r / r0 } else { 0.0 }.min(1.0);
                (h, s, l)
            }
        }
    }
    fn xyz_color(&self, x: f64, y: f64, z: f64, led_color: &LedColor) -> (u8, u8, u8) {
        let (h, s, l) = self.xyz_to_hsl(x, y, z);
        led_color.hsl_color(h, s, l)
    }

    pub fn get(&self, led_color: &LedColor) -> (u8, u8, u8) {
        self.xyz_color(self.xyz.0, self.xyz.1, self.xyz.2, led_color)
    }

    pub fn get_compl(&self, led_color: &LedColor) -> (u8, u8, u8) {
        self.xyz_color(-self.xyz.0, -self.xyz.1, self.xyz.2, led_color)
    }

    pub fn get_xyz(&self) -> (f64, f64, f64) {
        self.xyz
    }

    pub fn get_hsl(&self) -> (f64, f64, f64) {
        self.xyz_to_hsl(self.xyz.0, self.xyz.1, self.xyz.2)
    }

    pub fn step(&mut self) {
        let mut rng = rand::thread_rng();
        let (mut nx, mut ny, mut nz) = (
            self.xyz.0 + self.dir.0 * self.step_length,
            self.xyz.1 + self.dir.1 * self.step_length,
            self.xyz.2 + self.dir.2 * self.step_length,
        );

        let (mut ndir_x, mut ndir_y, mut ndir_z) = self.dir;
        match self.style {
            MeanderStyle::Cylinder => {
                nz = nz.clamp(-1.0, 1.0);
                let nrm = (nx * nx + ny * ny).sqrt();
                if nrm > 1.0 {
                    nx /= nrm;
                    ny /= nrm;
                    self.dir = self.normalize((nx - self.xyz.0, ny - self.xyz.1, nz - self.xyz.2));
                }
                ndir_x += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_y += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_z += rng.gen_range(-self.noise_level..self.noise_level);
                if (nz + ndir_z).abs() > 1.0 {
                    let sgn = if nz + ndir_z > 0.0 { 1.0 } else { -1.0 };
                    let delta = (1.0 - (sgn - nz).powi(2)).sqrt();
                    let nrm = (ndir_x * ndir_x + ndir_y * ndir_y).sqrt();
                    ndir_x = ndir_x * delta / nrm;
                    ndir_y = ndir_y * delta / nrm;
                    ndir_z = sgn - nz;
                }
            }
            MeanderStyle::Surface => {
                let nrm = (nx * nx + ny * ny + nz * nz).sqrt();
                nx /= nrm;
                ny /= nrm;
                nz /= nrm;
                self.dir = self.normalize((nx - self.xyz.0, ny - self.xyz.1, nz - self.xyz.2));
                ndir_x += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_y += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_z += rng.gen_range(-self.noise_level..self.noise_level);
            }
            _ => {
                let nrm = (nx * nx + ny * ny + nz * nz).sqrt();
                if nrm > 1.0 {
                    nx /= nrm * nrm;
                    ny /= nrm * nrm;
                    nz /= nrm * nrm;
                    self.dir = self.normalize((nx - self.xyz.0, ny - self.xyz.1, nz - self.xyz.2));
                }
                ndir_x += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_y += rng.gen_range(-self.noise_level..self.noise_level);
                ndir_z += rng.gen_range(-self.noise_level..self.noise_level);
            }
        }

        let nrm = (ndir_x * ndir_x + ndir_y * ndir_y + ndir_z * ndir_z).sqrt();
        if nrm != 0.0 {
            self.dir = (ndir_x / nrm, ndir_y / nrm, ndir_z / nrm);
        }

        self.xyz = (nx, ny, nz);
    }
}
