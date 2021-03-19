use crate::config::SymbolSensor;
use crate::config::SymbolTable;
use crate::{ast, config::SymbolOutput};
use std::rc::Rc;

#[derive(new, Debug)]
pub struct ThermalProgram {
    pub symbol_table: SymbolTable,
    pub rules: Vec<When>,
}

#[derive(Debug)]
pub enum When {
    Unbounded(WhenUnbounded),
    Bounded(WhenBounded),
}

impl When {
    pub fn sensor(&self) -> &Rc<SymbolSensor> {
        match &self {
            When::Unbounded(when) => &when.sensor,
            When::Bounded(when) => &when.sensor,
        }
    }
}

#[derive(Debug, new)]
pub struct OutputSetFixed {
    pub target: Rc<SymbolOutput>,
    pub value: i32,
}

#[derive(Debug, new)]
pub struct OutputSetGeneric {
    pub target: Rc<SymbolOutput>,
    pub value: ast::OutputValue,
}

#[derive(Debug)]
pub enum Action<A: std::fmt::Debug> {
    Log,
    OutputSet(A),
}

#[derive(Debug)]
pub enum WhenUnboundedCond {
    Greater(i32),
    Less(i32),
}

#[derive(Debug, new)]
pub struct WhenUnbounded {
    pub sensor: Rc<SymbolSensor>,
    pub condition: WhenUnboundedCond,
    pub actions: Vec<Action<OutputSetFixed>>,
}

#[derive(Debug, new)]
pub struct WhenBounded {
    pub sensor: Rc<SymbolSensor>,
    pub min_value: i32,
    pub max_value: i32,
    pub actions: Vec<Action<OutputSetGeneric>>,
}
