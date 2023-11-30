//! # Glow Control Library for Twinkly LEDs
//!
//! `glow-control-lib` is a Rust library for controlling Twinkly LED devices. It provides
//! a set of APIs to interact with LED hardware, allowing users to discover devices,
//! set device modes, control real-time lighting effects, and more.
//!
//! This library is designed to be used by command-line tools or other client applications
//! that require control over LED lighting systems.
//!
//! ## Features
//!
//! - Device discovery on local networks
//! - High-level control interfaces for device modes and settings
//! - Real-time effect control and custom LED movie uploads
//! - Utility functions for device authentication and communication
//!
//! ## Example
//!
//! Here is a simple example of how to use the library to discover Twinkly devices on your network:
//!
//! ```no_run
//! use glow_control_lib::util::discovery::Discovery;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Discover devices with a 5-second timeout
//!     let devices = Discovery::find_devices(Duration::from_secs(5)).await?;
//!
//!     // Iterate over the discovered devices and print their details
//!     for device in devices {
//!         println!("Found device: {:?}", device);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Disclaimer
//!
//! This project is not affiliated with, authorized by, endorsed by, or in any way officially connected
//! with Twinkly or its affiliates. The official Twinkly website can be found at [https://www.twinkly.com](https://www.twinkly.com).
//!
//! ## License
//!
//! This project is dual-licensed under the MIT License and the Apache License, Version 2.0.
//! You may choose to use either license, depending on your project needs.
//! See the `LICENSE-MIT` and `LICENSE-APACHE` files for the full text of the licenses.
// The `control_interface` module provides an interface for communicating with
// LED devices. It includes methods for sending commands, querying device status,
// and managing device settings.
//
// Example usage:
//
// ```
// use glow_control_lib::control_interface::ControlInterface;
// use glow_control_lib::control_interface::DeviceMode;
//
// #[tokio::main]
// async fn main() {
//     let control = ControlInterface::new("192.168.1.100", "AA:BB:CC:DD:EE:FF").await.unwrap();
//     control.set_mode(DeviceMode::Color).await.unwrap();
// }
// ```
pub mod control_interface;

// The `led` module contains abstractions and utilities for working with LED colors,
// patterns, and animations. It provides functionality to create and manipulate
// color patterns, apply effects, and convert between different color models.
//
// Example usage:
//
// ```
// use glow_control_lib::led::pattern::Pattern;
// use glow_control_lib::led::led_color::LedColor;
//
// let led_color = LedColor::new();
// let pattern = Pattern::make_color_spectrum_pattern(30, 0, 0.5, &led_color);
// ```
pub mod led;

// The `util` module provides various utility functions and structures that support
// the main functionality of the library. This includes authentication helpers,
// device discovery mechanisms, and other shared resources used across the library.
//
// Example usage:
//
// ```
// use glow_control_lib::util::discovery::Discovery;
// use std::time::Duration;
//
// #[tokio::main]
// async fn main() {
//     let devices = Discovery::find_devices(Duration::from_secs(5)).await.unwrap();
//     for device in devices {
//         println!("Found device: {:?}", device);
//     }
// }
// ```
pub mod util;
