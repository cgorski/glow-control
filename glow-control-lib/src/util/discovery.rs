use std::cmp::max;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::net::Ipv4Addr;
use std::time::Duration;

use anyhow::Context;
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Instant};

use derivative::Derivative;

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

#[derive(Derivative)]
#[derivative(Hash, PartialEq, Eq, PartialOrd)]
#[derive(Debug, Clone, Serialize)]
pub struct DeviceIdentifier {
    pub ip_address: Ipv4Addr,
    pub device_id: String,
    pub mac_address: String,
    pub device_name: String,
    pub led_count: u16,

    /**
    The auth-token if the device was authenticated.

    If the device was found by a search, the auth_token must
    be generated to pull info from the device, and then that new token must be used.

    **Too frequent token generation _can_ lead to erroneous behavior.**
    */
    #[derivative(Hash = "ignore", PartialEq = "ignore", PartialOrd = "ignore")]
    pub auth_token: Option<String>,
}

impl DeviceIdentifier {
    pub fn new(
        ip_address: Ipv4Addr,
        device_id: String,
        mac_address: String,
        device_name: String,
        led_count: u16,
        auth_token: Option<String>,
    ) -> Self {
        DeviceIdentifier {
            ip_address,
            device_id,
            mac_address,
            device_name,
            led_count,
            auth_token,
        }
    }
}

pub struct Discovery;

/**
The response from the discovery request.

It includes the newly found devices, and the existing devices
(if the `existing_devices` argument has been supplied to
[`Discovery::find_new_devices`]).
*/
pub struct ResponseNewExisting {
    /// Newly found devices.
    pub new_devices: HashSet<DeviceIdentifier>,
    
    /**
    Existing devices, which have been re-discovered.
    Used by [`Self::find_new_devices`] if the `existing_devices` argument  has been supplied.
     */
    pub existing_devices: HashSet<DeviceIdentifier>,
}

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

    pub async fn find_devices(given_timeout: Duration) -> anyhow::Result<HashSet<DeviceIdentifier>> {
        Self::find_new_devices(given_timeout, None).await
            .map(|devices: ResponseNewExisting| devices.new_devices)
    }

    /**
    Finds new devices on the network.

    Skips devices which are already in `existing_devices` and reports them in [`ResponseNewExisting::existing_devices`].
    Newly found devices are reported in [`ResponseNewExisting::new_devices`].
     */
    pub async fn find_new_devices(
        given_timeout: Duration,
        existing_devices: Option<HashSet<DeviceIdentifier>>
    ) -> anyhow::Result<ResponseNewExisting> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        socket.send_to(PING_MESSAGE, BROADCAST_ADDRESS).await?;

        let mut discovered_devices = HashSet::<DeviceIdentifier>::new();
        let mut buffer = [0; 1024];

        let timeout_end = Instant::now() + given_timeout;

        let mut found_existing_devices = HashSet::<DeviceIdentifier>::new();

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
                        /*
                         Look first if the device (i.e. address) is already in `discovered_devices`,
                         and skip it, if it is, that makes the discovery process faster.

                         It also saves the needless re-authentication of the device, which saves additional time.

                         Cause: Some devices may respond multiple times for one request, to make sure the listener
                                gets it.
                         */
                        // Search if `discovered_devices` matches a `discovery_response`:
                        if Self::find_discovered_device(&discovered_devices, &discovery_response).is_some() {
                            info!("Found device {:?} again, skipping", discovery_response);
                            continue;
                        }
                        // Search if `existing_devices` matches a `discovery_response`:
                        if let Some(existing_devices) = &existing_devices {
                            if let Some(exist) = Self::find_discovered_device(&existing_devices, &discovery_response) {
                                found_existing_devices.insert(exist);
                                info!("Device {:?} isn't new, skipping", discovery_response);
                                continue;
                            }
                        }
                        info!("Found device: {:?}", discovery_response);
                        match Self::fetch_gestalt_info(discovery_response.ip_address).await {
                            Ok(gestalt_info) => {
                                info!("MAC address: {}", gestalt_info);
                                // Fetch the LED count from a high control interface
                                let high_control_interface = ControlInterface::new(
                                    &discovery_response.ip_address.to_string(),
                                    &gestalt_info.mac,
                                    None,
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
                                    // Reuse the auth token from the high control interface to speed up authentication.
                                    Some(high_control_interface.auth_token),
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

        Ok(ResponseNewExisting { new_devices: discovered_devices, existing_devices: found_existing_devices })
    }

    /// Returns if `discovery_response` is in the Set of `devices`.
    fn find_discovered_device(devices: &HashSet<DeviceIdentifier>, discovery_response: &DiscoveryResponse) -> Option<DeviceIdentifier> {
        let filtered: Vec<DeviceIdentifier> = devices.iter().filter(|device_identifier: &&DeviceIdentifier| {
            device_identifier.device_id == discovery_response.device_id
                && device_identifier.ip_address == discovery_response.ip_address
        }).cloned().collect();
        match filtered.len() {
            0 => None,
            1 => filtered.first().cloned(),
            _ => {
                error!("Found multiple devices with the same IP address {} and device ID {}",
                    discovery_response.ip_address, discovery_response.device_id);
                None
            },
        }
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
