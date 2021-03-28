#![feature(associated_type_bounds)]
#![feature(concat_idents)]

#[macro_use]
extern crate derive_new;

use clap::{App, Arg};
use env_logger::fmt::Color;
use log::{debug, info, warn};
use std::hash::Hash;
use types::{Percent, TempCelsius};

mod config;
mod device;
mod types;
mod util;

use std::{cell::RefCell, collections::HashMap, error::Error, io::Write};

use config::{
    ast, checker::model as cmodel, model::When, SymbolDevice, SymbolOutput, SymbolSensor,
};
use device::{driver_registry_find, udev_find_with_tags, Device, PwmMode};
use udev::Device as UdevDevice;

#[macro_use]
extern crate lalrpop_util;

fn create_device(driver: &str, name: String, device: UdevDevice, dryrun: bool) -> Box<dyn Device> {
    // FIXME Unwrap
    let builder = driver_registry_find(driver).unwrap();
    builder.from_udev(name, device, dryrun)
}

#[derive(new, Debug)]
struct OnlineThermalRule<'prog> {
    sensor: OnlineSensor<'prog>,
    when: &'prog cmodel::When,
}

impl<'prog> OnlineThermalRule<'prog> {
    pub fn is_triggered(&self) -> bool {
        let sensor_value = self.sensor.read_cached();
        match &self.when.behavior {
            cmodel::WhenBehavior::Unbounded(rule) => match rule.condition {
                cmodel::WhenUnboundedCond::Greater(lo) => {
                    sensor_value > TempCelsius::from_celsius(lo)
                }
                cmodel::WhenUnboundedCond::Less(hi) => sensor_value < TempCelsius::from_celsius(hi),
            },
            cmodel::WhenBehavior::Bounded(rule) => {
                sensor_value >= TempCelsius::from_celsius(rule.cond_min_value)
                    && sensor_value <= TempCelsius::from_celsius(rule.cond_max_value)
            }
        }
    }
}

#[derive(Debug)]
struct OnlineSensor<'prog> {
    device: &'prog Box<dyn Device>,
    symbol: &'prog SymbolSensor,
    _cached_value: RefCell<Option<TempCelsius>>,
}

impl<'prog> OnlineSensor<'prog> {
    fn new(device: &'prog Box<dyn Device>, symbol: &'prog SymbolSensor) -> Self {
        Self {
            device,
            symbol,
            _cached_value: RefCell::new(None),
        }
    }

    fn read(&'prog self) -> TempCelsius {
        self.device.read_temp(self.symbol.index as u8).unwrap()
    }

    fn read_cached(&'prog self) -> TempCelsius {
        // TODO fix this shit.
        if let Some(&value) = self._cached_value.borrow().as_ref() {
            return value;
        }

        let temp = self.read();
        self._cached_value.replace(Some(temp));
        temp
    }
}

/// Result of combining multiple ComputedRule's with a fold. Have a
/// different signature from ComputedRule, to keep the final results
/// for each output traceable from their origin rules.
#[derive(new)]
struct CombinedRule<'prog> {
    output_values: HashMap<ComputedRuleOutputKey<'prog>, CombinedRuleOutputValue<'prog>>,
}

impl<'prog> Default for CombinedRule<'prog> {
    fn default() -> Self {
        CombinedRule::new(HashMap::new())
    }
}

#[derive(new)]
struct CombinedRuleOutputValue<'prog> {
    rule: &'prog OnlineThermalRule<'prog>,
    value: i32,
}

/// Result of taking a single rule and and computing its actions based
/// on the configuration of the rule and the inputs of the sensors.
#[derive(Debug, new)]
struct ComputedRule<'prog> {
    rule: &'prog OnlineThermalRule<'prog>,
    output_values: HashMap<ComputedRuleOutputKey<'prog>, i32>,
    should_log: bool,
}

#[repr(transparent)]
#[derive(Debug)]
struct ComputedRuleOutputKey<'prog> {
    output: &'prog SymbolOutput,
}

impl<'prog> Hash for ComputedRuleOutputKey<'prog> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.output, state);
    }
}

impl<'prog> PartialEq for ComputedRuleOutputKey<'prog> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.output, other.output)
    }
}

impl<'prog> Eq for ComputedRuleOutputKey<'prog> {}

impl<'prog> From<&'prog SymbolOutput> for ComputedRuleOutputKey<'prog> {
    fn from(symbol: &'prog SymbolOutput) -> Self {
        Self { output: symbol }
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

    pub fn find_device_mut<'prog>(
        &'prog mut self,
        name: &str,
    ) -> Option<&'prog mut Box<dyn Device>> {
        self.online_devices
            .iter_mut()
            .find(|device| device.name() == name)
    }

    pub fn get_online_rules<'prog>(&'prog self) -> Vec<OnlineThermalRule<'prog>> {
        self.thermal_program
            .rules
            .iter()
            .filter_map(|rule| match self.find_device(&rule.sensor.device.name) {
                Some(device) => Some(OnlineThermalRule::new(
                    OnlineSensor::new(device, &rule.sensor),
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

    pub fn compute_rule_actions<'prog>(
        &'prog self,
        online_rule: &'prog OnlineThermalRule,
    ) -> ComputedRule<'prog> {
        let when = online_rule.when;

        let mut computed = ComputedRule::new(online_rule, HashMap::new(), false);

        for action in when.iter_actions() {
            match action {
                cmodel::AnyAction::Log => {
                    computed.should_log = true;
                }
                cmodel::AnyAction::BoundedOutputSet {
                    behavior,
                    target,
                    min,
                    max,
                } => {
                    let sensor_value = online_rule.sensor.read_cached().celsius() as f64;
                    let maxval = behavior.cond_max_value as f64;
                    let minval = behavior.cond_min_value as f64;
                    let progress = (sensor_value - minval) / (maxval - minval);

                    let output_per = min + (progress * (max as f64 - min as f64)) as i32;

                    // TODO Same key shouldn't exist already on the
                    // map. This must be checked on the semantic
                    // analysis of the config.
                    computed
                        .output_values
                        .insert(target.as_ref().into(), output_per);
                }
                cmodel::AnyAction::FixedOutputSet { target, value } => {
                    computed.output_values.insert(target.as_ref().into(), value);
                }
            }
        }

        computed
    }
}

fn print_log(rule: &When, sensor: &OnlineSensor) {
    info!(
        "[Rule '{}'] Value of {} is currently: {}",
        rule.rule_name(),
        &sensor.symbol.name,
        sensor.read_cached()
    );
}

enum ValueDiff<V1, V2> {
    Left(V1),
    Right(V2),
    Both(V1, V2),
}

fn diff_maps_into<K: Hash + Eq, V1, V2>(
    left: HashMap<K, V1>,
    mut right: HashMap<K, V2>,
) -> HashMap<K, ValueDiff<V1, V2>> {
    let mut result = HashMap::new();

    for (lkey, lvalue) in left.into_iter() {
        match right.remove_entry(&lkey) {
            Some((_, rvalue)) => {
                result.insert(lkey, ValueDiff::Both(lvalue, rvalue));
            }
            None => {
                result.insert(lkey, ValueDiff::Left(lvalue));
            }
        }
    }

    for (rkey, rvalue) in right.into_iter() {
        result.insert(rkey, ValueDiff::Right(rvalue));
    }

    result
}

fn priorization_fun<'prog>(
    pri: ast::OutputPriorization,
) -> fn(
    CombinedRuleOutputValue<'prog>,
    CombinedRuleOutputValue<'prog>,
) -> CombinedRuleOutputValue<'prog> {
    match pri {
        ast::OutputPriorization::Latest => |_, r| r,
        ast::OutputPriorization::Min => |l, r| {
            if l.value > r.value {
                r
            } else {
                l
            }
        },
        ast::OutputPriorization::Max => |l, r| {
            if l.value > r.value {
                l
            } else {
                r
            }
        },
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{}{} {:<5} {}{} {}",
                buf.style()
                    .set_color(Color::Black)
                    .set_intense(true)
                    .clone()
                    .value("["),
                buf.timestamp_seconds(),
                buf.default_styled_level(record.level()),
                record.target(),
                buf.style()
                    .set_color(Color::Black)
                    .set_intense(true)
                    .clone()
                    .value("]"),
                record.args()
            )
        })
        .init();

    let matches = App::new("Fancontrol")
        .version("1.0")
        .author("devcexx")
        .about("System monitor & fan control")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Specifies the path of the configuration file")
                .takes_value(true)
                .required(true),
        )
        .arg(
	    Arg::with_name("dry-run")
		.short("n")
		.long("dry-run")
		.help("Instructs the device drivers to run on dry-mode, without actually performing any changes on the underlying devices. This, combined with setting the environment variable \"RUST_LOG\" to \"debug\" is useful for debugging the program or configuration rules.")
	)
        .get_matches();

    let config_path = matches.value_of("config").unwrap();
    let dryrun = matches.is_present("dry-run");

    info!("Initializing fan control...");
    let conf_program = config::conffile::ProgramParser::new()
        .parse(&std::fs::read_to_string(config_path)?)
        .map_err::<Box<dyn Error>, _>(|err| err.to_string().into())?;

    let program = match config::check_program(conf_program) {
        Ok(p) => p,
        Err(e) => panic!("Semantic error: {}", e),
    };

    let devices = program
        .symbol_table
        .get_all_symbols_of_type::<SymbolDevice>()
        .into_iter()
        .filter_map(|device| match udev_find_with_tags(vec![&device.tag]) {
            Some(udev_dev) => {
                info!(
                    "Found device `{}` at {:?}",
                    device.name,
                    &udev_dev.devpath()
                );
                let device = create_device(&device.driver, device.name.clone(), udev_dev, dryrun);
                Some(device)
            }

            None => {
                warn!(
                    "Device not found: `{}`. Ignoring. (no device with udev tag '{}' found.)",
                    device.name, device.tag
                );
                None
            }
        })
        .collect::<Vec<Box<dyn Device>>>();

    let context: RunContext = RunContext::new(program, devices);
    loop {
        let online_rules: Vec<OnlineThermalRule> = context.get_online_rules();
        let applying_rules: Vec<ComputedRule> = online_rules
            .iter()
            .filter_map(|rule| {
                if rule.is_triggered() {
                    Some(context.compute_rule_actions(rule))
                } else {
                    None
                }
            })
            .collect();

        applying_rules
            .iter()
            .filter(|&rule| rule.should_log)
            .for_each(|computed_rule| {
                print_log(computed_rule.rule.when, &computed_rule.rule.sensor);
            });

        // Combine rules attending to the priorization rules specified
        // in the configuration.
        let combined_rules =
            applying_rules
                .into_iter()
                .fold(CombinedRule::default(), |acc, rule| {
                    let output_values = rule.output_values;
                    let source_rule = rule.rule;

                    let combined_rule = diff_maps_into(acc.output_values, output_values)
                        .into_iter()
                        .map(|(k, v)| {
                            let newval = match v {
                                ValueDiff::Left(value) => value,
                                ValueDiff::Right(value) => {
                                    CombinedRuleOutputValue::new(source_rule, value)
                                }
                                ValueDiff::Both(lvalue, rvalue) => {
                                    let rvalue = CombinedRuleOutputValue::new(source_rule, rvalue);
                                    priorization_fun(k.output.priorization.clone())(lvalue, rvalue)
                                }
                            };
                            (k, newval)
                        })
                        .collect();

                    CombinedRule::new(combined_rule)
                });

        for (key, value) in combined_rules.output_values.into_iter() {
            let output = key.output;

            if let Some(device) = context.find_device(&output.device.name) {
                debug!(
                    "Setting output `{}` to {}% by rule '{}'",
                    output.name,
                    value.value,
                    value.rule.when.rule_name()
                );

                device
                    .write_pwm(
                        output.index as u8,
                        PwmMode::ManualPercent(Percent::from(value.value as u32)), // FIXME Check for positive values on config semantic check.
                    )
                    .unwrap(); // FIXME Return a result
            } else {
                warn!(
                    "Couldn't completely apply rule: Cannot find device `{}`",
                    &output.device.name
                );
            }
        }

        std::thread::sleep_ms(1000);
    }
}
