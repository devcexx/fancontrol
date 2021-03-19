#[macro_use]
extern crate derive_new;

mod config;
mod device;

use std::cell::RefCell;

use config::{SymbolDevice, SymbolOutput, SymbolSensor, ast, checker::model as cmodel};
use device::{Device, HwmonGenericDevice, Nct6775Device, PwmMode, udev_find_with_tags};

#[macro_use]
extern crate lalrpop_util;

fn create_device(driver: &str, name: String, device: HwmonGenericDevice) -> Box<dyn Device> {
    match driver {
        "nct6775" => Box::new(Nct6775Device::new(name, device)),
        _ => panic!("Unknown driver {}", driver),
    }
}

#[derive(new, Debug)]
struct OnlineThermalRule<'prog> {
    sensor: OnlineSensor<'prog>,
    rule: &'prog cmodel::When,
}

impl<'prog> OnlineThermalRule<'prog> {
    pub fn is_triggered(&self) -> bool {
        let sensor_value = self.sensor.read_cached();
        match self.rule {
            cmodel::When::Unbounded(rule) => match rule.condition {
                cmodel::WhenUnboundedCond::Greater(lo) => sensor_value > lo,
                cmodel::WhenUnboundedCond::Less(hi) => sensor_value < hi,
            },
            cmodel::When::Bounded(rule) => {
                sensor_value >= rule.min_value && sensor_value <= rule.max_value
            }
        }
    }
}

#[derive(Debug)]
struct OnlineSensor<'prog> {
    device: &'prog Box<dyn Device>,
    symbol: &'prog SymbolSensor,
    _cached_value: RefCell<Option<i32>>,
}

impl<'prog> OnlineSensor<'prog> {
    fn new(device: &'prog Box<dyn Device>, symbol: &'prog SymbolSensor) -> Self {
        Self {
            device,
            symbol,
            _cached_value: RefCell::new(None),
        }
    }

    fn read(&'prog self) -> i32 {
        self.device.temp_read(self.symbol.index as u8).unwrap()
    }

    fn read_cached(&'prog self) -> i32 {
        // TODO fix this shit.
        if let Some(&value) = self._cached_value.borrow().as_ref() {
            return value;
        }

        let temp = self.read();
        self._cached_value.replace(Some(temp));
        temp
    }
}

#[derive(new, Debug)]
struct RunContext {
    pub thermal_program: cmodel::ThermalProgram,
    pub online_devices: Vec<Box<dyn Device>>,
}
 
impl RunContext {
    pub fn find_device<'prog>(&'prog self, name: &str) -> Option<&'prog Box<dyn Device>> {
        self.online_devices
            .iter()
            .find(|device| device.name() == name)
    }
    
    pub fn find_device_mut<'prog>(&'prog mut self, name: &str) -> Option<&'prog mut Box<dyn Device>> {
        self.online_devices
            .iter_mut()
            .find(|device| device.name() == name)
    }

    pub fn get_online_rules<'prog>(&'prog self) -> Vec<OnlineThermalRule<'prog>> {
        self.thermal_program
            .rules
            .iter()
            .filter_map(|rule| match self.find_device(&rule.sensor().device.name) {
                Some(device) => Some(OnlineThermalRule::new(
                    OnlineSensor::new(device, &rule.sensor()),
                    rule,
                )),
                None => None,
            })
            .collect()
    }

    pub fn register_device(&mut self, device: Box<dyn Device>) {
        if let Some(_) = self.find_device(device.name()) {
            panic!("Device already registered: {}", device.name())
        } else {
            self.online_devices.push(device);
        }
    }

    pub fn apply_rule(&self, rule: &OnlineThermalRule) {
	fn apply(context: &RunContext, rule: &OnlineThermalRule, target: &SymbolOutput, value: i32) {
	    if let Some(device) = context.find_device(&target.device.name) {
		device.pwm_set(target.index as u8, PwmMode::Manual(((value as f64) * 255.0 / 100.0) as u8))
	    } else {
		println!("Cannot find device {}", &target.device.name)
	    }
	}

	fn print_log(sensor: &OnlineSensor) {
	    println!("Value of {} is currently: {}",
		     &sensor.symbol.name,
		     sensor.read_cached())
	}
	
	match rule.rule {
	    cmodel::When::Unbounded(when) => {
		for action in &when.actions {
		    match action {
			cmodel::Action::Log => print_log(&rule.sensor),
		        cmodel::Action::OutputSet(action) =>
			    apply(self, rule, &action.target, action.value)
		    }
		}
	    }
	    cmodel::When::Bounded(when) => {
		for action in &when.actions {
		    match action {
			cmodel::Action::Log => print_log(&rule.sensor),
		        cmodel::Action::OutputSet(action) => {
			    match action.value {
				ast::OutputValue::Between(lo, hi) => {
				    let sensor_value = rule.sensor.read_cached() as f64;
				    let maxval = when.max_value as f64;
				    let minval = when.min_value as f64;
				    let progress = (sensor_value - minval) / (maxval - minval);

				    let output_per = lo + (progress * (hi as f64 - lo as f64)) as i32;
				    apply(self, rule, &action.target, output_per);
				}
				ast::OutputValue::Fixed(value) =>
				    apply(self, rule, &action.target, value)
			    }
			}
		    }
		}
	    }
	}
    }
}

fn main() {
    let input = r#"
DEFINE DEVICE `aio` UDEV TAG "fancontrol_aio" DRIVER "nct6775";

DEFINE SENSOR `aio_liquid_temp`
        DEVICE `aio`
        TYPE TERMISTOR
        INDEX 1;

DEFINE OUTPUT `aio_pump`
        DEVICE `aio`
        TYPE PWM
        INDEX 1;

WHEN `aio_liquid_temp` BETWEEN 10 AND 50 DO
   LOG;
   SET `aio_pump` BETWEEN 0% AND 50%;
END

WHEN `aio_liquid_temp` > 50 DO
   SET `aio_pump` TO 50%;
END
"#;
    let def_file = config::conffile::ProgramParser::new().parse(input).unwrap();

    let program = match config::check_program(def_file) {
        Ok(p) => p,
        Err(e) => panic!("Semantic error: {}", e),
    };

    let devices = program
        .symbol_table
        .get_all_symbols_of_type::<SymbolDevice>()
        .into_iter()
        .filter_map(|device| match udev_find_with_tags(vec![&device.tag]) {
            Some(udev_dev) => {
                let hwmon_dev = HwmonGenericDevice::from_udev(udev_dev);
                let device = create_device(&device.driver, device.name.clone(), hwmon_dev);
                Some(device)
            }

            None => {
                println!(
                    "Device not found: `{}` (no device with udev tag '{}' found.)",
                    device.name, device.tag
                );
                None
            }
        })
        .collect::<Vec<Box<dyn Device>>>();

    let context: RunContext = RunContext::new(program, devices);
    println!("{:#?}", context.online_devices);

    loop {
        let online_rules = context.get_online_rules();
	for rule in &online_rules {
	    if rule.is_triggered() {
		context.apply_rule(&rule);
	    }
	}
	
        std::thread::sleep_ms(1000);
    }
}
