use log::debug;
use udev::Device as UdevDevice;

use crate::{
    device::{Device, DeviceBuilder, PwmMode},
    types::TempCelsius,
};
use std::io::{Error, Result};
pub struct Builder;

impl DeviceBuilder for Builder {
    fn from_udev(&self, name: String, device: UdevDevice) -> Box<dyn Device> {
        Box::new(HwmonDevice::from_udev(name, device))
    }
}

#[derive(new)]
pub struct HwmonDevice {
    name: String,
    device: UdevDevice,
}

impl std::fmt::Debug for HwmonDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HwmonGenericDevice")
            .field("device", &self.device.devpath())
            .finish()
    }
}

impl HwmonDevice {
    pub fn from_udev(name: String, device: UdevDevice) -> HwmonDevice {
        HwmonDevice::new(name, device)
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
        let path = self.device.syspath().join(attr_value);
        debug!("Writing '{}' into '{:?}'", value, path);
        //	std::fs::write(&path, format!("{}\n", value)).unwrap();
        Ok(())
    }

    pub fn write_pwm_enable(&self, num: u8, enable: &str) -> Result<()> {
        let attr_enable = Self::pwm_enable_attr(num);
        let path = self.device.syspath().join(attr_enable);
        debug!("Writing '{}' into '{:?}'", enable, path);
        //	std::fs::write(&path, format!("{}\n", enable)).unwrap();
        Ok(())
    }

    pub fn write_pwm_enable_and_value(&self, num: u8, enable: &str, value: u8) -> Result<()> {
        self.write_pwm_enable(num, enable)?;
        self.write_raw_pwm(num, value)?;

        Ok(())
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
                self.write_raw_pwm(index, value.map_to_range(0u8, 255u8))
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
