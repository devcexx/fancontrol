use std::io::Result;

use super::hwmon::HwmonDevice;
use crate::dev_debug;
use crate::{
    device::{Device, DeviceBuilder, PwmMode},
    types::TempCelsius,
};
use udev::Device as UdevDevice;

const NCT6775_PWM_MODE_FULL: &str = "0";
const NCT6775_PWM_MODE_MANUAL: &str = "1";
const NCT6775_PWM_MODE_AUTO: &str = "5";

pub struct Builder;

impl DeviceBuilder for Builder {
    fn from_udev(&self, name: String, device: UdevDevice, dryrun: bool) -> Box<dyn Device> {
        Box::new(Nct6775Device::from_udev(name, device, dryrun))
    }
}

#[derive(new, Debug)]
pub struct Nct6775Device {
    name: String,
    device: HwmonDevice,
}

impl Nct6775Device {
    fn from_udev(name: String, device: UdevDevice, dryrun: bool) -> Nct6775Device {
        Nct6775Device::new(name.clone(), HwmonDevice::from_udev(name, device, dryrun))
    }
}

impl Device for Nct6775Device {
    fn write_pwm(&self, index: u8, mode: PwmMode) -> std::io::Result<()> {
        match mode {
            PwmMode::Auto => self.device.write_pwm_enable(index, NCT6775_PWM_MODE_AUTO),
            PwmMode::Full => self.device.write_pwm_enable(index, NCT6775_PWM_MODE_FULL),
            PwmMode::ManualPercent(percent) => {
                dev_debug!(self, "Request PWM {} set to {}.", index, percent);
                self.device.write_pwm_enable_and_value(
                    index,
                    NCT6775_PWM_MODE_MANUAL,
                    percent.point_at_range(0u8, 255u8),
                )
            }
            PwmMode::ManualAbs(value) => self.device.write_raw_pwm(index, value),
        }
    }

    fn read_temp(&self, index: u8) -> Result<TempCelsius> {
        self.device.read_temp(index)
    }

    fn name(&self) -> &str {
        &self.name
    }
}
