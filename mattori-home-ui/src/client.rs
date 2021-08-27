use eyre::{eyre, Result, WrapErr};

use futures_util::TryStreamExt;
use mattori_home::home_client::HomeClient;
use mattori_home::{AcStatus, AcStatusParam, AtmosphereFeatures, AtmosphereReading};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::{Request, Response, Streaming};

pub mod mattori_home {
    tonic::include_proto!("mattori_home");
}

pub enum ClientMessage {
    ChangeAtmosphereFeatures(AtmosphereFeatures),
    GetAcStatus,
    SetAcStatus(AcStatus),
    Stop,
}

pub struct Client {
    client: HomeClient<Channel>,
    atmo_features: Arc<Mutex<AtmosphereFeatures>>,
}

impl Client {
    pub async fn new(addr: String) -> Result<Client> {
        Ok(Client {
            client: HomeClient::connect(addr).await?,
            atmo_features: Arc::new(Mutex::new(AtmosphereFeatures {
                temperature: true,
                pressure: true,
                humidity: true,
                altitude: true,
            })),
        })
    }

    pub fn set_temperature(&mut self, val: bool) -> Result<()> {
        self.atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))?
            .temperature = val;
        Ok(())
    }

    pub fn set_pressure(&mut self, val: bool) -> Result<()> {
        self.atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))?
            .pressure = val;
        Ok(())
    }

    pub fn set_humidity(&mut self, val: bool) -> Result<()> {
        self.atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))?
            .humidity = val;
        Ok(())
    }

    pub fn set_altitude(&mut self, val: bool) -> Result<()> {
        self.atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))?
            .altitude = val;
        Ok(())
    }

    pub fn atmosphere_features(&self) -> Result<AtmosphereFeatures> {
        self.atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))
            .map(|g| g.clone())
    }

    pub fn set_atmosphere_features(&mut self, features: AtmosphereFeatures) -> Result<()> {
        *self
            .atmo_features
            .lock()
            .map_err(|_| eyre!("Could not lock atmosphere features mutex"))? = features;
        Ok(())
    }

    pub async fn start_read_atmosphere(&mut self) -> Result<Streaming<AtmosphereReading>> {
        let outbound = {
            let features = self.atmo_features.clone();
            futures_util::stream::repeat_with(move || {
                features
                    .lock()
                    .map_err(|_| eyre!("Could not lock atmosphere features mutex"))
                    .map(|g| g.clone())
            })
        }
        .inspect_err(|e| error!("Could not send atmosphere reading feature to server: {}", e))
        .filter_map(Result::ok)
        .throttle(Duration::from_secs(1));
        let stream = self
            .client
            .read_atmosphere(Request::new(outbound))
            .await
            .wrap_err("Could not receive atmosphere reading from server")?
            .into_inner();
        Ok(stream)
    }

    pub async fn get_ac_status(&mut self) -> Result<AcStatus> {
        self.client
            .get_ac_status(Request::new(AcStatusParam {}))
            .await
            .wrap_err("Could not receive AC status from server")
            .map(Response::into_inner)
    }

    pub async fn set_ac_status(&mut self, status: AcStatus) -> Result<AcStatus> {
        self.client
            .set_ac_status(Request::new(status))
            .await
            .wrap_err("Could not send AC status to server")
            .map(Response::into_inner)
    }
}
