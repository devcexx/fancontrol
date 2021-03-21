use log::debug;
use udev::Device as UdevDevice;

use crate::dev_debug;
use crate::{
    device::{Device, DeviceBuilder, PwmMode},
    types::TempCelsius,
};
use std::io::{Error, Result};
pub struct Builder;

impl DeviceBuilder for Builder {
    fn from_udev(&self, name: String, device: UdevDevice, dryrun: bool) -> Box<dyn Device> {
        Box::new(HwmonDevice::from_udev(name, device, dryrun))
    }
}

#[derive(new)]
pub struct HwmonDevice {
    name: String,
    device: UdevDevice,
    dryrun: bool,
}

impl std::fmt::Debug for HwmonDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HwmonGenericDevice")
            .field("device", &self.device.devpath())
            .finish()
    }
}

macro_rules! run_action {
    ($self:ident, $block:block) => {
        if !$self.dryrun {
            $block
        }
    };
}

macro_rules! log_write {
    ($self:ident, $value:expr, $attr:expr) => {
        dev_debug!($self, "write '{}' to attr `{}`.", $value, $attr);
    };
}

impl HwmonDevice {
    pub fn from_udev(name: String, device: UdevDevice, dryrun: bool) -> HwmonDevice {
        HwmonDevice::new(name, device, dryrun)
    }

    fn pwm_enable_attr(num: u8) -> String {
        return format!("pwm{}_enable", num);
    }

    fn pwm_attr(num: u8) -> String {
        return format!("pwm{}", num);
    }

    fn temp_input_attr(num: u8) -> String {
        return format!("temp{}_input", num);
    }

    pub fn write_raw_pwm(&self, num: u8, value: u8) -> Result<()> {
        let attr_value = Self::pwm_attr(num);
        let path = self.device.syspath().join(&attr_value);
        log_write!(self, value, attr_value);
        run_action!(self, {
            std::fs::write(&path, format!("{}\n", value))?;
        });

        Ok(())
    }

    pub fn write_pwm_enable(&self, num: u8, enable: &str) -> Result<()> {
        let attr_enable = Self::pwm_enable_attr(num);
        let path = self.device.syspath().join(&attr_enable);
        log_write!(self, enable, attr_enable);
        run_action!(self, {
            std::fs::write(&path, format!("{}\n", enable))?;
        });

        Ok(())
    }

    pub fn write_pwm_enable_and_value(&self, num: u8, enable: &str, value: u8) -> Result<()> {
        self.write_pwm_enable(num, enable)?;
        self.write_raw_pwm(num, value)
    }

    pub fn read_attr(&self, name: &str) -> Result<String> {
        let path = self.device.syspath().join(name);
        let s = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
        if s.ends_with("\n") {
            Ok((&s[0..s.len() - 1]).into())
        } else {
            Ok(s.into())
        }
    }
}

impl Device for HwmonDevice {
    fn write_pwm(&self, index: u8, mode: PwmMode) -> Result<()> {
        match mode {
            PwmMode::Auto | PwmMode::Full => {
                unimplemented!("Unsupported PWM mode for device: {:?}", mode)
            }
            PwmMode::ManualAbs(value) => self.write_raw_pwm(index, value),
            PwmMode::ManualPercent(value) => {
                dev_debug!(
                    self,
                    "Request set pwm {} of {} to {}.",
                    index,
                    &self.name,
                    value
                );
                self.write_raw_pwm(index, value.point_at_range(0u8, 255u8))
            }
        }
    }

    fn read_temp(&self, index: u8) -> Result<TempCelsius> {
        self.read_attr(&Self::temp_input_attr(index))?
            .parse::<i32>()
            .map(|temp| TempCelsius::from_mcelsius(temp))
            .map_err(|err| Error::new(std::io::ErrorKind::Other, err))
    }

    fn name(&self) -> &str {
        &self.name
    }
}
