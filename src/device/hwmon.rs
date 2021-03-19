use std::error::Error;

use log::debug;
use udev::Device as UdevDevice;

pub struct HwmonGenericDevice {
    device: UdevDevice,
}

impl std::fmt::Debug for HwmonGenericDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HwmonGenericDevice")
            .field("device", &self.device.devpath())
            .finish()
    }
}

impl HwmonGenericDevice {
    pub fn from_udev(device: UdevDevice) -> HwmonGenericDevice {
        return HwmonGenericDevice { device };
    }
}

impl HwmonGenericDevice {
    pub fn pwm_enable_attr(num: u8) -> String {
        return format!("pwm{}_enable", num);
    }

    pub fn pwm_attr(num: u8) -> String {
        return format!("pwm{}", num);
    }

    pub fn temp_input_attr(num: u8) -> String {
        return format!("temp{}_input", num);
    }

    pub fn set_pwm_enable(&self, num: u8, enable: &str) {
        let attr_enable = Self::pwm_enable_attr(num);
        let path = self.device.syspath().join(attr_enable);
        debug!("Writing '{}' into '{:?}'", enable, path);
        //	std::fs::write(&path, format!("{}\n", enable)).unwrap();
    }

    pub fn set_pwm_enable_and_value(&self, num: u8, enable: &str, value: &str) {
        self.set_pwm_enable(num, enable);
        let attr_value = Self::pwm_attr(num);
        let path = self.device.syspath().join(attr_value);
        debug!("Writing '{}' into '{:?}'", value, path);
        //	std::fs::write(&path, format!("{}\n", value)).unwrap();
    }

    fn read_attr(&self, name: &str) -> std::io::Result<String> {
        let path = self.device.syspath().join(name);
        let s = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
        if s.ends_with("\n") {
            Ok((&s[0..s.len() - 1]).into())
        } else {
            Ok(s.into())
        }
    }

    pub fn read_temp(&self, num: u8) -> Result<i32, Box<dyn Error>> {
        Ok(self
            .read_attr(&Self::temp_input_attr(num))?
            .parse::<i32>()?
            / 1000)
    }
}
