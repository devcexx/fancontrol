use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::Display,
    rc::Rc,
};

use super::ast;

pub enum SymbolTableError {
    Clash(String),
    NotFound(String),
    UnexpectedType {
        name: String,
        expected: String,
        found: String,
    },
}

impl Display for SymbolTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            &SymbolTableError::Clash(name) => write!(f, "Symbol defined twice: {}", name),
            &SymbolTableError::NotFound(name) => write!(f, "Undefined reference to {}", name),
            &SymbolTableError::UnexpectedType {
                name,
                expected,
                found,
            } => write!(
                f,
                "Unexpected symbol type for `{}`. Required '{}' but '{}' was found.",
                name, expected, found
            ),
        }
    }
}

pub type SymbolTableResult<T> = Result<T, SymbolTableError>;

pub trait SymbolType {
    type Value;
    fn name() -> &'static str;

    fn match_entry(s: &Symbol) -> Option<&Rc<Self::Value>>;
}

#[derive(Debug, Clone)]
pub enum Symbol {
    Device(Rc<SymbolDevice>),
    Sensor(Rc<SymbolSensor>),
    Output(Rc<SymbolOutput>),
}

impl Symbol {
    fn name(&self) -> &'static str {
        match self {
            &Symbol::Device(_) => "device",
            &Symbol::Sensor(_) => "sensor",
            &Symbol::Output(_) => "output",
        }
    }
}

#[derive(new, Debug)]
pub struct SymbolDevice {
    pub name: String,
    pub tag: String,
    pub driver: String,
}

#[derive(new, Debug)]
pub struct SymbolSensor {
    pub name: String,
    pub sensor_type: ast::SensorType,
    pub index: i32,
    pub device: Rc<SymbolDevice>,
}

#[derive(new, Debug)]
pub struct SymbolOutput {
    pub name: String,
    pub device: Rc<SymbolDevice>,
    pub output_type: ast::OutputType,
    pub index: u32,
}

impl SymbolType for SymbolDevice {
    type Value = SymbolDevice;

    fn name() -> &'static str {
        "device"
    }

    fn match_entry<'a>(s: &Symbol) -> Option<&Rc<Self::Value>> {
        match s {
            Symbol::Device(d) => Some(d),
            _ => None,
        }
    }
}

impl SymbolType for SymbolSensor {
    type Value = SymbolSensor;

    fn name() -> &'static str {
        "sensor"
    }

    fn match_entry<'a>(s: &Symbol) -> Option<&Rc<Self::Value>> {
        match s {
            Symbol::Sensor(s) => Some(s),
            _ => None,
        }
    }
}

impl SymbolType for SymbolOutput {
    type Value = SymbolOutput;

    fn name() -> &'static str {
        "output"
    }

    fn match_entry<'a>(s: &Symbol) -> Option<&Rc<Self::Value>> {
        match s {
            Symbol::Output(o) => Some(o),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct SymbolTable {
    map: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, symbol: Symbol) -> SymbolTableResult<&Symbol> {
        match self.map.entry(name.clone()) {
            Entry::Occupied(_) => Err(SymbolTableError::Clash(name)),
            Entry::Vacant(e) => Ok(e.insert(symbol)),
        }
    }

    pub fn require(&mut self, name: String) -> SymbolTableResult<&Symbol> {
        self.map
            .get(&name)
            .map_or_else(|| Err(SymbolTableError::NotFound(name)), |e| Ok(e))
    }

    pub fn require_type<A: SymbolType>(&self, name: &str) -> SymbolTableResult<&Rc<A::Value>> {
        self.map.get(name).map_or_else(
            || Err(SymbolTableError::NotFound(name.to_string())),
            |e| match A::match_entry(e) {
                Some(elem) => Ok(elem),
                None => Err(SymbolTableError::UnexpectedType {
                    name: name.to_string(),
                    expected: A::name().into(),
                    found: e.name().into(),
                }),
            },
        )
    }

    pub fn get_all_symbols_of_type<A: SymbolType>(&self) -> Vec<&Rc<A::Value>> {
        self.map
            .iter()
            .filter_map(|(_, symbol)| A::match_entry(symbol))
            .collect()
    }
}
