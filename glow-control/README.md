# Glow Control CLI for Twinkly LEDs

The `glow-control` crate is a command-line interface (CLI) application for controlling Twinkly LED devices. It leverages the `glow-control-lib` library to provide users with the ability to discover devices on their network, control lighting effects, and manage device settings.

This project is heavily based on the Python libraries [xled](https://github.com/scrool/xled) and [xled_plus](https://github.com/Anders-Holst/xled_plus), and it aims to provide an open-source solution for controlling Twinkly LED devices.

## Features

- Discover Twinkly devices on the network
- Set and get device modes
- Control real-time lighting effects
- Fetch and set device configurations
- Upload and manage custom LED movies

## Installation

To install the CLI application, ensure you have Rust and Cargo installed, then run:

```bash
cargo install --path .
```

## Usage

After installation, you can run the `glow_control` command to interact with your Twinkly devices. Here are some example commands:

```bash
# Discover Twinkly devices on your network
glow_control discover

# Set the device mode to 'movie'
glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> set-mode movie

# Show a solid color
glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> rt-effect show-color --color Red
```

Replace `<DEVICE_IP>` and `<DEVICE_MAC>` with the IP and MAC address of your Twinkly device.

For more detailed usage instructions, run `glow_control --help`.

## License

This project is dual-licensed under the MIT License and the Apache License, Version 2.0. You may choose to use either license, depending on your project needs. See the `LICENSE-MIT` and `LICENSE-APACHE` files for the full text of the licenses.

## Disclaimer

This project is not affiliated with, authorized by, endorsed by, or in any way officially connected with Twinkly or its affiliates. The official Twinkly website can be found at [https://www.twinkly.com](https://www.twinkly.com).

## Contributions

We welcome contributions from the community! If you have a suggestion, bug report, or a feature request, please open an issue or submit a pull request.
