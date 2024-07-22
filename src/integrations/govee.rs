use super::Integration;
use crate::utils::json::boolean_int;
use crate::{config::CuteLightsConfig, utils::future::FutureBatch};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::net::UdpSocket;

use super::Light;

// ANCHOR - GoveeLight
pub struct GoveeLight {
    udp_socket: Arc<UdpSocket>,
    device_addr: SocketAddr,
    is_on: bool,
    brightness: u8,
    red: u8,
    green: u8,
    blue: u8,
    id: String,
}

impl GoveeLight {
    pub async fn new(udp_socket: Arc<UdpSocket>, ip: &str) -> anyhow::Result<GoveeLight> {
        let device_addr = SocketAddr::new(IpAddr::V4(ip.parse()?), 4003);
        let mut dev = GoveeLight {
            udp_socket,
            device_addr,
            is_on: false,
            brightness: 0,
            red: 0,
            green: 0,
            blue: 0,
            id: ip.to_string(),
        };

        dev.refresh_state().await?;
        Ok(dev)
    }
}

#[async_trait::async_trait]
impl Light for GoveeLight {
    async fn refresh_state(&mut self) -> anyhow::Result<()> {
        let msg = Request::DevStatus {};
        let response = send_message(&self.udp_socket, &self.device_addr, msg, true).await?;

        let response = match response {
            Response::DevStatus(status) => status,
            _ => return Err(anyhow::anyhow!("Unexpected response")),
        };

        self.is_on = response.on;
        self.brightness = response.brightness as u8;
        self.red = response.color.r;
        self.green = response.color.g;
        self.blue = response.color.b;

        Ok(())
    }

    async fn set_on(&mut self, on: bool) -> anyhow::Result<()> {
        let msg = Request::Turn { value: on as u8 };
        send_message(&self.udp_socket, &self.device_addr, msg, false).await?;
        self.is_on = on;
        Ok(())
    }

    async fn set_color(&mut self, red: u8, green: u8, blue: u8) -> anyhow::Result<()> {
        let msg = Request::Color {
            color: DeviceColor {
                r: red,
                g: green,
                b: blue,
            },
        };
        send_message(&self.udp_socket, &self.device_addr, msg, false).await?;
        self.red = red;
        self.green = green;
        self.blue = blue;
        Ok(())
    }

    async fn set_brightness(&mut self, brightness: u8) -> anyhow::Result<()> {
        let msg = Request::Brightness {
            value: brightness as u8,
        };
        send_message(&self.udp_socket, &self.device_addr, msg, false).await?;
        self.brightness = brightness;
        Ok(())
    }

    fn id(&self) -> String {
        format!("govee::{}", self.id)
    }

    fn is_on(&self) -> bool {
        self.is_on
    }
    fn name(&self) -> String {
        format!("Govee Light ({})", self.id)
    }
    fn supports_color(&self) -> bool {
        true
    }

    fn red(&self) -> u8 {
        self.red
    }

    fn green(&self) -> u8 {
        self.green
    }

    fn blue(&self) -> u8 {
        self.blue
    }

    fn brightness(&self) -> u8 {
        self.brightness
    }
}

// ANCHOR - GoveeConfig

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct GoveeConfig {
    pub enabled: bool,
    pub addresses: Vec<String>,
    #[serde(default = "default_scan_timeout")]
    pub scan_timeout: u64,
}

fn default_scan_timeout() -> u64 {
    5000
}

// ANCHOR - GoveeIntegration

pub struct GoveeIntegration;

#[async_trait::async_trait]
impl Integration for GoveeIntegration {
    fn name() -> String {
        "govee".to_string()
    }
    async fn discover(config: &'static CuteLightsConfig) -> anyhow::Result<Vec<Box<dyn Light>>> {
        let mut batch = FutureBatch::new();
        let client_sock = Arc::new(UdpSocket::bind("0.0.0.0:4002").await?);

        for ip in &config.govee.addresses {
            let client_sock = client_sock.clone();
            batch.push(async move {
                match GoveeLight::new(client_sock.clone(), &ip).await {
                    Ok(light) => Some(Box::new(light) as Box<dyn Light>),
                    Err(e) => {
                        eprintln!("Failed to connect to Govee light at {}: {}", ip, e);
                        None
                    }
                }
            });
        }

        Ok(batch.run().await.into_iter().flatten().collect())
    }

    fn preflight(config: &CuteLightsConfig) -> bool {
        config.govee.enabled
    }
}
// ANCHOR - Messages

async fn send_message(
    sock: &UdpSocket,
    addr: &SocketAddr,
    data: Request,
    expect_response: bool,
) -> anyhow::Result<Response> {
    sock.send_to(
        serde_json::to_string(&RequestMessage { msg: data })?.as_bytes(),
        addr,
    )
    .await?;

    if !expect_response {
        return Ok(Response::Void);
    }
    let mut buf = [0; 1024];

    let (amt, _) = sock.recv_from(&mut buf).await?;

    let response: ResponseMessage = serde_json::from_str(&String::from_utf8_lossy(&buf[..amt]))?;

    Ok(response.msg)
}

#[derive(Debug, Deserialize, Serialize)]
pub enum AccountTopic {
    #[serde(rename = "reserve")]
    Reserve,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd", content = "data")]
pub enum Request {
    #[serde(rename = "scan")]
    Scan { topic: AccountTopic },
    #[serde(rename = "devStatus")]
    DevStatus {},
    #[serde(rename = "turn")]
    Turn { value: u8 },
    #[serde(rename = "brightness")]
    Brightness { value: u8 },
    #[serde(rename = "colorwc")]
    Color { color: DeviceColor },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cmd", content = "data")]
pub enum Response {
    #[serde(rename = "scan")]
    Scan(LanDevice),
    #[serde(rename = "devStatus")]
    DevStatus(DeviceStatus),
    Void,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DeviceStatus {
    #[serde(rename = "onOff", deserialize_with = "boolean_int")]
    pub on: bool,
    pub brightness: u8,
    pub color: DeviceColor,
    #[serde(rename = "colorTemInKelvin")]
    pub color_temperature_kelvin: u32,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct DeviceColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LanDevice {
    pub ip: IpAddr,
    pub device: String,
    pub sku: String,
    #[serde(rename = "bleVersionHard")]
    pub ble_version_hard: String,
    #[serde(rename = "bleVersionSoft")]
    pub ble_version_soft: String,
    #[serde(rename = "wifiVersionHard")]
    pub wifi_version_hard: String,
    #[serde(rename = "wifiVersionSoft")]
    pub wifi_version_soft: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestMessage {
    msg: Request,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseMessage {
    msg: Response,
}
