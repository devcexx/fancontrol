use super::{Measure, MeasureUnit};
use std::fmt::{Debug, Display};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Celsius;
impl MeasureUnit for Celsius {
    type Holder = i32;
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Fahrenheit;
impl MeasureUnit for Fahrenheit {
    type Holder = f32;
}

// Celsius units represented as millicelsius, to prevent precision loss.
pub type TempCelsius = Measure<Celsius>;

#[allow(unused)] // FIXME Remove when Fahrenheit support is added.
pub type TempFahrenheit = Measure<Fahrenheit>;

impl Measure<Celsius> {
    pub fn from_mcelsius(value: i32) -> TempCelsius {
        Measure::new(value)
    }

    pub fn from_celsius(value: i32) -> TempCelsius {
        Measure::new(value * 1000)
    }

    pub fn celsius(self) -> i32 {
        self.raw_value() / 1000
    }

    pub fn mcelsius(self) -> i32 {
        self.raw_value()
    }
}

#[allow(unused)] // FIXME Remove when Fahrenheit support is added.
impl Measure<Fahrenheit> {
    pub fn value(self) -> f32 {
        self.raw_value()
    }
}

impl Display for Measure<Celsius> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let int = self.raw_value() / 1000;
        let dec = self.raw_value() % 1000;

        if dec == 0 {
            write!(f, "{} °C", int)
        } else {
            write!(f, "{}.{:03} °C", int, dec)
        }
    }
}

impl Debug for Measure<Celsius> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <TempCelsius as Display>::fmt(self, f)
    }
}
