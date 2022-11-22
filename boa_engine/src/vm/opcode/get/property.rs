use crate::{
    property::PropertyKey,
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult, JsString,
};

#[cfg(feature = "instrumentation")]
use crate::{instrumentation::EvaluationMode::BaseEvaluation, JsValue};

/// `GetPropertyByName` implements the Opcode Operation for `Opcode::GetPropertyByName`
///
/// Operation:
///  - Get a property by name from an object an push it on the stack.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GetPropertyByName;

impl Operation for GetPropertyByName {
    const NAME: &'static str = "GetPropertyName";
    const INSTRUCTION: &'static str = "INST - GetPropertyName";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        if let BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.get_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let index = context.vm.read::<u32>();

                        let value = context.vm.pop();

                        let name = context.vm.frame().code.names[index as usize];
                        let name: JsValue = context
                            .interner()
                            .resolve_expect(name.sym())
                            .into_common::<JsString>(false)
                            .into();

                        let result = context.call(trap, &advice, &[value, name]);

                        match result {
                            Ok(result) => {
                                context.instrumentation_conf.set_mode_base();

                                context.vm.push(result);
                                return Ok(ShouldExit::False);
                            }
                            Err(v) => {
                                panic!("Instrumentation: Uncaught {}", v.to_string());
                            }
                        }
                    }
                }
            }
        }

        let index = context.vm.read::<u32>();

        let value = context.vm.pop();
        let object = if let Some(object) = value.as_object() {
            object.clone()
        } else {
            value.to_object(context)?
        };

        let name = context.vm.frame().code.names[index as usize];
        let name: PropertyKey = context
            .interner()
            .resolve_expect(name.sym())
            .into_common::<JsString>(false)
            .into();
        let result = object.get(name, context)?;

        context.vm.push(result);
        Ok(ShouldExit::False)
    }
}

/// `GetPropertyByValue` implements the Opcode Operation for `Opcode::GetPropertyByValue`
///
/// Operation:
///  - Get a property by value from an object an push it on the stack.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GetPropertyByValue;

impl Operation for GetPropertyByValue {
    const NAME: &'static str = "GetPropertyByValue";
    const INSTRUCTION: &'static str = "INST - GetPropertyByValue";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        if let BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.get_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let key = context.vm.pop();
                        let object = context.vm.pop();

                        let result = context.call(trap, &advice, &[object, key]);

                        match result {
                            Ok(result) => {
                                context.instrumentation_conf.set_mode_base();

                                context.vm.push(result);
                                return Ok(ShouldExit::False);
                            }
                            Err(v) => {
                                panic!("Instrumentation: Uncaught {}", v.to_string());
                            }
                        }
                    }
                }
            }
        }

        let key = context.vm.pop();
        let object = context.vm.pop();
        let object = if let Some(object) = object.as_object() {
            object.clone()
        } else {
            object.to_object(context)?
        };

        let key = key.to_property_key(context)?;
        let value = object.get(key, context)?;

        context.vm.push(value);
        Ok(ShouldExit::False)
    }
}

/// `GetPropertyByValuePush` implements the Opcode Operation for `Opcode::GetPropertyByValuePush`
///
/// Operation:
///  - Get a property by value from an object an push the key and value on the stack.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GetPropertyByValuePush;

impl Operation for GetPropertyByValuePush {
    const NAME: &'static str = "GetPropertyByValuePush";
    const INSTRUCTION: &'static str = "INST - GetPropertyByValuePush";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        if let BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.get_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let key = context.vm.pop();
                        let object = context.vm.pop();

                        let result = context.call(trap, &advice, &[object, key.clone()]);

                        match result {
                            Ok(result) => {
                                context.instrumentation_conf.set_mode_base();

                                context.vm.push(key);
                                context.vm.push(result);

                                return Ok(ShouldExit::False);
                            }
                            Err(v) => {
                                panic!("Instrumentation: Uncaught {}", v.to_string());
                            }
                        }
                    }
                }
            }
        }

        let key = context.vm.pop();
        let object = context.vm.pop();
        let object = if let Some(object) = object.as_object() {
            object.clone()
        } else {
            object.to_object(context)?
        };

        let property_key = key.to_property_key(context)?;
        let value = object.get(property_key, context)?;

        context.vm.push(key);
        context.vm.push(value);
        Ok(ShouldExit::False)
    }
}
