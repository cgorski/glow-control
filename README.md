![Build Status](https://github.com/cgorski/glow-control/actions/workflows/rust.yml/badge.svg?branch=main)

[![Crates.io](https://img.shields.io/crates/v/glow-control-lib.svg)](https://crates.io/crates/glow-control-lib)
`Library`

[![Crates.io](https://img.shields.io/crates/v/glow-control.svg)](https://crates.io/crates/glow-control)
`CLI`

# Glow Control Library for Twinkly LEDs

The `glow-control-lib` crate is a Rust library designed to interface with Twinkly LED devices. It provides a
comprehensive set of APIs that facilitate the discovery of devices, manipulation of device modes, control of real-time
lighting effects, and more. This library serves as the backbone for the `glow-control` CLI and can be used to build
custom applications that manage Twinkly LED lights.

This project draws inspiration from the Python libraries [xled](https://github.com/scrool/xled)
and [xled_plus](https://github.com/Anders-Holst/xled_plus), and it is intended to be an open-source alternative for the
Rust ecosystem.

## Features

- Network-based discovery of Twinkly devices
- High-level control interfaces for managing device modes and settings
- Real-time effect control from an external network device
- Custom LED movie uploads
- Utility functions for device authentication and communication

## Library Usage

To include this library in your Rust project, add the following to your `Cargo.toml`:

```toml
[dependencies]
glow-control-lib = { version = "0.3.5", path = "../glow-control-lib" }
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
        println!("\n{} Found device {}\n{:?}\n", "=".repeat(30), "=".repeat(30), device);

        let control = ControlInterface::from_device_identifier(device).await?;
        control.set_mode(DeviceMode::Color).await?;
    }

    Ok(())
}
```

For more examples and detailed API documentation, run `cargo doc --open` after adding the library to your project.

## CLI Usage

To install the CLI application, ensure you have Rust and Cargo installed, then run:

```bash
cargo install glow-control
```

to install from crates-io, or:

```bash
cargo install --path .
```

to install directly from the repository.

### Preparing Your Devices

To get started, make sure your lights have been configured on your local network using the official Twinkly app, and
that you have used the app to generate a 3D point cloud. This point cloud will be stored on your individual Twinkly
devices by the official app, so after initial configuration, you no longer need the app to use `glow-control`.

### Discover Feature

The `glow-control` CLI provides a `discover` subcommand that allows you to scan your network for Twinkly devices. You
can use this feature to find the IP and MAC addresses of your devices, which are required for other CLI commands.

To use the `discover` feature, run the following command:

The discovery search time is 5000ms by default. You can adjust it if needed.

```
glow-control discover
```

```
IP Address    Device ID        MAC Address         Device Name
-----------   --------------   -----------------   -------  
10.10.0.42    Twinkly_C54ABC   11:38:aa:c4:aa:55   Living Room  
10.10.0.37    Twinkly_C5CDEF   bb:e5:7c:dd:bb:57   Kitchen    
```

You can also specify the output format using the `--output` option. Supported formats are:

- `plaintext` (default)
- `json`
- `yaml`

For example, to get the results in JSON format:

```
glow-control discover --output json
```

### Running the Real-Time Test Colors

To run the real-time test colors, use the `real-time-test` subcommand under the `device-call` command. This will display
a rotating color pattern on Twinkly device.

```bash
glow-control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> real-time-test
```

`<DEVICE_IP>` and `<DEVICE_MAC>` are the IP and MAC addresses of your Twinkly device, respectively. They must be
specified for all device-specific commands.

The real-time test inputs frames directly from the CLI binary. Once you terminate the program, the Twinkly device will
eventually timeout and return to its previous state.

### Other Examples

Set the device mode to 'movie':

```glow-control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> set-mode movie```

Show a solid color:

```glow-control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> rt-effect show-color --color Red```

## License

This library is dual-licensed under the MIT License and the Apache License, Version 2.0, allowing you to choose the
license that best fits your project's needs. The full text of the licenses can be found in the `LICENSE-MIT`
and `LICENSE-APACHE` files.

## Disclaimer

This project is not affiliated with, authorized by, endorsed by, or in any way officially connected with Twinkly or its
affiliates. The official Twinkly website can be found at [https://www.twinkly.com](https://www.twinkly.com).

## Contributions

Contributions are welcome! If you would like to contribute to this library, please feel free to open an issue or create
a pull request with your improvements or suggestions.


