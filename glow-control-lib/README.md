![Build Status](https://github.com/cgorski/glow-control/actions/workflows/rust.yml/badge.svg?branch=main)
[![Crates.io](https://img.shields.io/crates/v/glow-control-lib.svg)](https://crates.io/crates/glow-control-lib)


# Glow Control Library for Twinkly LEDs

The `glow-control-lib` crate is a Rust library designed to interface with Twinkly LED devices. It provides a comprehensive set of APIs that facilitate the discovery of devices, manipulation of device modes, control of real-time lighting effects, and more. This library serves as the backbone for the `glow-control` CLI and can be used to build custom applications that manage Twinkly LED lights.

This project draws inspiration from the Python libraries [xled](https://github.com/scrool/xled) and [xled_plus](https://github.com/Anders-Holst/xled_plus), and it is intended to be an open-source alternative for the Rust ecosystem.

## Features

- Network-based discovery of Twinkly devices
- High-level control interfaces for managing device modes and settings
- Real-time effect control and custom LED movie uploads
- Utility functions for device authentication and communication

## Usage

To include this library in your Rust project, add the following to your `Cargo.toml`:

```toml
[dependencies]
glow-control-lib = { version = "0.3.3", path = "../glow-control-lib" }
```

Here's a simple example of how to use the library to set Twinkly devices to a specific mode:

```rust
use std::collections::HashSet;
use std::time::Duration;
use glow_control_lib::control_interface::{ControlInterface, DeviceMode};
use glow_control_lib::util::discovery::{DeviceIdentifier, Discovery};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Discover devices with a 5-second timeout
    let devices: HashSet<DeviceIdentifier> = Discovery::find_devices(Duration::from_secs(5)).await?;

    // Iterate over the discovered devices, print their details and set their mode
    for device in devices {
        println!("Found device: {:?}", device);

        let control = ControlInterface::from_device_identifier(device).await?;
        control.set_mode(DeviceMode::Color).await?;
    }

    Ok(())
}
```

For more examples and detailed API documentation, run `cargo doc --open` after adding the library to your project.

## License

This library is dual-licensed under the MIT License and the Apache License, Version 2.0, allowing you to choose the license that best fits your project's needs. The full text of the licenses can be found in the `LICENSE-MIT` and `LICENSE-APACHE` files.

## Disclaimer

This project is not affiliated with, authorized by, endorsed by, or in any way officially connected with Twinkly or its affiliates. The official Twinkly website can be found at [https://www.twinkly.com](https://www.twinkly.com).

## Contributions

Contributions are welcome! If you would like to contribute to this library, please feel free to open an issue or create a pull request with your improvements or suggestions.
