use std::convert::TryInto;

use model::OutputValue;

use super::{model, NumBoundary, ProgramCheckError, ProgramCheckResult, SemanticError};
use crate::config::{ast, Symbol, SymbolDevice, SymbolOutput, SymbolSensor, SymbolTable};
use crate::types::Percent;
use std::convert::TryFrom;

fn process_define_rule(
    sym_table: &mut SymbolTable,
    rule: ast::RuleDefine,
) -> ProgramCheckResult<()> {
    (match rule {
        ast::RuleDefine::Device(device) => sym_table
            .insert(
                device.dev_name.clone(),
                Symbol::Device(
                    SymbolDevice::new(
                        device.dev_name,
                        device.udev_tag,
                        device.driver_name,
                        device.allow_hotplug,
                    )
                    .into(),
                ),
            )
            .map_err(|err| err.into()),
        ast::RuleDefine::Sensor(sensor) => {
            let device = sym_table.require_type::<SymbolDevice>(&sensor.device)?;

            let symbol = Symbol::Sensor(
                SymbolSensor::new(
                    sensor.sensor_name.clone(),
                    sensor.sensor_type,
                    sensor.index,
                    device.clone(),
                )
                .into(),
            );

            sym_table
                .insert(sensor.sensor_name, symbol)
                .map_err(|err| err.into())
        }
        ast::RuleDefine::Output(output) => {
            let device = sym_table.require_type::<SymbolDevice>(&output.device)?;

            let output_index: u8 = output.index.try_into().map_err(|_| {
                ProgramCheckError::SemanticError(SemanticError::NumberOutOfBounds(
                    NumBoundary::BetweenBothExclusive(0, 255),
                    output.index,
                ))
            })?;

            let symbol: Symbol = Symbol::Output(
                SymbolOutput::new(
                    output.output_name.clone(),
                    device.clone(),
                    output.output_type,
                    output_index,
                    output.priorization,
                )
                .into(),
            );
            sym_table
                .insert(output.output_name, symbol)
                .map_err(|err| err.into())
        }
    })
    .map(|_| ())
}

fn cast_percent(value: i32) -> ProgramCheckResult<Percent> {
    Percent::try_from(value)
        .map_err(|err| ProgramCheckError::SemanticError(SemanticError::InvalidPercent(value)))
}

fn process_when_rule(
    sym_table: &mut SymbolTable,
    rule_index: u32,
    rule: ast::RuleWhen,
) -> ProgramCheckResult<model::When> {
    fn into_fixed_actions(
        actions: Vec<model::Action<model::OutputSetGeneric>>,
    ) -> ProgramCheckResult<Vec<model::Action<model::OutputSetFixed>>> {
        let mut result = Vec::new();

        for action in actions {
            match action {
                model::Action::OutputSet(action) => match action.value {
                    OutputValue::Between(_, _) => {
                        return Err(ProgramCheckError::SemanticError(
                            SemanticError::BetweenActionInUnboundedRule,
                        ))
                    }
                    OutputValue::Fixed(value) => result.push(model::Action::OutputSet(
                        model::OutputSetFixed::new(action.target, value),
                    )),
                },
                model::Action::Log => result.push(model::Action::Log),
            }
        }

        Ok(result)
    }

    let sensor = sym_table.require_type::<SymbolSensor>(&rule.sensor)?;

    let mut actions =
        Vec::<model::Action<model::OutputSetGeneric>>::with_capacity(rule.actions.len());
    for action in rule.actions {
        match action {
            ast::WhenAction::Log => actions.push(model::Action::Log),
            ast::WhenAction::OutputSet(action) => {
                let output = sym_table.require_type::<SymbolOutput>(&action.target_output)?;
                let action_value = (match action.value {
                    ast::OutputValue::Between(lo, hi) => ProgramCheckResult::Ok(
                        OutputValue::Between(cast_percent(lo)?, cast_percent(hi)?),
                    ),
                    ast::OutputValue::Fixed(value) => Ok(OutputValue::Fixed(cast_percent(value)?)),
                })?;

                actions.push(model::Action::OutputSet(model::OutputSetGeneric::new(
                    output.clone(),
                    action_value,
                )))
            }
        }
    }

    let behavior = match rule.condition {
        ast::WhenCondition::Between(low, high) => {
            model::WhenBehavior::Bounded(model::WhenBoundedBehavior::new(low, high, actions))
        }
        ast::WhenCondition::GreaterThan(low) => {
            model::WhenBehavior::Unbounded(model::WhenUnboundedBehavior::new(
                model::WhenUnboundedCond::Greater(low),
                into_fixed_actions(actions)?,
            ))
        }
        ast::WhenCondition::LessThan(high) => {
            model::WhenBehavior::Unbounded(model::WhenUnboundedBehavior::new(
                model::WhenUnboundedCond::Less(high),
                into_fixed_actions(actions)?,
            ))
        }
    };

    let rule = model::When::new(rule_index, rule.tag, sensor.clone(), behavior);
    Ok(rule)
}

pub fn check_program(program: ast::Program) -> ProgramCheckResult<model::ThermalProgram> {
    let mut symbol_table = SymbolTable::new();
    let mut when_rules = Vec::<model::When>::new();

    for rule in program.statements {
        match rule {
            ast::Rule::Define(def) => {
                process_define_rule(&mut symbol_table, def)?;
            }

            ast::Rule::When(when) => {
                when_rules.push(process_when_rule(
                    &mut symbol_table,
                    when_rules.len() as u32,
                    when,
                )?);
            }
        }
    }

    Ok(model::ThermalProgram::new(symbol_table, when_rules))
}
