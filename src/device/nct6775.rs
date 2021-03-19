use std::error::Error;

use super::{Device, HwmonGenericDevice, PwmMode};
use udev::Device as UdevDevice;

const NCT6775_PWM_MODE_FULL: &str = "0";
const NCT6775_PWM_MODE_MANUAL: &str = "1";
const NCT6775_PWM_MODE_AUTO: &str = "5";

#[derive(new, Debug)]
pub struct Nct6775Device {
    name: String,
    device: HwmonGenericDevice,
}

impl Nct6775Device {
    fn from_udev(name: String, device: UdevDevice) -> Nct6775Device {
        Nct6775Device::new(name, HwmonGenericDevice::from_udev(device))
    }

    fn from_hwmon(name: String, device: HwmonGenericDevice) -> Nct6775Device {
        Nct6775Device::new(name, device)
    }
}

impl Nct6775Device {
    fn pwm_enable_attr(num: u8) -> String {
        return format!("pwm{}_enable", num);
    }

    fn pwm_attr(num: u8) -> String {
        return format!("pwm{}", num);
    }

    fn temp_input_attr(num: u8) -> String {
        return format!("pwm{}", num);
    }
}

impl Device for Nct6775Device {
    fn pwm_set(&self, pwm_number: u8, mode: PwmMode) {
        let attr_mode = Self::pwm_enable_attr(pwm_number);

        match mode {
            PwmMode::Auto => {
                self.device
                    .set_pwm_enable(pwm_number, NCT6775_PWM_MODE_AUTO);
            }
            PwmMode::Full => {
                self.device
                    .set_pwm_enable(pwm_number, NCT6775_PWM_MODE_FULL);
            }
            PwmMode::Manual(value) => {
                self.device.set_pwm_enable_and_value(
                    pwm_number,
                    NCT6775_PWM_MODE_MANUAL,
                    &value.to_string(),
                );
            }
        }
    }

    fn temp_read(&self, sensor_number: u8) -> Result<i32, Box<dyn Error>> {
        self.device.read_temp(sensor_number)
    }

    fn name(&self) -> &str {
        &self.name
    }
}
