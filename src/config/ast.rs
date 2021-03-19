use std::fmt::Debug;

#[derive(new, Debug, Clone)]
pub struct Program {
    pub statements: Vec<Rule>,
}

#[derive(new, Debug, Clone)]
pub struct RuleDefineDevice {
    pub dev_name: String,
    pub udev_tag: String,
    pub driver_name: String,
}

#[derive(new, Debug, Clone)]
pub struct RuleDefineSensor {
    pub sensor_name: String,
    pub device: String,
    pub sensor_type: SensorType,
    pub index: i32,
}

#[derive(new, Debug, Clone)]
pub struct RuleDefineOutput {
    pub output_name: String,
    pub device: String,
    pub output_type: OutputType,
    pub index: i32,
}

#[derive(Clone)]
pub enum RuleDefine {
    Device(RuleDefineDevice),
    Sensor(RuleDefineSensor),
    Output(RuleDefineOutput),
}

impl Debug for RuleDefine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleDefine::Device(device) => device.fmt(f),
            RuleDefine::Sensor(sensor) => sensor.fmt(f),
            RuleDefine::Output(output) => output.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OutputValue {
    Between(i32, i32),
    Fixed(i32),
}

#[derive(new, Debug, Clone)]
pub struct WhenActionOutputSet {
    pub target_output: String,
    pub value: OutputValue,
}

#[derive(Clone)]
pub enum WhenAction {
    Log,
    OutputSet(WhenActionOutputSet),
}

impl Debug for WhenAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhenAction::OutputSet(set) => set.fmt(f),
            WhenAction::Log => write!(f, "Log")
	}
    }
}

#[derive(new, Debug, Clone)]
pub enum WhenCondition {
    Between(i32, i32),
    GreaterThan(i32),
    LessThan(i32),
}

#[derive(new, Debug, Clone)]
pub struct RuleWhen {
    pub sensor: String,
    pub condition: WhenCondition,
    pub actions: Vec<WhenAction>,
}

#[derive(Clone)]
pub enum Rule {
    Define(RuleDefine),
    When(RuleWhen),
}

impl Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rule::Define(define) => define.fmt(f),
            Rule::When(when) => when.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SensorType {
    Termistor,
    Fan,
}

#[derive(Debug, Clone)]
pub enum OutputType {
    Pwm,
}
