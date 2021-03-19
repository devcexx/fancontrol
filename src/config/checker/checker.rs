use super::{model, ProgramCheckError, ProgramCheckResult, SemanticError};
use crate::config::{
    ast, Symbol, SymbolDevice, SymbolOutput, SymbolSensor, SymbolTable, SymbolTableResult,
};

fn process_define_rule(
    sym_table: &mut SymbolTable,
    rule: ast::RuleDefine,
) -> SymbolTableResult<()> {
    (match rule {
        ast::RuleDefine::Device(device) => sym_table.insert(
            device.dev_name.clone(),
            Symbol::Device(
                SymbolDevice::new(device.dev_name, device.udev_tag, device.driver_name).into(),
            ),
        ),
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

            sym_table.insert(sensor.sensor_name, symbol)
        }
        ast::RuleDefine::Output(output) => {
            let device = sym_table.require_type::<SymbolDevice>(&output.device)?;

            // TODO Add index validation.
            let symbol: Symbol = Symbol::Output(
                SymbolOutput::new(
                    output.output_name.clone(),
                    device.clone(),
                    output.output_type,
                    output.index as u32,
                )
                .into(),
            );
            sym_table.insert(output.output_name, symbol)
        }
    })
    .map(|_| ())
}

fn process_when_rule(
    sym_table: &mut SymbolTable,
    rule: ast::RuleWhen,
) -> ProgramCheckResult<model::When> {
    fn into_fixed_actions(
        actions: Vec<model::Action<model::OutputSetGeneric>>,
    ) -> ProgramCheckResult<Vec<model::Action<model::OutputSetFixed>>> {
        let mut result = Vec::new();

        for action in actions {
            match action {
                model::Action::OutputSet(action) => match action.value {
                    ast::OutputValue::Between(_, _) => {
                        return Err(ProgramCheckError::SemanticError(
                            SemanticError::BetweenActionInUnboundedRule,
                        ))
                    }
                    ast::OutputValue::Fixed(value) => result.push(model::Action::OutputSet(
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
                actions.push(model::Action::OutputSet(model::OutputSetGeneric::new(
                    output.clone(),
                    action.value,
                )))
            }
        }
    }

    Ok(match rule.condition {
        ast::WhenCondition::Between(low, high) => {
            model::When::Bounded(model::WhenBounded::new(sensor.clone(), low, high, actions))
        }
        ast::WhenCondition::GreaterThan(low) => model::When::Unbounded(model::WhenUnbounded::new(
            sensor.clone(),
            model::WhenUnboundedCond::Greater(low),
            into_fixed_actions(actions)?,
        )),
        ast::WhenCondition::LessThan(high) => model::When::Unbounded(model::WhenUnbounded::new(
            sensor.clone(),
            model::WhenUnboundedCond::Less(high),
            into_fixed_actions(actions)?,
        )),
    })
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
                when_rules.push(process_when_rule(&mut symbol_table, when)?);
            }
        }
    }

    Ok(model::ThermalProgram::new(symbol_table, when_rules))
}
