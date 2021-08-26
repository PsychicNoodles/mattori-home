use std::convert::TryFrom;
use std::fmt::Display;

use crate::server::mattori_home;
use mattori_home_peripherals::atmosphere::{AtmosphereFeatures, Reading};
use mattori_home_peripherals::ir::types::{ACMode, IrStatus, IrTarget};

impl From<mattori_home::AtmosphereFeatures> for AtmosphereFeatures {
    fn from(
        mattori_home::AtmosphereFeatures {
            temperature,
            pressure,
            humidity,
            altitude,
        }: mattori_home::AtmosphereFeatures,
    ) -> Self {
        AtmosphereFeatures {
            temperature,
            pressure,
            humidity,
            altitude,
        }
    }
}

impl From<ACMode> for mattori_home::ac_status::Mode {
    fn from(mode: ACMode) -> Self {
        match mode {
            ACMode::Auto => mattori_home::ac_status::Mode::Auto,
            ACMode::Warm => mattori_home::ac_status::Mode::Warm,
            ACMode::Dry => mattori_home::ac_status::Mode::Dry,
            ACMode::Cool => mattori_home::ac_status::Mode::Cool,
            ACMode::Fan => mattori_home::ac_status::Mode::Fan,
        }
    }
}

impl From<mattori_home::ac_status::Mode> for ACMode {
    fn from(mode: mattori_home::ac_status::Mode) -> Self {
        match mode {
            mattori_home::ac_status::Mode::Auto => ACMode::Auto,
            mattori_home::ac_status::Mode::Warm => ACMode::Warm,
            mattori_home::ac_status::Mode::Dry => ACMode::Dry,
            mattori_home::ac_status::Mode::Cool => ACMode::Cool,
            mattori_home::ac_status::Mode::Fan => ACMode::Fan,
        }
    }
}

impl<T: IrTarget> From<IrStatus<T>> for mattori_home::AcStatus
where
    <<T as IrTarget>::Temperature as TryFrom<u32>>::Error: Display,
{
    fn from(
        IrStatus {
            powered,
            mode,
            temperature,
        }: IrStatus<T>,
    ) -> Self {
        let mut ac_status = mattori_home::AcStatus {
            powered,
            temperature: temperature.into(),
            ..mattori_home::AcStatus::default()
        };
        ac_status.set_mode(mode.into());
        ac_status
    }
}

impl From<Reading> for mattori_home::AtmosphereReading {
    fn from(
        Reading {
            temperature,
            pressure,
            humidity,
            altitude,
        }: Reading,
    ) -> Self {
        mattori_home::AtmosphereReading {
            temperature: temperature.unwrap_or_default(),
            pressure: pressure.unwrap_or_default(),
            humidity: humidity.unwrap_or_default(),
            altitude: altitude.unwrap_or_default(),
        }
    }
}
