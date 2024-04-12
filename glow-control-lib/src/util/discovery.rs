use std::cmp::max;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Instant};

use crate::control_interface::ControlInterface;

const PING_MESSAGE: &[u8] = b"\x01discover";
const BROADCAST_ADDRESS: &str = "255.255.255.255:5555";

#[derive(Deserialize, Debug)]
pub struct GestaltResponse {
    mac: String,
    device_name: String,
    // Include other fields from the response as needed
}

impl Display for GestaltResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "MAC: {}, Device Name: {}", self.mac, self.device_name)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct DiscoveryResponse {
    ip_address: Ipv4Addr,
    device_id: String,
}

impl DiscoveryResponse {
    pub fn new(ip_address: Ipv4Addr, device_id: String) -> Self {
        DiscoveryResponse {
            ip_address,
            device_id,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Clone, Serialize)]
pub struct DeviceIdentifier {
    pub ip_address: Ipv4Addr,
    pub device_id: String,
    pub mac_address: String,
    pub device_name: String,
    pub led_count: u16,
}

impl DeviceIdentifier {
    pub fn new(
        ip_address: Ipv4Addr,
        device_id: String,
        mac_address: String,
        device_name: String,
        led_count: u16,
    ) -> Self {
        DeviceIdentifier {
            ip_address,
            device_id,
            mac_address,
            device_name,
            led_count,
        }
    }
}

pub struct Discovery;

impl Discovery {
    pub fn decode_discovery_response(data: &[u8]) -> Option<DiscoveryResponse> {
        // Check if the response is at least 8 bytes long and ends with a zero byte
        if data.len() < 8 || *data.last().unwrap() != 0 {
            return None;
        }

        // Check if the response contains "OK" status
        if data[4..6] != [b'O', b'K'] {
            return None;
        }

        // Extract the IP address from the response
        let ip_address = Ipv4Addr::new(data[3], data[2], data[1], data[0]);

        // Extract the device ID from the response, which starts at byte 6 and ends before the last byte
        let device_id_bytes = &data[6..data.len() - 1];
        let device_id = match std::str::from_utf8(device_id_bytes) {
            Ok(v) => v.to_string(),
            Err(_) => return None,
        };

        // Return the struct with the IP address object and device ID
        Some(DiscoveryResponse {
            ip_address,
            device_id,
        })
    }

    pub async fn find_devices(
        given_timeout: Duration,
    ) -> anyhow::Result<HashSet<DeviceIdentifier>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        socket.send_to(PING_MESSAGE, BROADCAST_ADDRESS).await?;

        let mut discovered_devices = HashSet::new();
        let mut buffer = [0; 1024];

        let timeout_end = Instant::now() + given_timeout;

        loop {
            if Instant::now() >= timeout_end {
                break;
            }

            let remaining_time = timeout_end - Instant::now();
            let result = timeout(remaining_time, socket.recv_from(&mut buffer)).await;

            match result {
                Ok(Ok((number_of_bytes, _src_addr))) => {
                    let received_data = &buffer[..number_of_bytes];
                    if let Some(discovery_response) = Self::decode_discovery_response(received_data)
                    {
                        info!("Found device: {:?}", discovery_response);
                        match Self::fetch_gestalt_info(discovery_response.ip_address).await {
                            Ok(gestalt_info) => {
                                info!("MAC address: {}", gestalt_info);
                                // Fetch the LED count from a high control interface
                                let high_control_interface = ControlInterface::new(
                                    &discovery_response.ip_address.to_string(),
                                    &gestalt_info.mac,
                                )
                                .await?;
                                let led_count =
                                    high_control_interface.get_device_info().number_of_led as u16;
                                let device = DeviceIdentifier::new(
                                    discovery_response.ip_address,
                                    discovery_response.device_id,
                                    gestalt_info.mac,
                                    gestalt_info.device_name,
                                    led_count,
                                );
                                discovered_devices.insert(device);
                            }
                            Err(e) => eprintln!("Error fetching MAC address: {:?}", e),
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Failed to receive response: {}", e);
                    break;
                }
                Err(_) => {
                    eprintln!("Discovery time complete. If devices are missing, try increasing the search timeout.");
                    break;
                }
            }
        }

        Ok(discovered_devices)
    }
    async fn fetch_gestalt_info(ip_address: Ipv4Addr) -> anyhow::Result<GestaltResponse> {
        let url = format!("http://{}/xled/v1/gestalt", ip_address);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to device")?;

        if response.status().is_success() {
            let gestalt: GestaltResponse = response
                .json()
                .await
                .context("Failed to parse JSON response")?;
            Ok(gestalt)
        } else {
            Err(anyhow::anyhow!(
                "Received non-success status code: {}",
                response.status()
            ))
        }
    }
    pub fn pretty_print_devices(devices: &HashSet<DeviceIdentifier>) {
        // Determine the maximum width for each column
        let max_ip_width = devices
            .iter()
            .map(|d| d.ip_address.to_string().len())
            .max()
            .unwrap_or(0);
        let max_device_id_width = devices.iter().map(|d| d.device_id.len()).max().unwrap_or(0);
        let max_mac_width = devices
            .iter()
            .map(|d| d.mac_address.len())
            .max()
            .unwrap_or(0);
        let max_device_name_width = devices
            .iter()
            .map(|d| max(d.device_name.len(), 20))
            .max()
            .unwrap_or(0);

        let max_led_count_width = devices
            .iter()
            .map(|d| d.led_count.to_string().len())
            .max()
            .unwrap_or(0);

        // Print the header with appropriate spacing
        println!(
            "{:<ip_width$} {:<device_id_width$} {:<mac_width$} {:<device_name_width$} {:<led_count_width$}",
            "IP Address",
            "Device ID",
            "MAC Address",
            "Device Name",
            "LED Count",
            ip_width = max_ip_width + 2, // Add some padding
            device_id_width = max_device_id_width + 2,
            mac_width = max_mac_width + 2,
            device_name_width = max_device_name_width + 2,
            led_count_width = max_led_count_width + 2,
        );

        // Print the separator line
        println!(
            "{:<ip_width$} {:<device_id_width$} {:<mac_width$} {:<device_name_width$} {:<led_count_width$}",
            "-".repeat(max_ip_width),
            "-".repeat(max_device_id_width),
            "-".repeat(max_mac_width),
            "-".repeat(max_device_name_width),
            "-".repeat(max_led_count_width),
            ip_width = max_ip_width + 2,
            device_id_width = max_device_id_width + 2,
            mac_width = max_mac_width + 2,
            device_name_width = max_device_name_width + 2,
            led_count_width = max_led_count_width + 2,
        );

        // Print each device entry with appropriate spacing
        for device in devices {
            println!(
                "{:<ip_width$} {:<device_id_width$} {:<mac_width$} {:<device_name_width$} {:<led_count_width$}",
                device.ip_address,
                device.device_id,
                device.mac_address,
                device.device_name,
                device.led_count,
                ip_width = max_ip_width + 2,
                device_id_width = max_device_id_width + 2,
                mac_width = max_mac_width + 2,
                device_name_width = max_device_name_width + 2,
                led_count_width = max_led_count_width + 2,
            );
        }
    }
}
