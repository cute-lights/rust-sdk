use crate::{
    config::CuteLightsConfig,
    integrations::{
        govee::GoveeIntegration, hue::HueIntegration, kasa::KasaIntegration,
        openrgb::OpenRgbIntegration, Integration, Light,
    },
    utils::future::FutureBatch,
};

struct Discoverer {
    config: &'static CuteLightsConfig,
    batch: FutureBatch<Vec<Box<dyn Light>>>,
}

impl Discoverer {
    fn new(config: &'static CuteLightsConfig) -> Self {
        Self {
            config,
            batch: FutureBatch::new(),
        }
    }

    fn register<I: Integration + Send + Sync + 'static>(&mut self) {
        let config = self.config;
        if I::preflight(&self.config) {
            self.batch.push(async move {
                match I::discover(&config).await {
                    Ok(lights) => lights,
                    Err(e) => {
                        eprintln!("Failed to discover lights for {}: {}", I::name(), e);
                        Vec::new()
                    }
                }
            });
        }
    }

    async fn run(self) -> Vec<Box<dyn Light>> {
        self.batch.run().await.into_iter().flatten().collect()
    }
}

pub async fn discover_lights() -> Vec<Box<dyn Light>> {
    let config = Box::leak(Box::new(CuteLightsConfig::load_default()));
    let mut discoverer = Discoverer::new(config);

    discoverer.register::<KasaIntegration>();
    discoverer.register::<HueIntegration>();
    discoverer.register::<GoveeIntegration>();
    discoverer.register::<OpenRgbIntegration>();

    discoverer.run().await
}
