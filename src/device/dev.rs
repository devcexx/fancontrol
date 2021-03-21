use crate::types::{Percent, TempCelsius};

use std::fmt::Debug;
use std::io::Result;
use udev::Device as UdevDevice;

#[derive(Debug)]
pub enum PwmMode {
    Auto,
    Full,
    ManualPercent(Percent),
    ManualAbs(u8),
}

pub trait DeviceBuilder {
    fn from_udev(&self, name: String, device: UdevDevice, dryrun: bool) -> Box<dyn Device>;
}

pub trait Device: Debug {
    fn write_pwm(&self, index: u8, mode: PwmMode) -> Result<()>;
    fn read_temp(&self, index: u8) -> Result<TempCelsius>;
    // TODO Add fan_read / voltage_reaḋ for supporting other kind sources.
    fn name(&self) -> &str;
}
