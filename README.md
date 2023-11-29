
# Glow Control

![Build Status](https://github.com/cgorski/glow-control/actions/workflows/rust.yml/badge.svg?branch=main)

This project provides a Rust-based command-line interface (CLI) for controlling Twinkly programmable LED lights. It allows users to interact with Twinkly devices on their network, offering a variety of commands to control lighting effects, colors, and device settings.

## Features

- Discover Twinkly devices on the network
- Set and get device modes
- Control real-time lighting effects
- Fetch and set device configurations
- Upload and manage custom LED movies

## Prerequisites

- Rust programming language
- Cargo package manager
- Access to a local network with Twinkly LED devices

## Installation

Clone the repository to your local machine:

```bash
git clone https://github.com/your-username/glow-control.git
cd glow-control
```

Build the project using Cargo:

```bash
cargo build --release
```

The executable will be located in `target/release`.

## Usage

### Discover Devices

To discover Twinkly devices on your network:

```bash
./glow_control discover
```

### Set Device Mode

To set the device mode:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> set-mode --mode <MODE>
```

### Show Solid Color

To show a solid color using the real-time functionality:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> rt-effect show-color --color <COLOR_NAME>
```

### Shine Effect

To start a shine effect with multiple colors:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> rt-effect shine --num_start_simultaneous 5 --colors Red Green Blue --time_between_glow_start 1000 --time_to_max_glow 500 --time_to_fade 500 --frame_rate 30.0
```

### Real-Time Test

To test the real-time color wheel display:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> real-time-test
```

### Fetch Layout

To fetch the LED layout:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> fetch-layout
```

### Clear Movies

To clear all uploaded movies from the device:

```bash
./glow_control device-call --ip <DEVICE_IP> --mac <DEVICE_MAC> clear-movies
```

Replace `<DEVICE_IP>`, `<DEVICE_MAC>`, `<MODE>`, and `<COLOR_NAME>` with the appropriate values for your device, desired mode, and color.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or create issues for bugs and feature requests.

## License

This project is open source and available under the [MIT License](LICENSE).

## Acknowledgments

This project is not affiliated with Twinkly but was created to provide an open-source solution for controlling Twinkly LED devices.
