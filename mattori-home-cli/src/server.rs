use std::convert::TryFrom;
use std::fmt::{Debug, Display};
use std::pin::Pin;

use tokio::sync::Mutex;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::{Stream, StreamExt};

use mattori_home::home_server::Home;
use mattori_home::{AcStatus, AcStatusParam, AtmosphereReading};
use mattori_home_peripherals::atmosphere::Atmosphere;
use mattori_home_peripherals::ir::output::IrOut;
use mattori_home_peripherals::ir::types::{ACMode, IrTarget};

pub mod mattori_home {
    tonic::include_proto!("mattori_home");
}

#[derive(Debug)]
pub struct HomeServer<T: IrTarget + Debug + Send + Sync + 'static>
where
    <<T as IrTarget>::Temperature as TryFrom<u32>>::Error: Display,
{
    atmosphere: Atmosphere,
    ir_out: Mutex<IrOut<T>>,
}

#[tonic::async_trait]
impl<T: IrTarget + Debug + Send + Sync + 'static> Home for HomeServer<T>
where
    <<T as IrTarget>::Temperature as TryFrom<u32>>::Error: Display,
{
    type ReadAtmosphereStream = Pin<
        Box<dyn Stream<Item = Result<AtmosphereReading, tonic::Status>> + Send + Sync + 'static>,
    >;

    async fn read_atmosphere(
        &self,
        request: tonic::Request<tonic::Streaming<mattori_home::AtmosphereFeatures>>,
    ) -> Result<tonic::Response<Self::ReadAtmosphereStream>, tonic::Status> {
        let mut feature_stream = request.into_inner();
        let reading_stream = WatchStream::new(self.atmosphere.subscribe()).map(|res| {
            res.map(mattori_home::AtmosphereReading::from)
                .map_err(|e| tonic::Status::internal(e.to_string()))
        });

        tokio::spawn(async move {
            while let Some(_) = feature_stream.next().await {
                // todo implement
            }
        });

        Ok(tonic::Response::new(
            Box::pin(reading_stream) as Self::ReadAtmosphereStream
        ))
    }

    async fn get_ac_status(
        &self,
        _: tonic::Request<AcStatusParam>,
    ) -> Result<tonic::Response<AcStatus>, tonic::Status> {
        Ok(tonic::Response::new(
            self.ir_out.lock().await.status().into(),
        ))
    }

    async fn set_ac_status(
        &self,
        request: tonic::Request<AcStatus>,
    ) -> Result<tonic::Response<AcStatus>, tonic::Status> {
        let new_status = request.into_inner();
        let new_powered = new_status.powered;
        let powered_change = self.ir_out.lock().await.status().powered != new_powered;
        let new_mode = ACMode::from(new_status.mode());
        let new_temperature = T::Temperature::try_from(new_status.temperature)
            .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        self.ir_out
            .lock()
            .await
            .send_target(move |target| {
                target.mode_set(new_mode)?;
                let temp_set_sequence = target.temp_set(new_temperature)?;
                if powered_change {
                    if new_powered {
                        target.power_on()
                    } else {
                        target.power_off()
                    }
                } else {
                    Ok(temp_set_sequence)
                }
            })
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        Ok(tonic::Response::new(
            self.ir_out.lock().await.status().into(),
        ))
    }
}
