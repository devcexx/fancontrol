use std::{error::Error, fmt::Debug};

pub enum PwmMode {
    Auto,
    Full,
    Manual(u8),
}

pub trait Device: Debug {
    fn pwm_set(&self, pwm_number: u8, mode: PwmMode);
    fn temp_read(&self, sensor_number: u8) -> Result<i32, Box<dyn Error>>;
    fn name(&self) -> &str;
}
