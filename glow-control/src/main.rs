use std::collections::HashSet;
use std::time::Duration;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};

use glow_control_lib::control_interface::{
    CliColors, CliDeviceMode, ControlInterface, RtStdinErrorMode, RtStdinFormat, RGB,
};
use glow_control_lib::util::discovery::Discovery;

// Function to generate a random challenge

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    handle_cli(cli).await
}

// src/cli.rs

/// This struct defines the command line interface of the application
#[derive(Parser)]
#[clap(
    name = "glow_control",
    about = "Controls commercial LED devices",
    version = "0.3.3"
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

/// Supported output formats for the `discover` command.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputFormat {
    /// Plain text format.
    Plaintext,
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
}

/// Subcommands available for the CLI
#[derive(Subcommand)]
pub enum Commands {
    /// Subcommand for operations that require device communication
    #[clap(name = "device-call")]
    DeviceCall {
        /// Sets the IP address of the Twinkly device
        #[clap(long)]
        ip: String,

        /// Sets the MAC address of the Twinkly device
        #[clap(long)]
        mac: String,

        #[clap(subcommand)]
        action: DeviceAction,
    },
    /// Subcommand for operations that require device communication
    #[clap(name = "discover")]
    Discover {
        /// Output format (plaintext, json, yaml)
        #[clap(short, long, value_enum, default_value_t = OutputFormat::Plaintext)]
        output: OutputFormat,

        /// Search timeout in milliseconds
        #[clap(short = 't', long = "timeout", default_value_t = 5000)]
        timeout: u64,
    },
}

/// Real-time effects that can be applied to the device.
#[derive(Subcommand)]
pub enum RtEffect {
    /// Shows a solid color using the real-time functionality.
    #[clap(name = "show-color")]
    ShowColor {
        /// The color to display by name
        #[clap(value_enum)]
        color: Option<CliColors>,

        /// Red component of the color (0-255)
        #[clap(short = 'r', long = "red", value_parser = clap::value_parser!(u8))]
        red: Option<u8>,

        /// Green component of the color (0-255)
        #[clap(short = 'g', long = "green", value_parser = clap::value_parser!(u8))]
        green: Option<u8>,

        /// Blue component of the color (0-255)
        #[clap(short = 'b', long = "blue", value_parser = clap::value_parser!(u8))]
        blue: Option<u8>,
    },
    Shine {
        /// The number of LEDs that should start glowing simultaneously
        #[clap(long)]
        num_start_simultaneous: usize,

        /// The set of colors to use for the brightest state
        #[clap(long, use_value_delimiter = true)]
        colors: Vec<CliColors>,

        /// The time between starting the glow of each LED
        #[clap(long, value_parser = parse_duration)]
        time_between_glow_start: Duration,

        /// The time for an LED to reach maximum brightness
        #[clap(long, value_parser = parse_duration)]
        time_to_max_glow: Duration,

        /// The time for an LED to fade to off
        #[clap(long, value_parser = parse_duration)]
        time_to_fade: Duration,

        /// The frame rate for updating LED states
        #[clap(long)]
        frame_rate: f64,
    },
}

fn parse_duration(s: &str) -> Result<Duration, &'static str> {
    let millis = s
        .parse::<u64>()
        .map_err(|_| "could not parse duration in milliseconds")?;
    Ok(Duration::from_millis(millis))
}
/// Actions available under the `device-call` subcommand
#[derive(Subcommand)]
pub enum DeviceAction {
    /// Gets current device mode.
    #[clap(name = "get-mode")]
    GetMode,
    /// Gets current timer settings.
    #[clap(name = "get-timer")]
    GetTimer,
    /// Gets the current playlist.
    #[clap(name = "get-playlist")]
    GetPlaylist,
    /// Sets the device mode.
    #[clap(name = "set-mode")]
    SetMode {
        /// The mode to set the device to
        #[clap(value_enum)]
        mode: CliDeviceMode,
    },
    /// Fetches the LED layout.
    #[clap(name = "fetch-layout")]
    FetchLayout,
    /// Clears all uploaded movies from the device.
    #[clap(name = "clear-movies")]
    ClearMovies,

    /// Retrieves the device's capacity for movies.
    #[clap(name = "get-device-capacity")]
    GetDeviceCapacity,

    /// Print the device's configuration
    #[clap(name = "print-config")]
    PrintConfig,
    /// Tests the real-time color wheel display.
    #[clap(name = "real-time-test")]
    RealTimeTest,
    /// Subcommand for real-time effects.
    #[clap(name = "rt-effect")]
    RtEffect {
        #[clap(subcommand)]
        effect: RtEffect,
    },
    #[clap(name = "rt-stdin")]
    RtStdin {
        /// The format of the input stream
        #[clap(long, value_enum)]
        format: RtStdinFormat,

        /// The error mode for the input stream
        #[clap(long, value_enum)]
        error_mode: RtStdinErrorMode,

        /// LEDs to read before writing to the device
        #[clap(long, value_enum)]
        leds_per_frame: u16,

        /// Minimum time between frames in milliseconds
        #[clap(long, value_parser = parse_duration)]
        min_frame_duration: Duration,
    },
}

async fn handle_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Discover { output, timeout } => {
            let devices = Discovery::find_devices(Duration::from_millis(timeout)).await?;
            match output {
                OutputFormat::Plaintext => {
                    Discovery::pretty_print_devices(&devices);
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string(&devices)?;
                    println!("{}", json);
                }
                OutputFormat::Yaml => {
                    let yaml = serde_yaml::to_string(&devices)?;
                    println!("{}", yaml);
                }
            }
        }
        Commands::DeviceCall { ip, mac, action } => {
            let high_control_interface = ControlInterface::new(&ip, &mac, None).await?;

            match action {
                DeviceAction::GetMode => {
                    let mode_response = high_control_interface.get_mode().await?;
                    println!("Current mode: {}", mode_response);
                }
                DeviceAction::GetTimer => {
                    let timer_response = high_control_interface.get_timer().await?;
                    println!("Current timer settings:");
                    println!("Time now: {}", timer_response.time_now);
                    println!("Time to turn on: {}", timer_response.time_on);
                    println!("Time to turn off: {}", timer_response.time_off);
                }
                DeviceAction::GetPlaylist => {
                    let playlist_response = high_control_interface.get_playlist().await?;
                    println!("Current playlist:");
                    for entry in playlist_response.entries {
                        println!("ID: {}, Name: {}", entry.id, entry.name);
                    }
                }
                DeviceAction::SetMode { mode } => {
                    high_control_interface.set_mode(mode.into()).await?;
                    println!("Device mode set to {:?}", mode);
                }

                DeviceAction::FetchLayout => {
                    let layout = high_control_interface.fetch_layout().await?;
                    println!("LED layout fetched: {:?}", layout);
                }

                DeviceAction::ClearMovies => {
                    high_control_interface.clear_movies().await?;
                    println!("All movies have been cleared from the device.");
                }

                DeviceAction::GetDeviceCapacity => {
                    let capacity = high_control_interface.get_device_capacity().await?;
                    println!("Device capacity for movies: {}", capacity);
                }
                DeviceAction::PrintConfig => {
                    let device_info = high_control_interface.get_device_info();
                    println!("The device configuration:\n{:#?}", device_info);
                }
                DeviceAction::RealTimeTest => {
                    high_control_interface
                        .show_real_time_test_color_wheel(0.02, 20.0)
                        .await?;
                    println!("Real-time test color wheel displayed.");
                }
                DeviceAction::RtEffect { effect } => {
                    match effect {
                        RtEffect::ShowColor {
                            color,
                            red,
                            green,
                            blue,
                        } => {
                            let color_to_show = match (color, red, green, blue) {
                                (Some(color_name), None, None, None) => color_name.into(),
                                (None, Some(r), Some(g), Some(b)) => RGB {
                                    red: r,
                                    green: g,
                                    blue: b,
                                },
                                _ => return Err(anyhow!("Invalid color specification")),
                            };

                            high_control_interface
                                .show_solid_color(color_to_show)
                                .await?;
                            println!("Displayed color: {:?}", color_to_show);
                        }
                        RtEffect::Shine {
                            num_start_simultaneous,
                            time_between_glow_start,
                            time_to_max_glow,
                            time_to_fade,
                            colors,
                            frame_rate,
                        } => {
                            // Convert the list of CliColors to a HashSet of RGB
                            let color_set: HashSet<RGB> =
                                colors.into_iter().map(Into::into).collect();

                            // Check if the color set is not empty
                            if color_set.is_empty() {
                                return Err(anyhow!("At least one color must be specified"));
                            }
                            high_control_interface
                                .shine_leds(
                                    time_between_glow_start,
                                    time_to_max_glow,
                                    time_to_fade,
                                    color_set,
                                    frame_rate,
                                    num_start_simultaneous,
                                )
                                .await?;
                            println!("Shine effect started.");
                        }
                    }
                }
                DeviceAction::RtStdin {
                    format,
                    error_mode,
                    leds_per_frame,
                    min_frame_duration: min_frame_time,
                } => {
                    high_control_interface
                        .show_real_time_stdin_stream(
                            format,
                            error_mode,
                            leds_per_frame,
                            min_frame_time,
                        )
                        .await?;
                }
            }
        }
    }

    Ok(())
}
