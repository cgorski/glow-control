use crate::util::control::LedProfile;
use anyhow::Result;
use std::fs::File;
use std::io;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct Movie {
    pub frames: Vec<Vec<(u8, u8, u8)>>,
    pub fps: f64,
}
impl Movie {
    // ...

    /// Converts a vector of frames into a binary movie format.
    /// This function handles both RGB and RGBW LED profiles.
    pub fn to_movie(frames: Vec<Vec<(u8, u8, u8)>>, led_profile: LedProfile) -> Vec<u8> {
        let mut movie_data = Vec::new();
        for frame in frames {
            for &(r, g, b) in &frame {
                match led_profile {
                    LedProfile::RGB => {
                        movie_data.push(r);
                        movie_data.push(g);
                        movie_data.push(b);
                    }
                    LedProfile::RGBW => {
                        // Calculate the white component as the minimum of r, g, b
                        let w = r.min(g).min(b);
                        movie_data.push(r - w);
                        movie_data.push(g - w);
                        movie_data.push(b - w);
                        movie_data.push(w);
                    }
                }
            }
        }
        movie_data
    }

    pub fn load_movie<P: AsRef<Path>>(path: P, led_profile: LedProfile) -> Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read the header
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header_parts: Vec<&str> = header.split_whitespace().collect();
        if header_parts.len() != 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid header format").into());
        }

        let num_frames: usize = header_parts[0].parse()?;
        let num_leds: usize = header_parts[1].parse()?;
        let bytes_per_led: usize = header_parts[2].parse()?;
        let fps: f64 = header_parts[3].parse()?;

        // Read the frames
        let mut frames = Vec::with_capacity(num_frames);
        for _ in 0..num_frames {
            let mut frame_hex = String::new();
            reader.read_line(&mut frame_hex)?;
            let frame_bytes = hex::decode(frame_hex.trim())?;

            // Convert frame data to RGB or RGBW tuples
            let mut frame = Vec::with_capacity(num_leds);
            for chunk in frame_bytes.chunks(bytes_per_led) {
                let rgb_tuple = match led_profile {
                    LedProfile::RGB => (chunk[0], chunk[1], chunk[2]),
                    LedProfile::RGBW => {
                        // Assuming the white component is the last byte
                        let w = chunk[3];
                        (chunk[0] + w, chunk[1] + w, chunk[2] + w)
                    }
                };
                frame.push(rgb_tuple);
            }
            frames.push(frame);
        }

        Ok(Movie { frames, fps })
    }

    /// Saves a movie to a file in a text-based format.
    pub fn save_movie<P: AsRef<Path>>(&self, path: P, led_profile: LedProfile) -> io::Result<()> {
        let mut file = File::create(path)?;

        // Write the header
        let num_frames = self.frames.len();
        let num_leds = self.frames.first().map_or(0, Vec::len);
        let bytes_per_led = match led_profile {
            LedProfile::RGB => 3,
            LedProfile::RGBW => 4,
        };
        writeln!(
            file,
            "{} {} {} {}",
            num_frames, num_leds, bytes_per_led, self.fps
        )?;

        // Write the frames
        for frame in &self.frames {
            for &(r, g, b) in frame {
                match led_profile {
                    LedProfile::RGB => {
                        write!(file, "{:02X}{:02X}{:02X}", r, g, b)?;
                    }
                    LedProfile::RGBW => {
                        // Calculate the white component as the minimum of r, g, b
                        let w = r.min(g).min(b);
                        write!(file, "{:02X}{:02X}{:02X}{:02X}", r - w, g - w, b - w, w)?;
                    }
                }
            }
            writeln!(file)?; // Newline after each frame
        }

        Ok(())
    }

    // ... Additional methods ...
}
