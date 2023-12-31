use crate::util::auth::Auth;
use crate::util::movie::Movie;
use anyhow::{anyhow, Context};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use bytes::{BufMut, BytesMut};
use chrono::{NaiveTime, Timelike};
use clap::ValueEnum;
use palette::{FromColor, Hsl, IntoColor, Srgb};
use rand::prelude::{IteratorRandom, SliceRandom};
use rand::thread_rng;
use reqwest::{Client, StatusCode};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::{interval, sleep, Instant};

#[derive(Debug)]
pub struct ControlInterface {
    host: String,
    hw_address: String,
    auth_token: String,
    client: Client,
    device_info: DeviceInfoResponse,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum CliDeviceMode {
    Movie,
    Playlist,
    RealTime,
    Demo,
    Effect,
    Color,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceMode {
    Movie,
    Playlist,
    RealTime,
    Demo,
    Effect,
    Color,
    Off,
}

impl std::str::FromStr for DeviceMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "movie" => Ok(DeviceMode::Movie),
            "playlist" => Ok(DeviceMode::Playlist),
            "rt" => Ok(DeviceMode::RealTime),
            "demo" => Ok(DeviceMode::Demo),
            "effect" => Ok(DeviceMode::Effect),
            "color" => Ok(DeviceMode::Color),
            "off" => Ok(DeviceMode::Off),
            _ => Err(anyhow!("Invalid mode")),
        }
    }
}

impl fmt::Display for DeviceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode_str = match self {
            DeviceMode::Movie => "movie",
            DeviceMode::Playlist => "playlist",
            DeviceMode::RealTime => "rt",
            DeviceMode::Demo => "demo",
            DeviceMode::Effect => "effect",
            DeviceMode::Color => "color",
            DeviceMode::Off => "off",
        };
        write!(f, "{}", mode_str)
    }
}

impl ControlInterface {
    pub async fn new(host: &str, hw_address: &str) -> anyhow::Result<Self> {
        let client = Client::new();
        let auth_token = ControlInterface::authenticate(&client, host, hw_address).await?;

        // Fetch the device information
        let device_info = ControlInterface::fetch_device_info(&client, host, &auth_token).await?;

        Ok(ControlInterface {
            host: host.to_string(),
            hw_address: hw_address.to_string(),
            auth_token,
            client,
            device_info,
        })
    }

    pub fn get_hw_address(&self) -> String {
        self.hw_address.clone()
    }

    pub async fn shine_leds(
        &self,
        time_between_glow_start: Duration,
        time_to_max_glow: Duration,
        time_to_fade: Duration,
        colors: HashSet<RGB>,
        frame_rate: f64,
        num_start_simultaneous: usize,
    ) -> anyhow::Result<()> {
        let num_leds = self.device_info.number_of_led;

        // Validate num_start_simultaneous
        if num_start_simultaneous == 0 || num_start_simultaneous > num_leds {
            return Err(anyhow!(
                "num_start_simultaneous must be between 1 and {}",
                num_leds
            ));
        }

        // Validate colors is not empty
        if colors.is_empty() {
            return Err(anyhow!("colors set must not be empty"));
        }

        let mut leds = vec![(0, 0, 0); num_leds];
        let mut glow_start_times =
            vec![Instant::now() - (time_to_max_glow + time_to_fade) * 2; num_leds];
        let mut led_colors = vec![
            RGB {
                red: 0,
                green: 0,
                blue: 0
            };
            num_leds
        ]; // Track color for each LED

        let frame_duration = Duration::from_secs_f64(1.0 / frame_rate);
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((self.host.as_str(), 7777)).await?;
        self.set_mode(DeviceMode::RealTime).await?;

        let mut interval_timer = interval(frame_duration);
        let mut time_since_last_glow = time_between_glow_start;

        loop {
            interval_timer.tick().await;
            let now = Instant::now();

            // Check if it's time to start new glows
            if time_since_last_glow >= time_between_glow_start {
                let mut rng = thread_rng();
                let mut available_leds: Vec<usize> = (0..num_leds)
                    .filter(|&i| {
                        now.duration_since(glow_start_times[i]) >= time_to_max_glow + time_to_fade
                    })
                    .collect();

                // Shuffle the available LEDs to randomize the selection
                available_leds.shuffle(&mut rng);

                for &led_index in available_leds.iter().take(num_start_simultaneous) {
                    glow_start_times[led_index] = now;
                    // Randomly select a color for the LED
                    led_colors[led_index] = *colors
                        .iter()
                        .choose(&mut rng)
                        .expect("colors set is not empty");
                }

                time_since_last_glow = Duration::from_secs(0);
            } else {
                time_since_last_glow += frame_duration;
            }

            // Update the state of each LED
            for i in 0..num_leds {
                let elapsed = now.duration_since(glow_start_times[i]);
                let color = led_colors[i];

                // Calculate the brightness based on the elapsed time
                let brightness = if elapsed < time_to_max_glow {
                    elapsed.as_secs_f64() / time_to_max_glow.as_secs_f64()
                } else if elapsed < time_to_max_glow + time_to_fade {
                    1.0 - (elapsed - time_to_max_glow).as_secs_f64() / time_to_fade.as_secs_f64()
                } else {
                    0.0 // LED is off
                };

                // Set the color with the calculated brightness
                leds[i] = (
                    (color.red as f64 * brightness) as u8,
                    (color.green as f64 * brightness) as u8,
                    (color.blue as f64 * brightness) as u8,
                );
            }

            // Send the updated frame to the device
            let flattened_frame = ControlInterface::flatten_rgb_vec(leds.clone());
            self.set_rt_frame_socket(&socket, &flattened_frame, 3)
                .await?;
        }
    }

    pub async fn show_solid_color(&self, rgb: RGB) -> anyhow::Result<()> {
        let frame = vec![(rgb.red, rgb.green, rgb.blue); self.device_info.number_of_led];
        let flattened_frame = ControlInterface::flatten_rgb_vec(frame);
        self.set_mode(DeviceMode::RealTime).await?;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((self.host.as_str(), 7777)).await?;
        loop {
            self.set_rt_frame_socket(&socket, &flattened_frame, 3)
                .await?;
            sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn show_real_time_test_color_wheel(
        &self,
        step: f64,
        frame_rate: f64,
    ) -> anyhow::Result<()> {
        let interval = Duration::from_secs_f64(1.0 / frame_rate);
        let mut offset = 0_f64;
        self.set_mode(DeviceMode::RealTime).await?;
        let layout = self.fetch_layout().await?;
        loop {
            //   let gradient_frame = generate_color_wheel_gradient(self.device_info.number_of_led, offset);
            let gradient_frame =
                generate_color_gradient_along_axis(&layout.coordinates, Axis::Z, offset);
            let gradient_frame = ControlInterface::flatten_rgb_vec(gradient_frame);
            let socket = UdpSocket::bind("0.0.0.0:0").await?;
            socket.connect((self.host.as_str(), 7777)).await?;
            self.set_rt_frame_socket(&socket, &gradient_frame, 3)
                .await?;

            // Increment the offset for the next frame
            offset = (offset + step) % 1.0;

            // Sleep for the interval duration to maintain the frame rate
            sleep(interval).await;

            println!("Offset: {}", offset);
        }
    }

    pub fn flatten_rgb_vec(rgb_vec: Vec<(u8, u8, u8)>) -> Vec<u8> {
        rgb_vec
            .into_iter()
            .flat_map(|(r, g, b)| vec![r, g, b])
            .collect()
    }
    pub async fn set_rt_frame_socket(
        &self,
        socket: &UdpSocket,
        frame: &[u8],
        version: u32,
    ) -> anyhow::Result<()> {
        // Determine the protocol version from the device configuration
        // let version = self.device_info.fw_version; // Assuming fw_version is a field in DeviceInfoResponse

        // Decode the access token
        let access_token = STANDARD
            .decode(&self.auth_token)
            .context("Failed to decode access token")?;

        // Prepare the packet based on the protocol version
        let mut packet = BytesMut::new();
        match version {
            1 => {
                packet.put_u8(1); // Protocol version 1
                packet.extend_from_slice(&access_token);
                packet.put_u8(self.device_info.number_of_led as u8); // Number of LEDs
                packet.extend_from_slice(frame);
            }
            2 => {
                packet.put_u8(2); // Protocol version 2
                packet.extend_from_slice(&access_token);
                packet.put_u8(0); // Placeholder byte
                packet.extend_from_slice(frame);
            }
            _ => {
                // Protocol version 3 or higher
                let packet_size = 900;
                for (i, chunk) in frame.chunks(packet_size).enumerate() {
                    packet.clear();
                    packet.put_u8(3); // Protocol version 3
                    packet.extend_from_slice(&access_token);
                    packet.put_u16(0); // Placeholder bytes
                    packet.put_u8(i as u8); // Frame index
                    packet.extend_from_slice(chunk);
                    socket.send(&packet).await?;
                }
                return Ok(()); // Early return for version 3
            }
        }

        // Send the packet for versions 1 and 2
        socket.send(&packet).await?;

        Ok(())
    }
    pub async fn show_rt_frame(&self, frame: &[u8]) -> anyhow::Result<()> {
        // Fetch the current mode from the device
        let mode_response = self.get_mode().await?;
        let current_mode = mode_response;

        // Check if we need to switch to real-time mode
        if current_mode != DeviceMode::RealTime {
            self.set_mode(DeviceMode::RealTime).await?;
        }

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect((self.host.as_str(), 7777)).await?;
        // Call the set_rt_frame_socket method to send the frame
        self.set_rt_frame_socket(&socket, frame, 3).await?;

        Ok(())
    }

    pub fn get_device_info(&self) -> &DeviceInfoResponse {
        &self.device_info
    }
    async fn fetch_device_info(
        client: &Client,
        host: &str,
        auth_token: &str,
    ) -> anyhow::Result<DeviceInfoResponse> {
        let url = format!("http://{}/xled/v1/gestalt", host);
        let response = client
            .get(&url)
            .header("X-Auth-Token", auth_token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch layout: {}", e))?;

        if response.status() != reqwest::StatusCode::OK {
            return Err(anyhow!(
                "Failed to fetch device info with status: {}",
                response.status()
            ));
        }
        let response = response.text().await?;
        println!("Response: {}", response);
        let device_info: DeviceInfoResponse = serde_json::from_str(&response)?;
        // let device_info = response
        //     .json::<DeviceInfoResponse>()
        //     .await
        //     .map_err(|e| anyhow!("Failed to deserialize device info: {}", e))?;

        Ok(device_info)
    }

    /// Uploads a new movie to the device.
    pub async fn upload_movie<P: AsRef<Path>>(
        &self,
        path: P,
        led_profile: LedProfile,
        _fps: f64,
        force: bool,
    ) -> anyhow::Result<u32> {
        let movie = Movie::load_movie(path, led_profile)?;
        let num_frames = movie.frames.len();
        let _num_leds = self.device_info.number_of_led;
        let _bytes_per_led = match led_profile {
            LedProfile::RGB => 3,
            LedProfile::RGBW => 4,
        };

        // Check if the movie fits in the remaining capacity
        let capacity = self.get_device_capacity().await?;
        if num_frames > capacity && !force {
            return Err(anyhow!("Not enough capacity for the movie"));
        }

        // Clear existing movies if necessary
        if force {
            self.clear_movies().await?;
        }

        // Convert the movie to the binary format expected by the device
        let movie_data = Movie::to_movie(movie.frames, led_profile);

        // Upload the movie to the device
        let url = format!("http://{}/xled/v1/led/movie/full", self.host);
        let response = self
            .client
            .post(&url)
            .header("X-Auth-Token", &self.auth_token)
            .body(movie_data)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let response_text = response.text().await?;
                let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
                if let Some(id) = response_json["id"].as_u64() {
                    Ok(id as u32)
                } else {
                    Err(anyhow!("Failed to get movie ID from response"))
                }
            }
            _ => Err(anyhow!(
                "Failed to upload movie with status: {}",
                response.status()
            )),
        }
    }

    /// Turns on the device by setting it to the last known mode or a default mode.
    pub async fn turn_on(&self) -> anyhow::Result<()> {
        // Fetch the current mode from the device
        let mode_response = self.get_mode().await?;
        let current_mode = mode_response;

        // If the device is already on, we don't need to change the mode
        if current_mode != DeviceMode::Off {
            return Ok(());
        }

        // If the device is off, set it to a default mode
        // Here we choose "movie" as the default mode, but you can adjust as needed
        self.set_mode(DeviceMode::Movie).await
    }

    /// Turns off the device and remembers the last non-real-time mode.
    pub async fn turn_off(&mut self) -> anyhow::Result<()> {
        // Set the device mode to "off"
        self.set_mode(DeviceMode::Off).await
    }

    /// Helper method to set the device mode.
    pub async fn set_mode(&self, mode: DeviceMode) -> anyhow::Result<()> {
        let url = format!("http://{}/xled/v1/led/mode", self.host);
        let response = self
            .client
            .post(&url)
            .header("X-Auth-Token", &self.auth_token)
            .json(&json!({ "mode": mode.to_string() }))
            .send()
            .await
            .context("Failed to set mode")?;

        if response.status() == StatusCode::OK {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to set mode with status: {}",
                response.status()
            ))
        }
    }

    async fn authenticate(client: &Client, host: &str, hw_address: &str) -> anyhow::Result<String> {
        // Generate a random challenge
        let challenge = Auth::generate_challenge();

        // Send the challenge to the device and get the response
        let challenge_response = send_challenge(client, host, &challenge).await?;

        // Create a challenge response based on the device's response and the MAC address
        let response = Auth::make_challenge_response(&challenge, hw_address)?;

        // Send the verification to the device
        send_verify(
            client,
            host,
            &challenge_response.authentication_token,
            &response,
        )
        .await?;

        Ok(challenge_response.authentication_token)
    }

    pub async fn get_mode(&self) -> anyhow::Result<DeviceMode> {
        let url = format!("http://{}/xled/v1/led/mode", self.host);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await
            .context("Failed to get mode")?;

        match response.status() {
            StatusCode::OK => {
                let mode_response = response.json::<ModeResponse>().await?;
                println!("Mode response: {:#?}", mode_response);
                println!("Mode: {}", mode_response.mode);
                let mode = DeviceMode::from_str(&mode_response.mode)
                    .map_err(|e| anyhow!("Failed to parse mode: {}", e))?;
                Ok(mode)
            }
            _ => Err(anyhow::anyhow!(
                "Failed to get mode with status: {}",
                response.status()
            )),
        }
    }

    pub async fn get_timer(&self) -> anyhow::Result<TimerResponse> {
        let url = format!("http://{}/xled/v1/timer", self.host);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await
            .context("Failed to get timer")?;

        match response.status() {
            StatusCode::OK => {
                let timer_response = response.json::<TimerResponse>().await?;
                Ok(timer_response)
            }
            _ => Err(anyhow::anyhow!(
                "Failed to get timer with status: {}",
                response.status()
            )),
        }
    }

    pub async fn set_formatted_timer(
        &self,
        time_on_str: &str,
        time_off_str: &str,
    ) -> anyhow::Result<()> {
        // Parse the time strings into NaiveTime objects
        let time_on = NaiveTime::parse_from_str(time_on_str, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(time_on_str, "%H:%M"))
            .context("Failed to parse time_on string")?;
        let time_off = NaiveTime::parse_from_str(time_off_str, "%H:%M:%S")
            .or_else(|_| NaiveTime::parse_from_str(time_off_str, "%H:%M"))
            .context("Failed to parse time_off string")?;

        // Convert NaiveTime objects to seconds after midnight
        let time_on_seconds = time_on.num_seconds_from_midnight() as i32;
        let time_off_seconds = time_off.num_seconds_from_midnight() as i32;

        // Construct the URL for setting the timer
        let url = format!("http://{}/xled/v1/timer", self.host);

        // Send the request to set the timer
        let response = self
            .client
            .post(&url)
            .header("X-Auth-Token", &self.auth_token)
            .json(&json!({
                "time_on": time_on_seconds,
                "time_off": time_off_seconds,
            }))
            .send()
            .await
            .context("Failed to set timer")?;

        // Check the response status
        if response.status() == StatusCode::OK {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Failed to set timer with status: {}",
                response.status()
            ))
        }
    }

    pub async fn get_playlist(&self) -> anyhow::Result<PlaylistResponse> {
        let url = format!("http://{}/xled/v1/playlist", self.host);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let response = response.text().await?;
                println!("Response: {}", response);
                let playlist_response: PlaylistResponse = serde_json::from_str(&response)?;
                // let playlist_response = response.json::<PlaylistResponse>().await?;
                Ok(playlist_response)
            }
            _ => Err(response.error_for_status().unwrap_err().into()),
        }
    }

    /// Fetches the LED layout from the device.
    pub async fn fetch_layout(&self) -> anyhow::Result<LayoutResponse> {
        let url = format!("http://{}/xled/v1/led/layout/full", self.host);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await
            .context("Failed to fetch layout")?;

        if response.status() == StatusCode::OK {
            let layout_response = response
                .json::<LayoutResponse>()
                .await
                .context("Failed to deserialize layout response")?;
            Ok(layout_response)
        } else {
            Err(anyhow::anyhow!(
                "Failed to fetch layout with status: {}",
                response.status()
            ))
        }
    }

    pub async fn get_device_capacity(&self) -> anyhow::Result<usize> {
        let url = format!("http://{}/xled/v1/led/movies", self.host);
        let response = self
            .client
            .get(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let response_json = response.json::<serde_json::Value>().await?;
                if let Some(available_frames) = response_json["available_frames"].as_u64() {
                    Ok(available_frames as usize)
                } else {
                    Err(anyhow!("Failed to get available frames from response"))
                }
            }
            _ => Err(anyhow!(
                "Failed to get device capacity with status: {}",
                response.status()
            )),
        }
    }

    /// Clears all uploaded movies from the device.
    pub async fn clear_movies(&self) -> anyhow::Result<()> {
        let url = format!("http://{}/xled/v1/led/movies", self.host);
        let response = self
            .client
            .delete(&url)
            .header("X-Auth-Token", &self.auth_token)
            .send()
            .await?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            _ => Err(anyhow!(
                "Failed to clear movies with status: {}",
                response.status()
            )),
        }
    }

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

    // ... other methods ...
}

// Define a struct to deserialize the device information response
#[derive(Deserialize, Debug)]
pub struct DeviceInfoResponse {
    pub product_name: String,
    pub hardware_version: String,
    pub bytes_per_led: usize,
    pub hw_id: String,
    pub flash_size: usize,
    pub led_type: usize,
    pub product_code: String,
    pub fw_family: String,
    pub device_name: String,
    #[serde(deserialize_with = "deserialize_duration_millis")]
    pub uptime: Duration, // Uptime is now an unsigned 64-bit integer
    pub mac: String,
    pub uuid: String,
    pub max_supported_led: usize,
    pub number_of_led: usize,
    pub led_profile: LedProfile, // LedProfile is now an enum
    pub frame_rate: f64,
    pub measured_frame_rate: f64,
    pub movie_capacity: usize,
    pub max_movies: usize,
    pub wire_type: usize,
    pub copyright: String,
    pub code: usize,
}

fn deserialize_duration_millis<'de, D>(deserializer: D) -> anyhow::Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millis_str: String = Deserialize::deserialize(deserializer)?;
    millis_str
        .parse::<u64>()
        .map(Duration::from_millis)
        .map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum LedProfile {
    RGB,
    RGBW,
    // Add other LED profiles as needed
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaylistEntry {
    pub id: u32,
    pub unique_id: String,
    pub name: String,
    pub duration: u32,
    pub handle: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlaylistResponse {
    pub entries: Vec<PlaylistEntry>,
    pub unique_id: String,
    pub name: String,
    pub code: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ModeResponse {
    pub mode: String,
    pub code: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TimerResponse {
    pub time_now: i32,
    pub time_off: i32,
    pub time_on: i32,
    pub code: u32,
}

#[derive(Deserialize, Debug)]
pub struct LedCoordinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Define a struct to deserialize the layout response
#[derive(Deserialize, Debug)]
pub struct LayoutResponse {
    pub source: String,
    pub synthesized: bool,
    pub uuid: String,
    pub coordinates: Vec<LedCoordinate>,
    pub code: u32,
}

pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Challenge {
    challenge: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RGB {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliColors {
    Red,
    Green,
    Blue,
    Yellow,
    Orange,
    Purple,
    Cyan,
    Magenta,
    Lime,
    Pink,
    Teal,
    Lavender,
    Brown,
    Beige,
    Maroon,
    Mint,
}

impl FromStr for CliColors {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "red" => Ok(CliColors::Red),
            "green" => Ok(CliColors::Green),
            "blue" => Ok(CliColors::Blue),
            "yellow" => Ok(CliColors::Yellow),
            "orange" => Ok(CliColors::Orange),
            "purple" => Ok(CliColors::Purple),
            "cyan" => Ok(CliColors::Cyan),
            "magenta" => Ok(CliColors::Magenta),
            "lime" => Ok(CliColors::Lime),
            "pink" => Ok(CliColors::Pink),
            "teal" => Ok(CliColors::Teal),
            "lavender" => Ok(CliColors::Lavender),
            "brown" => Ok(CliColors::Brown),
            "beige" => Ok(CliColors::Beige),
            "maroon" => Ok(CliColors::Maroon),
            "mint" => Ok(CliColors::Mint),
            _ => Err(anyhow!("Invalid color")),
        }
    }
}

impl From<CliDeviceMode> for DeviceMode {
    fn from(mode: CliDeviceMode) -> Self {
        match mode {
            CliDeviceMode::Movie => DeviceMode::Movie,
            CliDeviceMode::Playlist => DeviceMode::Playlist,
            CliDeviceMode::RealTime => DeviceMode::RealTime,
            CliDeviceMode::Demo => DeviceMode::Demo,
            CliDeviceMode::Effect => DeviceMode::Effect,
            CliDeviceMode::Color => DeviceMode::Color,
            CliDeviceMode::Off => DeviceMode::Off,
        }
    }
}

impl From<CliColors> for RGB {
    fn from(color: CliColors) -> Self {
        match color {
            CliColors::Red => RGB {
                red: 255,
                green: 0,
                blue: 0,
            },
            CliColors::Green => RGB {
                red: 0,
                green: 255,
                blue: 0,
            },
            CliColors::Blue => RGB {
                red: 0,
                green: 0,
                blue: 255,
            },
            CliColors::Yellow => RGB {
                red: 255,
                green: 255,
                blue: 0,
            },
            CliColors::Orange => RGB {
                red: 255,
                green: 165,
                blue: 0,
            },
            CliColors::Purple => RGB {
                red: 128,
                green: 0,
                blue: 128,
            },
            CliColors::Cyan => RGB {
                red: 0,
                green: 255,
                blue: 255,
            },
            CliColors::Magenta => RGB {
                red: 255,
                green: 0,
                blue: 255,
            },
            CliColors::Lime => RGB {
                red: 50,
                green: 205,
                blue: 50,
            },
            CliColors::Pink => RGB {
                red: 255,
                green: 192,
                blue: 203,
            },
            CliColors::Teal => RGB {
                red: 0,
                green: 128,
                blue: 128,
            },
            CliColors::Lavender => RGB {
                red: 230,
                green: 230,
                blue: 250,
            },
            CliColors::Brown => RGB {
                red: 165,
                green: 42,
                blue: 42,
            },
            CliColors::Beige => RGB {
                red: 245,
                green: 245,
                blue: 220,
            },
            CliColors::Maroon => RGB {
                red: 128,
                green: 0,
                blue: 0,
            },
            CliColors::Mint => RGB {
                red: 189,
                green: 252,
                blue: 201,
            },
        }
    }
}

async fn send_verify(
    client: &Client,
    ip: &str,
    auth_token: &str,
    challenge_response: &str,
) -> anyhow::Result<()> {
    let verify_url = format!("http://{}/xled/v1/verify", ip);

    let response = client
        .post(&verify_url)
        .header("X-Auth-Token", auth_token)
        .json(&json!({ "challenge-response": challenge_response }))
        .send()
        .await
        .context("Failed to send verification")?;

    match response.status() {
        StatusCode::OK => {
            let verify_response = response
                .json::<VerifyResponse>()
                .await
                .context("Failed to deserialize verify response")?;
            if verify_response.code == 1000 {
                println!("Verify response code is 1000");
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Verification failed with code: {}",
                    verify_response.code
                ))
            }
        }
        _ => {
            let error_msg = format!("Verification failed with status: {}", response.status());
            Err(anyhow::anyhow!(error_msg))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct LoginResponse {
    authentication_token: String,
    #[serde(rename = "challenge-response")]
    challenge_response: String,
    code: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct VerifyResponse {
    code: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChallengeResponse {
    #[serde(rename = "challenge-response")]
    challenge_response: String,
    authentication_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Mode {
    mode: String,
}

pub fn generate_color_wheel_gradient(num_leds: usize, offset: usize) -> Vec<(u8, u8, u8)> {
    (0..num_leds)
        .map(|i| {
            // Calculate the index with offset, wrapping around using modulo if the offset is larger than num_leds
            let offset_index = (i + offset) % num_leds;
            // Calculate the hue for this LED, spreading the hues evenly across the color wheel
            let hue = offset_index as f32 / num_leds as f32 * 360.0;
            // Create an HSL color with full saturation and lightness for a fully saturated color
            let hsl_color = Hsl::new(hue, 1.0, 0.5);
            // Convert the HSL color to RGB
            let rgb_color = Srgb::from_color(hsl_color);
            // Convert the RGB color to 8-bit color components
            let (r, g, b) = rgb_color.into_components();
            ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        })
        .collect()
}

fn generate_color_gradient_along_axis(
    leds: &[LedCoordinate],
    axis: Axis,
    offset: f64,
) -> Vec<(u8, u8, u8)> {
    assert!(
        (0.0..1.0).contains(&offset),
        "Offset must be in the range [0.0, 1.0)"
    );

    // Determine the range of the specified axis
    let (min_value, max_value) = match axis {
        Axis::X => leds
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), led| {
                (min.min(led.x), max.max(led.x))
            }),
        Axis::Y => leds
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), led| {
                (min.min(led.y), max.max(led.y))
            }),
        Axis::Z => leds
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), led| {
                (min.min(led.z), max.max(led.z))
            }),
    };

    // Calculate the total range
    let total_range = max_value - min_value;

    // Apply the offset to the range
    let offset_value = total_range * offset;

    // Map each LED's position to a hue value and convert to RGB
    leds.iter()
        .map(|led| {
            // Determine the position of the LED on the specified axis
            let position = match axis {
                Axis::X => led.x,
                Axis::Y => led.y,
                Axis::Z => led.z,
            };

            // Apply the offset and wrap around using modulo to ensure the gradient is continuous
            let adjusted_position = (position - min_value + offset_value) % total_range;
            let hue = (adjusted_position / total_range) * 360.0;

            // Create an HSL color with full saturation and lightness for a fully saturated color
            let hsl_color = Hsl::new(hue as f32, 1.0, 0.5);

            // Convert the HSL color to RGB
            let rgb_color: Srgb = hsl_color.into_color();

            // Convert the RGB color to 8-bit color components
            let (r, g, b) = rgb_color.into_components();
            ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        })
        .collect()
}

async fn send_challenge(
    client: &Client,
    ip: &str,
    challenge: &[u8],
) -> anyhow::Result<ChallengeResponse> {
    let login_url = format!("http://{}/xled/v1/login", ip);
    let challenge_b64 = STANDARD.encode(challenge);

    let response = client
        .post(&login_url)
        .json(&Challenge {
            challenge: challenge_b64,
        })
        .send()
        .await
        .context("Failed to send authentication challenge")?;

    if response.status() != 200 {
        anyhow::bail!(
            "Authentication challenge failed with status: {}",
            response.status()
        );
    }

    println!("Challenge response: {:?}", response);
    let content = response.text().await?;
    println!("Challenge response content: {:?}", content);
    let challenge_response: ChallengeResponse =
        serde_json::from_str(&content).context("Failed to deserialize challenge response")?;

    Ok(challenge_response)
}
