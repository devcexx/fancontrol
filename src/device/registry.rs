use super::drivers;
use super::DeviceBuilder;
use lazy_static::lazy_static;
use std::collections::HashMap;

macro_rules! driver_registry {
    ($(($name:literal . $builder:expr)),*) => {
	lazy_static! {
	    static ref DEV_REG: HashMap<&'static str, Box<dyn DeviceBuilder + Sync>> = {
		let mut reg = HashMap::<&'static str, Box<dyn DeviceBuilder + Sync>>::new();
		$(
		    reg.insert($name, Box::new($builder));
		)*
		reg
	    };
	}
    };
}

driver_registry! {
    ("nct6775" . drivers::nct6775::Builder {})
}

pub fn driver_registry_find(name: &str) -> Option<&Box<dyn DeviceBuilder + Sync>> {
    DEV_REG.get(name)
}
