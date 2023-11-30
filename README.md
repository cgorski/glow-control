![Build Status](https://github.com/cgorski/glow-control/actions/workflows/rust.yml/badge.svg?branch=main)

# Glow Control for Twinkly LEDs

This workspace is the home of `glow-control`, an unofficial Rust-based toolset for controlling Twinkly LED lights. It consists of a command-line interface (CLI) and a library that provides the underlying functionality for the CLI and can be used to integrate Twinkly LED control into other Rust applications.

The project is heavily inspired by the Python libraries [xled](https://github.com/scrool/xled) and [xled_plus](https://github.com/Anders-Holst/xled_plus), and it aims to provide an open-source solution for controlling Twinkly LED devices.

## Workspace Structure

- `glow-control`: The binary CLI application that allows users to interact with Twinkly LED devices from the command line.
- `glow-control-lib`: The library crate that provides APIs for device discovery, control interfaces, and utility functions.

## Usage

To use the CLI application, navigate to the `glow-control` directory and follow the instructions in the `README.md` file.

For integrating the Twinkly LED control library into your projects, see the `glow-control-lib` directory and its `README.md` file for more details.

## Disclaimer

This project is not affiliated with, authorized by, endorsed by, or in any way officially connected with Twinkly or its affiliates. The official Twinkly website can be found at [https://www.twinkly.com](https://www.twinkly.com).

## Contributions

We welcome contributions from the community! If you have a suggestion, bug report, or a feature request, please open an issue or submit a pull request.

## License

This project is dual-licensed under the MIT License and the Apache License, Version 2.0. You may choose to use either license, depending on your project needs. See the `LICENSE-MIT` and `LICENSE-APACHE` files for the full text of the licenses.
