# Introduction

_by Chris Gorski and the `glow-control` team_

The Rust crates `glow-control` and `glow-control-lib` are a CLI and a library, respectively,
for controlling Twinkly lights. These unoffical tools, not affiliated with Twinkly,
are designed to provide a flexible and powerful interface for those who are
looking for alternatives to the official Twinkly app.

Twinkly lights elegantly solve a problem with programmable LED arrays:
the mapping of LEDs to physical locations. The Twinkly app allows users to
do this by taking video of the lights, repeatedly, and from different angles,
such that the app can derive the mapping of the LEDs. This mapping is stored
on the Twinkly devices themselves, and is accessible over TCP/IP.

The `glow-control` tools have the following features:

- Network-based discovery of Twinkly devices
- Easy integration with any app that can pipe output to the CLI
- High-level control interfaces for managing device modes and settings
- Real-time effect control from an external network device
- Custom LED movie uploads
- Utility functions for device authentication and communication

