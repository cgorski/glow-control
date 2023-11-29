use anyhow::{Result, anyhow};
use rand::prelude::SliceRandom;
use rand::Rng;
use rand_distr::Poisson;
use rand_distr::Distribution;
use crate::led::led_color::LedColor;

pub struct Pattern;

impl Pattern {
    pub fn random_discrete(probs: &[f64]) -> Result<usize> {
        let sum: f64 = probs.iter().sum();
        const TOLERANCE: f64 = 1e-5;

        if (sum - 1.0).abs() > TOLERANCE {
            // The sum of probabilities does not equal 1 within the tolerance
            return Err(anyhow!("Probabilities do not sum up to 1.0"));
        }

        let mut rng = rand::thread_rng();
        let mut acc = 0.0;
        let r: f64 = rng.gen(); // generates a float between 0.0 and 1.0
        for (ind, &prob) in probs.iter().enumerate() {
            acc += prob;
            if acc >= r {
                return Ok(ind);
            }
        }

        // This point should not be reached if the distribution is valid
        Err(anyhow!("Invalid probability distribution"))
    }

    pub fn random_poisson(lam: f64) -> Result<usize> {
        let poisson = Poisson::new(lam).map_err(|e| anyhow!("Poisson error: {}", e))?;
        let mut rng = rand::thread_rng();
        let sample = poisson.sample(&mut rng) as u64;

        if sample > usize::MAX as u64 {
            return Err(anyhow!("Sampled value is too large for usize"));
        }

        Ok(sample as usize)
    }

    pub fn dim_color(rgb: (u8, u8, u8), prop: f64) -> (u8, u8, u8) {
        let dimmed_r = (rgb.0 as f64 * prop).clamp(0.0, 255.0) as u8;
        let dimmed_g = (rgb.1 as f64 * prop).clamp(0.0, 255.0) as u8;
        let dimmed_b = (rgb.2 as f64 * prop).clamp(0.0, 255.0) as u8;
        (dimmed_r, dimmed_g, dimmed_b)
    }

    pub fn blend_colors(rgb1: (u8, u8, u8), rgb2: (u8, u8, u8), prop: f64) -> (u8, u8, u8) {
        let blend = |c1, c2| ((c1 as f64 * (1.0 - prop) + c2 as f64 * prop).clamp(0.0, 255.0) as u8);
        let blended_r = blend(rgb1.0, rgb2.0);
        let blended_g = blend(rgb1.1, rgb2.1);
        let blended_b = blend(rgb1.2, rgb2.2);
        (blended_r, blended_g, blended_b)
    }

    pub fn random_color() -> (u8, u8, u8) {
        let mut rng = rand::thread_rng();
        let r = rng.gen_range(0..=255);
        let g = rng.gen_range(0..=255);
        let b = rng.gen_range(0..=255);
        (r, g, b)
    }

    pub fn random_hsl_color_func<'a>(
        hue: Option<(f64, f64)>,
        sat: Option<(f64, f64)>,
        light: Option<(f64, f64)>,
        led_color: &'a LedColor, // Use explicit lifetime 'a
    ) -> Result<Box<dyn Fn() -> Result<(u8, u8, u8)> + 'a>> {
        // Helper to generate a random value within a given range or default to the full range if None
        let random_in_range = |range_option: Option<(f64, f64)>| -> f64 {
            let mut rng = rand::thread_rng();
            match range_option {
                Some((start, end)) => rng.gen_range(start..=end),
                None => rng.gen(),
            }
        };

        Ok(Box::new(move || {
            let h = random_in_range(hue.or(Some((0.0, 1.0))));
            let s = random_in_range(sat.or(Some((0.0, 1.0))));
            let l = random_in_range(light.or(Some((0.0, 1.0))));
            // Use the provided LedColor instance to convert HSL to RGB
            Ok(led_color.hsl_color(h, s, l))
        }))
    }



    pub fn sprinkle_pattern(
        &self,
        pat: &mut Vec<(u8, u8, u8)>,
        rgblst: &[(u8, u8, u8)],
        freq: f64,
    ) -> Result<()> {
        let n = Pattern::random_poisson(freq)?;
        let mut rng = rand::thread_rng();
        let leds = (0..pat.len()).collect::<Vec<_>>();
        let inds = leds.choose_multiple(&mut rng, n).cloned().collect::<Vec<_>>();
        for &i in &inds {
            let &color = rgblst.choose(&mut rng).ok_or_else(|| anyhow!("Color list is empty"))?;
            pat[i] = color;
        }
        Ok(())
    }

    pub fn make_alternating_color_pattern(leds: usize, rgblst: &[(u8, u8, u8)]) -> Vec<(u8, u8, u8)> {
        (0..leds).map(|i| rgblst[i % rgblst.len()]).collect()
    }

    pub fn make_color_spectrum_pattern(leds: usize, offset: usize, lightness: f64, led_color: &LedColor) -> Vec<(u8, u8, u8)> {
        (0..leds).map(|i| {
            let hue = ((i + offset) % leds) as f64 / leds as f64;
            led_color.hsl_color(hue, 1.0, lightness)
        }).collect()
    }

    pub fn make_random_select_color_pattern(leds: usize, rgblst: &[(u8, u8, u8)], probs: Option<&[f64]>) -> Result<Vec<(u8, u8, u8)>> {
        let mut rng = rand::thread_rng();
        let pattern = (0..leds).map(|_| {
            if let Some(probs) = probs {
                let ind = Pattern::random_discrete(probs)?;
                Ok(rgblst[ind])
            } else {
                let ind = rng.gen_range(0..rgblst.len());
                Ok(rgblst[ind])
            }
        }).collect::<Result<Vec<_>>>()?;
        Ok(pattern)
    }

    pub fn make_random_blend_color_pattern(leds: usize, rgb1: (u8, u8, u8), rgb2: (u8, u8, u8)) -> Vec<(u8, u8, u8)> {
        let mut rng = rand::thread_rng();
        (0..leds).map(|_| {
            let prop = rng.gen::<f64>();
            Pattern::blend_colors(rgb1, rgb2, prop)
        }).collect()
    }

    pub fn make_random_colors_pattern(leds: usize, lightness: f64, led_color: &LedColor) -> Vec<(u8, u8, u8)> {
        let mut rng = rand::thread_rng();
        (0..leds).map(|_| {
            let hue = rng.gen::<f64>();
            led_color.hsl_color(hue, 1.0, lightness)
        }).collect()
    }

    pub fn make_random_lightness_pattern(leds: usize, hue: f64, led_color: &LedColor) -> Vec<(u8, u8, u8)> {
        let mut rng = rand::thread_rng();
        (0..leds).map(|_| {
            let lightness = rng.gen::<f64>() * 2.0 - 1.0;
            led_color.hsl_color(hue, 1.0, lightness)
        }).collect()
    }

    pub fn make_random_hsl_pattern(
        leds: usize,
        hue: Option<(f64, f64)>,
        sat: Option<(f64, f64)>,
        light: Option<(f64, f64)>,
        led_color: &LedColor, // Pass a reference to LedColor
    ) -> Result<Vec<(u8, u8, u8)>> {
        // Now we pass the reference directly without dereferencing
        let color_func = Pattern::random_hsl_color_func(hue, sat, light, led_color)?;
        let pattern = (0..leds).map(|_| color_func()).collect::<Result<Vec<_>>>()?;
        Ok(pattern)
    }

}