#![feature(associated_type_bounds)]
#![feature(duration_zero)]

#[macro_use]
extern crate derive_new;

use clap::{App, Arg};
use env_logger::fmt::Color;
use guard::guard;
use log::{error, info, warn};
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::{
    convert::TryFrom,
    hash::Hash,
    time::{Duration, Instant},
};
use targeted_log::targeted_log;
use types::{Percent, TempCelsius};
use udevpoll::{PollMode, UdevPoller};

mod config;
mod device;
mod types;
mod udevpoll;
mod util;

use std::{cell::RefCell, collections::HashMap, error::Error, io::Write};

use config::{
    ast, checker::model as cmodel, model::When, SymbolDevice, SymbolOutput, SymbolSensor,
};
use device::{driver_registry_find, udev_extract_tags, udev_find_with_tags, Device, PwmMode};
use udev::{Device as UdevDevice, Event, MonitorBuilder};

#[macro_use]
extern crate lalrpop_util;

targeted_log!("config::rule {}", rule_);
targeted_log!("udev_events", devev_);

const EXIT_CODE_GENERAL_ERROR: i32 = 1;
const EXIT_CODE_HOT_UNPLUG: i32 = 2;

fn create_device(driver: &str, name: String, device: UdevDevice, dryrun: bool) -> Box<dyn Device> {
    // FIXME Unwrap
    let builder = driver_registry_find(driver).unwrap();
    builder.from_udev(name, device, dryrun)
}

fn create_online_device_for_symbol<'prog>(
    context: &RunContext<'prog>,
    symbol: &'prog Rc<SymbolDevice>,
    udev_device: UdevDevice,
) -> OnlineDevice<'prog> {
    OnlineDevice::new(
        create_device(
            &symbol.driver,
            symbol.name.clone(),
            udev_device,
            context.dryrun,
        ),
        symbol,
    )
}

enum DeviceState {
    Offline(usize),
    Online(usize),
    UnknownDevice,
    InvalidTags,
}

fn find_device_state(device: &UdevDevice, context: &mut RunContext) -> DeviceState {
    guard!(let Some(tags) = udev_extract_tags(&device) else {
    return DeviceState::InvalidTags;
    });

    let device_tags_text = tags.join(", ");
    devev_debug!("Device tags: [{}]", &device_tags_text);

    context
        .online_devices
        .iter()
        .position(|online_device| {
            tags.iter()
                .any(|&device_tag| device_tag == online_device.symbol.tag)
        })
        .map(|index| DeviceState::Online(index))
        .or_else(|| {
            context
                .offline_devices
                .iter()
                .position(|&offline_device| {
                    tags.iter()
                        .any(|&device_tag| device_tag == offline_device.tag)
                })
                .map(|index| DeviceState::Offline(index))
        })
        .unwrap_or(DeviceState::UnknownDevice)
}

fn process_device_event(context: &mut RunContext, event: Event) {
    guard!(let Some(action) = event.action() else {
    devev_debug!("Ignored event with no defined action");
    return;
    });

    guard!(let Some(action) = action.to_str() else {
    devev_debug!("Invalid action got: {:?}", event.action());
    return;
    });

    let adding;
    match action {
        "add" => {
            devev_debug!("Device added: {:?}.", event.devpath());
            adding = true;
        }

        "remove" => {
            devev_debug!("Device removed: {:?}.", event.devpath());
            adding = false;
        }

        other_action => {
            devev_debug!("Skipped action of type {}", other_action);
            return;
        }
    }

    let device_state = find_device_state(&event, context);

    const UNEXPECTED_DEVICE_STATE_MSG: &'static str =
        "This might be caused due to either the program didn't receive a \
	 previous udev event for that device, or that your udev rules \
	 are too general and are being triggered for multiple devices. \
	 Please review your udev rules for ensuring this is not the case, \
	 or issues may happen when attempting to monitor or cool your system.";

    match device_state {
        DeviceState::Offline(index) => {
            if adding {
                // Device is being added, and is now offline, everything's ok.
                let symbol = context.offline_devices.remove(index);
                context.online_devices.push(create_online_device_for_symbol(
                    &context,
                    symbol,
                    event.device(),
                ));
                devev_info!("Device {} plugged in at {:?}", symbol.name, event.devpath());
            } else {
                // Device is offline but there's an attempt of removing it again.
                devev_error!(
                    "Received an udev event for unregistering a non registered device. {}",
                    UNEXPECTED_DEVICE_STATE_MSG
                );
                devev_error!("Offending device: {:?}", context.offline_devices[index]);
            }
        }
        DeviceState::Online(index) => {
            if adding {
                // Device is online but there's an attempt of adding it again.
                let device = &context.online_devices[index];
                devev_error!(
                    "Received an udev event for re-registering an already registered device. {}",
                    UNEXPECTED_DEVICE_STATE_MSG
                );
                devev_error!(
                    "Offending device: {:?}; {:?}",
                    device.symbol,
                    device.inner.as_ref()
                );
            } else {
                let device = context.online_devices.remove(index);
                context.offline_devices.push(device.symbol);
                devev_info!("Device {} unplugged", device.name());
            }
        }
        DeviceState::UnknownDevice => {} // Ignore
        DeviceState::InvalidTags => {
            devev_debug!("Device has invalid tags. Ignoring.");
        }
    }
}

fn poll_device_events(context: &mut RunContext, poller: &mut UdevPoller, mode: PollMode) {
    for event in poller.poll_events(mode).unwrap() {
        process_device_event(context, event);
    }
}

#[derive(Debug, new)]
struct OnlineDevice<'prog> {
    inner: Box<dyn Device>,
    symbol: &'prog Rc<SymbolDevice>,
}

impl Deref for OnlineDevice<'_> {
    type Target = dyn Device;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl DerefMut for OnlineDevice<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
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
    device: &'prog OnlineDevice<'prog>,
    symbol: &'prog SymbolSensor,
    _cached_value: RefCell<Option<TempCelsius>>,
}

impl<'prog> OnlineSensor<'prog> {
    fn new(device: &'prog OnlineDevice<'prog>, symbol: &'prog SymbolSensor) -> Self {
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
    value: Percent,
}

/// Result of taking a single rule and and computing its actions based
/// on the configuration of the rule and the inputs of the sensors.
#[derive(Debug, new)]
struct ComputedRule<'prog> {
    rule: &'prog OnlineThermalRule<'prog>,
    output_values: HashMap<ComputedRuleOutputKey<'prog>, Percent>,
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
struct RunContext<'prog> {
    pub thermal_program: &'prog cmodel::ThermalProgram,
    pub online_devices: Vec<OnlineDevice<'prog>>,
    pub offline_devices: Vec<&'prog Rc<SymbolDevice>>,
    pub dryrun: bool,
}

// TODO Create a better interface for adding or dropping new online devices, and
// recalculating online rules.
impl<'prog, 'this> RunContext<'prog>
where
    'prog: 'this,
{
    pub fn find_device(&'this self, name: &str) -> Option<&'this OnlineDevice<'prog>> {
        self.online_devices
            .iter()
            .find(|device| device.name() == name)
    }

    pub fn find_device_mut(&'this mut self, name: &str) -> Option<&'this mut OnlineDevice<'prog>> {
        self.online_devices
            .iter_mut()
            .find(|device| device.name() == name)
    }

    pub fn get_online_rules(&'this self) -> Vec<OnlineThermalRule<'this>> {
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

    pub fn register_device(&mut self, device: OnlineDevice<'prog>) {
        if let Some(_) = self.find_device(device.name()) {
            panic!("Device already registered: {}", device.name())
        } else {
            self.online_devices.push(device);
        }
    }

    fn filter_non_hotpluggable_offline_devices(&self) -> impl Iterator<Item = &&Rc<SymbolDevice>> {
        self.offline_devices.iter().filter(|dev| !dev.allow_hotplug)
    }

    pub fn any_non_hotpluggable_device_offline(&self) -> bool {
        self.filter_non_hotpluggable_offline_devices()
            .next()
            .is_some()
    }

    pub fn compute_rule_actions(
        &'this self,
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
                    // TODO Can this calculation be improved?
                    let min = min.value() as f64;
                    let max = max.value() as f64;

                    let sensor_value = online_rule.sensor.read_cached().celsius() as f64;
                    let maxval = behavior.cond_max_value as f64;
                    let minval = behavior.cond_min_value as f64;
                    let progress = (sensor_value - minval) / (maxval - minval);

                    let output_per =
                        Percent::try_from((min + (progress * (max - min))) as i32).unwrap();

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
    rule_info!(@ rule.rule_name();
        "Value of {} is {}.",
        sensor.symbol.name,
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
        .arg(
	    Arg::with_name("interval")
		.short("t")
		.long("interval")
		.help("Defines the interval between each time the sensors and the rule are evaluated, in milliseconds.")
		.default_value("1000")
	)
        .arg(
	    Arg::with_name("discover-timeout")
		.short("k")
		.long("discover-timeout")
		.help("Defines how much time should the program wait at most when starting to allow all devices to become full available, in seconds. A value of 0 will indicate that no wait will be performed.")
		.default_value("30")
	)
        .get_matches();

    let config_path = matches.value_of("config").unwrap();
    let dryrun = matches.is_present("dry-run");
    let interval = Duration::from_millis(clap::value_t_or_exit!(matches.value_of("interval"), u64));
    let discover_timeout = Duration::from_secs(clap::value_t_or_exit!(
        matches.value_of("discover-timeout"),
        u64
    ));

    info!("Initializing fan control...");
    let conf_program = config::conffile::ProgramParser::new()
        .parse(&std::fs::read_to_string(config_path)?)
        .map_err::<Box<dyn Error>, _>(|err| err.to_string().into())?;

    let program = match config::check_program(conf_program) {
        Ok(p) => p,
        Err(e) => panic!("Configuration error: {}", e),
    };

    let mut udev_poller = UdevPoller::poll_on(
        MonitorBuilder::new()?
            .match_subsystem("hidraw")?
            .match_tag("my_mouse")?
            .listen()?,
    );

    info!("Discovering devices...");

    let mut context: RunContext = RunContext::new(&program, Vec::new(), Vec::new(), dryrun);

    // TODO Multiple devices must not be identified by the same udev tag.
    for device_symbol in program
        .symbol_table
        .get_all_symbols_of_type::<SymbolDevice>()
        .into_iter()
    {
        match udev_find_with_tags(vec![&device_symbol.tag]) {
            Some(udev_dev) => {
                info!(
                    "Found device `{}` at {:?}",
                    device_symbol.name,
                    &udev_dev.devpath()
                );

                context.online_devices.push(create_online_device_for_symbol(
                    &context,
                    device_symbol,
                    udev_dev,
                ));
            }

            None => {
                context.offline_devices.push(device_symbol);
            }
        }
    }

    // TODO Avoid constant calculation of existing offline
    // non-hotpluggable devices by... keeping track of them somehow?
    if context.any_non_hotpluggable_device_offline() && !discover_timeout.is_zero() {
        info!(
            "Waiting until offline devices becomes visible... : [{}]",
            context
                .offline_devices
                .iter()
                .map(|d| d.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        let start_time = Instant::now();
        let mut elapsed;
        while {
            elapsed = start_time.elapsed();
            elapsed
        } < discover_timeout
            && context.any_non_hotpluggable_device_offline()
        {
            let remaining = discover_timeout - elapsed;
            poll_device_events(
                &mut context,
                &mut udev_poller,
                PollMode::WaitTimeout(remaining),
            )
        }
    }

    let remaining_mandatory_devs = context
        .filter_non_hotpluggable_offline_devices()
        .map(|dev| dev.name.as_ref())
        .collect::<Vec<&str>>();
    if remaining_mandatory_devs.len() > 0 {
        error!(
            "Unable to find some required devices attached in the system: [{}]. Quitting NOW!",
            remaining_mandatory_devs.join(", ")
        );

        std::process::exit(EXIT_CODE_GENERAL_ERROR);
    }

    loop {
        let start_time = Instant::now();
        poll_device_events(&mut context, &mut udev_poller, PollMode::NoWait);
        let offline_non_hotpluggable_devices = context
            .filter_non_hotpluggable_offline_devices()
            .map(|dev| dev.name.as_ref())
            .collect::<Vec<_>>();
        if !offline_non_hotpluggable_devices.is_empty() {
            error!(
                "One or more non hot-plugabble device has been removed: {}",
                offline_non_hotpluggable_devices.join(", ")
            );
            std::process::exit(EXIT_CODE_HOT_UNPLUG);
        }

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
                rule_debug!(@ value.rule.when.rule_name();
                    "Set `{}` to {}.",
                    output.name,
                    value.value
                );

                device
                    .write_pwm(output.index, PwmMode::ManualPercent(value.value))
                    .unwrap(); // FIXME Return a result
            } else {
                warn!(
                    "Couldn't completely apply rule: Cannot find device `{}`",
                    &output.device.name
                );
            }
        }

        std::thread::sleep((interval - start_time.elapsed()).max(Duration::default()));
    }
}
