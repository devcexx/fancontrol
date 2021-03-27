use crate::config::SymbolSensor;
use crate::config::SymbolTable;
use crate::{ast, config::SymbolOutput};
use std::{borrow::Cow, rc::Rc};

#[derive(new, Debug)]
pub struct ThermalProgram {
    pub symbol_table: SymbolTable,
    pub rules: Vec<When>,
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
pub enum AnyAction<'a> {
    Log,
    BoundedOutputSet {
        behavior: &'a WhenBoundedBehavior,
        target: &'a Rc<SymbolOutput>,
        min: i32,
        max: i32,
    },
    FixedOutputSet {
        target: &'a Rc<SymbolOutput>,
        value: i32,
    },
}

#[derive(Debug)]
pub enum WhenUnboundedCond {
    Greater(i32),
    Less(i32),
}

#[derive(Debug, new)]
pub struct When {
    pub rule_index: u32,
    pub tag: Option<String>,
    pub sensor: Rc<SymbolSensor>,
    pub behavior: WhenBehavior,
}

impl When {
    pub fn iter_actions<'a>(&'a self) -> WhenRuleIter<'a> {
        match &self.behavior {
            WhenBehavior::Bounded(bounded) => WhenRuleIter::from_bounded(bounded),
            WhenBehavior::Unbounded(unbounded) => WhenRuleIter::from_unbouded(unbounded),
        }
    }

    pub fn rule_name<'a>(&'a self) -> Cow<'a, str> {
        if let Some(tag) = &self.tag {
            Cow::Borrowed(tag)
        } else {
            Cow::Owned(format!("#{}", self.rule_index + 1))
        }
    }
}

pub struct WhenRuleIter<'a> {
    iter: Box<dyn (FnMut() -> Option<AnyAction<'a>>) + 'a>,
}

impl<'a> WhenRuleIter<'a> {
    fn from_bounded<'b>(bounded: &'b WhenBoundedBehavior) -> WhenRuleIter<'b> {
        let mut iterator = bounded.actions.iter();

        let fun = move || match iterator.next() {
            Some(Action::Log) => Some(AnyAction::Log),
            Some(Action::OutputSet(OutputSetGeneric {
                target,
                value: ast::OutputValue::Between(min, max),
            })) => Some(AnyAction::BoundedOutputSet {
                behavior: bounded,
                target,
                min: *min,
                max: *max,
            }),
            Some(Action::OutputSet(OutputSetGeneric {
                target,
                value: ast::OutputValue::Fixed(value),
            })) => Some(AnyAction::FixedOutputSet {
                target,
                value: *value,
            }),
            None => None,
        };

        WhenRuleIter {
            iter: Box::new(fun),
        }
    }

    fn from_unbouded<'b>(unbouded: &'b WhenUnboundedBehavior) -> WhenRuleIter<'b> {
        let mut iterator = unbouded.actions.iter();

        let fun = move || match iterator.next() {
            Some(Action::Log) => Some(AnyAction::Log),
            Some(Action::OutputSet(OutputSetFixed { target, value })) => {
                Some(AnyAction::FixedOutputSet {
                    target,
                    value: *value,
                })
            }
            None => None,
        };

        WhenRuleIter {
            iter: Box::new(fun),
        }
    }
}

impl<'a> Iterator for WhenRuleIter<'a> {
    type Item = AnyAction<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        (self.iter)()
    }
}

#[derive(Debug)]
pub enum WhenBehavior {
    Bounded(WhenBoundedBehavior),
    Unbounded(WhenUnboundedBehavior),
}

#[derive(Debug, new)]
pub struct WhenUnboundedBehavior {
    pub condition: WhenUnboundedCond,
    pub actions: Vec<Action<OutputSetFixed>>,
}

#[derive(Debug, new)]
pub struct WhenBoundedBehavior {
    pub cond_min_value: i32,
    pub cond_max_value: i32,
    pub actions: Vec<Action<OutputSetGeneric>>,
}
