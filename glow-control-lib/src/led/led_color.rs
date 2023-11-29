use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy)]
pub enum ColorStyle {
    Col3,
    Col4,
    Col6,
    Col8,
    Col10,
}

#[derive(Debug, Clone, Copy)]
pub enum LightnessPolicy {
    Linear,
    Equilight,
}

#[derive(Debug)]
pub struct ColorModel {
    color_style: ColorStyle,
    lightness_policy: LightnessPolicy,
}

impl Default for ColorModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorModel {
    pub fn new() -> Self {
        ColorModel {
            color_style: ColorStyle::Col8,
            lightness_policy: LightnessPolicy::Equilight,
        }
    }

    pub fn set_color_style(&mut self, style: &str) -> Result<()> {
        match style {
            "3col" => self.color_style = ColorStyle::Col3,
            "4col" => self.color_style = ColorStyle::Col4,
            "6col" => self.color_style = ColorStyle::Col6,
            "8col" => self.color_style = ColorStyle::Col8,
            "10col" => self.color_style = ColorStyle::Col10,
            "linear" => self.lightness_policy = LightnessPolicy::Linear,
            "equilight" => self.lightness_policy = LightnessPolicy::Equilight,
            _ => return Err(anyhow!("Invalid color style or lightness policy")),
        }
        Ok(())
    }

    pub fn get_color_style(&self) -> (ColorStyle, LightnessPolicy) {
        (self.color_style, self.lightness_policy)
    }
}

pub struct LedColor {
    gamma: f64,
    brightness: Vec<f64>,
    balance: Vec<f64>,
    col_style: ColorModel,
}

impl Default for LedColor {
    fn default() -> Self {
        Self::new()
    }
}

impl LedColor {
    pub fn new() -> Self {
        LedColor {
            gamma: 1.0,
            brightness: vec![0.35, 0.50, 0.15],
            balance: vec![0.9, 1.0, 0.6],
            col_style: ColorModel::new(),
        }
    }

    pub fn color_gamma(&self, x: f64) -> f64 {
        if self.gamma == 1.0 {
            x
        } else {
            x.powf(self.gamma)
        }
    }

    pub fn inv_color_gamma(&self, x: f64) -> f64 {
        if self.gamma == 1.0 {
            x
        } else {
            x.powf(1.0 / self.gamma)
        }
    }

    pub fn color_gamma_image(x: f64) -> f64 {
        if x > 0.003_130_8 {
            (x.powf(1.0 / 2.4) * 1.055) - 0.055
        } else {
            x * 12.92
        }
    }

    pub fn inv_color_gamma_image(x: f64) -> f64 {
        if x > 0.040_45 {
            ((x + 0.055) / 1.055).powf(2.4)
        } else {
            x / 12.92
        }
    }

    pub fn color_brightness(&self, r: f64, g: f64, b: f64) -> f64 {
        [r, g, b]
            .iter()
            .zip(self.brightness.iter())
            .map(|(&c, &br)| c * br)
            .sum()
    }

    pub fn rgb_color(&self, r: f64, g: f64, b: f64) -> (u8, u8, u8) {
        let rgb = [r, g, b]
            .iter()
            .zip(self.balance.iter())
            .map(|(&c, &bal)| {
                let value = (255.0 * bal * self.color_gamma(c))
                    .round()
                    .clamp(0.0, 255.0);
                value as u8
            })
            .collect::<Vec<u8>>();
        (rgb[0], rgb[1], rgb[2])
    }

    pub fn image_to_led_rgb(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let rgb = [r, g, b]
            .iter()
            .zip(self.balance.iter())
            .map(|(&c, &bal)| {
                let value =
                    (255.0 * bal * self.color_gamma(Self::inv_color_gamma_image(c as f64 / 255.0)))
                        .round()
                        .clamp(0.0, 255.0);
                value as u8
            })
            .collect::<Vec<u8>>();
        (rgb[0], rgb[1], rgb[2])
    }

    pub fn led_to_image_rgb(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        let rgb = [r, g, b]
            .iter()
            .zip(self.balance.iter())
            .map(|(&c, &bal)| {
                let value = (255.0
                    * Self::color_gamma_image(self.inv_color_gamma(c as f64 / (bal * 255.0))))
                .round()
                .clamp(0.0, 255.0);
                value as u8
            })
            .collect::<Vec<u8>>();
        (rgb[0], rgb[1], rgb[2])
    }

    pub fn hsl_color(&self, h: f64, s: f64, l: f64) -> (u8, u8, u8) {
        let hramp = match self.col_style.color_style {
            ColorStyle::Col3 => vec![
                0.0,
                1.0 / 6.0,
                2.0 / 6.0,
                3.0 / 6.0,
                4.0 / 6.0,
                5.0 / 6.0,
                1.0,
            ],
            ColorStyle::Col4 => vec![
                0.0,
                1.0 / 8.0,
                1.0 / 4.0,
                2.0 / 4.0,
                3.0 / 4.0,
                7.0 / 8.0,
                1.0,
            ],
            ColorStyle::Col6 => vec![
                0.0,
                1.0 / 12.0,
                1.0 / 6.0,
                1.0 / 3.0,
                2.0 / 3.0,
                3.0 / 4.0,
                1.0,
            ],
            ColorStyle::Col8 => vec![
                0.0,
                1.0 / 8.0,
                2.0 / 8.0,
                3.0 / 8.0,
                5.0 / 8.0,
                6.0 / 8.0,
                1.0,
            ],
            ColorStyle::Col10 => vec![
                0.0,
                2.0 / 10.0,
                3.0 / 10.0,
                4.0 / 10.0,
                7.0 / 10.0,
                8.0 / 10.0,
                1.0,
            ],
        };

        let balance = &self.balance;
        let (ir, ig, ib) = (1.0 / balance[0], 1.0 / balance[1], 1.0 / balance[2]);
        let (irg, irb, igb) = (ir.min(ig), ir.min(ib), ig.min(ib));
        let iramp = [
            (0.0, 0.0, ib),
            (0.0, igb / 2.0, igb / 2.0),
            (0.0, ig, 0.0),
            (irg / 2.0, irg / 2.0, 0.0),
            (ir, 0.0, 0.0),
            (irb / 2.0, 0.0, irb / 2.0),
            (0.0, 0.0, ib),
        ];

        let mut i = 0;
        while h > hramp[i + 1] {
            i += 1;
        }
        let p = (h - hramp[i]) / (hramp[i + 1] - hramp[i]);
        let (r, g, b) = {
            let (x1, y1, z1) = iramp[i];
            let (x2, y2, z2) = iramp[i + 1];
            (p * (x2 - x1) + x1, p * (y2 - y1) + y1, p * (z2 - z1) + z1)
        };

        let nrm = r / ir.max(g / ig).max(b / ib);
        let (r, g, b) = (r / nrm, g / nrm, b / nrm);
        let ll = (l + 1.0) * 0.5;
        let (t1, t2) = match self.col_style.lightness_policy {
            LightnessPolicy::Linear => {
                if ll < 0.5 {
                    (l + 1.0, 0.0)
                } else {
                    (1.0 - l, l)
                }
            }
            LightnessPolicy::Equilight => {
                let br = self.color_brightness(r, g, b);
                let e = r.max(g).max(b);
                let p = 1.0_f64
                    .min((1.0 - ll / e) / (1.0 - br))
                    .min((1.0 - ll * balance[1]) / (1.0 - self.brightness[1]));
                let t1 = ll * p / ((br - e) * p + e);
                let t2 = (ll - t1 * br).max(0.0);
                (t1, t2)
            }
        };

        let t1 = s * t1;
        let t2 = s * t2 + ll * (1.0 - s);
        self.rgb_color(r * t1 + t2, g * t1 + t2, b * t1 + t2)
    }

    // ... Any additional methods needed.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts;

    #[test]
    fn test_color_gamma_no_correction() {
        let led_color = LedColor::new();
        assert_eq!(led_color.color_gamma(0.5), 0.5);
    }

    #[test]
    fn test_color_gamma_less_than_one() {
        let mut led_color = LedColor::new();
        led_color.gamma = 0.5; // Gamma less than 1.0
        assert!((led_color.color_gamma(0.5) - consts::FRAC_1_SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn test_color_gamma_greater_than_one() {
        let mut led_color = LedColor::new();
        led_color.gamma = 2.0; // Gamma greater than 1.0
        assert_eq!(led_color.color_gamma(0.5), 0.25);
    }

    #[test]
    fn test_color_gamma_edge_cases() {
        let mut led_color = LedColor::new();
        led_color.gamma = 2.0;
        assert_eq!(led_color.color_gamma(0.0), 0.0);
        assert_eq!(led_color.color_gamma(1.0), 1.0);
    }

    fn test_inv_color_gamma_no_correction() {
        let led_color = LedColor::new();
        assert_eq!(led_color.inv_color_gamma(0.5), 0.5);
    }

    #[test]
    fn test_inv_color_gamma_less_than_one() {
        let mut led_color = LedColor::new();
        led_color.gamma = 0.5; // Gamma less than 1.0
        assert!((led_color.inv_color_gamma(consts::FRAC_1_SQRT_2) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_inv_color_gamma_greater_than_one() {
        let mut led_color = LedColor::new();
        led_color.gamma = 2.0; // Gamma greater than 1.0
        assert!((led_color.inv_color_gamma(0.25) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_inv_color_gamma_edge_cases() {
        let mut led_color = LedColor::new();
        led_color.gamma = 2.0;
        assert_eq!(led_color.inv_color_gamma(0.0), 0.0);
        assert_eq!(led_color.inv_color_gamma(1.0), 1.0);
    }

    #[test]
    fn test_color_gamma_image_standard_values() {
        let input: f64 = 0.5;
        let expected = if input > 0.0031308 {
            (input.powf(1.0 / 2.4) * 1.055) - 0.055
        } else {
            input * 12.92
        };
        assert!((LedColor::color_gamma_image(input) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_color_gamma_image_low_values() {
        assert_eq!(LedColor::color_gamma_image(0.003), 0.003 * 12.92);
    }

    #[test]
    fn test_color_gamma_image_high_values() {
        assert!(
            (LedColor::color_gamma_image(0.5) - ((0.5_f64.powf(1.0 / 2.4) * 1.055) - 0.055)).abs()
                < 1e-10
        );
    }

    #[test]
    fn test_color_gamma_image_edge_cases() {
        assert_eq!(LedColor::color_gamma_image(0.0), 0.0);
        assert!(
            (LedColor::color_gamma_image(1.0) - ((1.0_f64.powf(1.0 / 2.4) * 1.055) - 0.055)).abs()
                < 1e-10
        );
    }

    #[test]
    fn test_inv_color_gamma_image_standard_values() {
        let input = 0.21404114048223255_f64;
        let expected = if input > 0.04045 {
            ((input + 0.055) / 1.055).powf(2.4)
        } else {
            input / 12.92
        };
        assert!((LedColor::inv_color_gamma_image(input) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_inv_color_gamma_image_low_values() {
        let input = 0.003;
        let expected = input / 12.92;
        assert_eq!(LedColor::inv_color_gamma_image(input), expected);
    }

    #[test]
    fn test_inv_color_gamma_image_high_values() {
        let input: f64 = 0.5;
        let expected = ((input + 0.055) / 1.055).powf(2.4);
        assert!((LedColor::inv_color_gamma_image(input) - expected).abs() < 1e-10);
    }

    #[test]
    fn test_inv_color_gamma_image_edge_cases() {
        assert_eq!(LedColor::inv_color_gamma_image(0.0), 0.0);
        let max_input: f64 = 1.0;
        let expected_max = ((max_input + 0.055) / 1.055).powf(2.4);
        assert!((LedColor::inv_color_gamma_image(max_input) - expected_max).abs() < 1e-10);
    }

    #[test]
    fn test_color_brightness_typical_values() {
        let led_color = LedColor::new();
        let r = 0.5;
        let g = 0.5;
        let b = 0.5;
        let expected_brightness: f64 = led_color
            .brightness
            .iter()
            .zip([r, g, b].iter())
            .map(|(br, &c)| br * c)
            .sum();
        assert_eq!(led_color.color_brightness(r, g, b), expected_brightness);
    }

    #[test]
    fn test_color_brightness_extreme_values() {
        let led_color = LedColor::new();
        assert_eq!(led_color.color_brightness(0.0, 0.0, 0.0), 0.0);
        let max_brightness = led_color.brightness.iter().sum::<f64>();
        assert_eq!(led_color.color_brightness(1.0, 1.0, 1.0), max_brightness);
    }

    #[test]
    fn test_color_brightness_varying_brightness() {
        let mut led_color = LedColor::new();
        led_color.brightness = vec![0.1, 0.2, 0.3]; // Custom brightness coefficients
        let r = 0.5;
        let g = 0.5;
        let b = 0.5;
        let expected_brightness: f64 = led_color
            .brightness
            .iter()
            .zip([r, g, b].iter())
            .map(|(br, &c)| br * c)
            .sum();
        assert_eq!(led_color.color_brightness(r, g, b), expected_brightness);
    }

    #[test]
    fn test_rgb_color_typical_values() {
        let led_color = LedColor::new();
        let r = 0.5;
        let g = 0.5;
        let b = 0.5;
        let expected = led_color.rgb_color(r, g, b);
        assert_eq!(expected, (115, 128, 77)); // Expected values calculated from the function logic
    }

    #[test]
    fn test_rgb_color_extreme_values() {
        let led_color = LedColor::new();
        assert_eq!(led_color.rgb_color(0.0, 0.0, 0.0), (0, 0, 0));
        assert_eq!(led_color.rgb_color(1.0, 1.0, 1.0), (230, 255, 153)); // Expected values calculated from the function logic
    }

    #[test]
    fn test_rgb_color_varying_balance() {
        let mut led_color = LedColor::new();
        led_color.balance = vec![1.0, 1.0, 1.0]; // Custom balance coefficients for simplicity
        let r = 0.5;
        let g = 0.5;
        let b = 0.5;
        let expected = led_color.rgb_color(r, g, b);
        assert_eq!(expected, (128, 128, 128)); // With equal balance, the result should be a shade of grey
    }

    #[test]
    fn test_image_to_led_rgb_typical_values() {
        let led_color = LedColor::new();
        let r = 128;
        let g = 128;
        let b = 128;
        let expected = led_color.image_to_led_rgb(r, g, b);

        // Calculate the expected values using the same logic as the function
        let expected_r = (255.0
            * led_color.balance[0]
            * led_color.color_gamma(LedColor::inv_color_gamma_image(r as f64 / 255.0)))
        .round() as u8;
        let expected_g = (255.0
            * led_color.balance[1]
            * led_color.color_gamma(LedColor::inv_color_gamma_image(g as f64 / 255.0)))
        .round() as u8;
        let expected_b = (255.0
            * led_color.balance[2]
            * led_color.color_gamma(LedColor::inv_color_gamma_image(b as f64 / 255.0)))
        .round() as u8;

        assert_eq!(expected, (expected_r, expected_g, expected_b));
    }

    #[test]
    fn test_image_to_led_rgb_extreme_values() {
        let led_color = LedColor::new();
        assert_eq!(led_color.image_to_led_rgb(0, 0, 0), (0, 0, 0));
        // Assuming the LED balance coefficients are such that full intensity (255) maps to (230, 255, 153)
        assert_eq!(led_color.image_to_led_rgb(255, 255, 255), (230, 255, 153));
    }

    #[test]
    fn test_image_to_led_rgb_varying_balance() {
        let mut led_color = LedColor::new();
        led_color.balance = vec![1.0, 1.0, 1.0]; // Custom balance coefficients for simplicity
        let r = 128;
        let g = 128;
        let b = 128;

        // Calculate the expected values using the same logic as the function, but with balance set to 1.0
        let expected_r = (255.0
            * led_color.color_gamma(LedColor::inv_color_gamma_image(r as f64 / 255.0)))
        .round() as u8;
        let expected_g = (255.0
            * led_color.color_gamma(LedColor::inv_color_gamma_image(g as f64 / 255.0)))
        .round() as u8;
        let expected_b = (255.0
            * led_color.color_gamma(LedColor::inv_color_gamma_image(b as f64 / 255.0)))
        .round() as u8;

        let expected = (expected_r, expected_g, expected_b);
        let result = led_color.image_to_led_rgb(r, g, b);

        assert_eq!(result, expected);
    }
}
