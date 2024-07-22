use std::{net::IpAddr, sync::Arc};

use openrgb::OpenRGB;
use serde::{Deserialize, Serialize};

use crate::config::CuteLightsConfig;

use super::Light;

pub struct OpenRgbLight {
    controller_id: u32,
    controller: openrgb::data::Controller,
    client: Arc<OpenRGB<tokio::net::TcpStream>>,
}
#[async_trait::async_trait]
impl Light for OpenRgbLight {
    async fn refresh_state(&mut self) -> anyhow::Result<()> {
        self.controller = self.client.get_controller(self.controller_id).await?;
        Ok(())
    }

    async fn set_color(&mut self, red: u8, green: u8, blue: u8) -> anyhow::Result<()> {
        let new_colors =
            vec![openrgb::data::Color::new(red, green, blue); self.controller.colors.len()];
        self.client
            .update_leds(self.controller_id, new_colors)
            .await?;
        Ok(())
    }

    async fn set_on(&mut self, _on: bool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn set_brightness(&mut self, _brightness: u8) -> anyhow::Result<()> {
        Ok(())
    }

    fn id(&self) -> String {
        format!("openrgb::{}", self.controller_id)
    }

    fn brightness(&self) -> u8 {
        100
    }

    fn red(&self) -> u8 {
        self.controller.colors[0].b
    }

    fn green(&self) -> u8 {
        self.controller.colors[0].g
    }

    fn blue(&self) -> u8 {
        self.controller.colors[0].r
    }

    fn is_on(&self) -> bool {
        true
    }

    fn name(&self) -> String {
        self.controller.name.clone()
    }

    fn supports_color(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenRgbConfig {
    pub enabled: bool,
    pub address: String,
    pub port: u16,
}

impl Default for OpenRgbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            address: "localhost".to_string(),
            port: 6742,
        }
    }
}

pub struct OpenRgbIntegration;

#[async_trait::async_trait]
impl super::Integration for OpenRgbIntegration {
    fn name() -> String {
        "openrgb".to_string()
    }

    async fn discover(config: &'static CuteLightsConfig) -> anyhow::Result<Vec<Box<dyn Light>>> {
        let address: IpAddr = config.openrgb.address.parse()?;

        let client = OpenRGB::connect_to((address, config.openrgb.port)).await?;
        let client = Arc::new(client);

        let mut lights = vec![];

        for controller_id in 0..client.get_controller_count().await? {
            let controller = client.get_controller(controller_id).await?;
            lights.push(Box::new(OpenRgbLight {
                controller_id,
                controller,
                client: client.clone(),
            }) as Box<dyn Light>);
        }

        Ok(lights)
    }

    fn preflight(config: &CuteLightsConfig) -> bool {
        config.openrgb.enabled
    }
}
