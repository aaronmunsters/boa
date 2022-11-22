use crate::{
    property::{PropertyDescriptor, PropertyKey},
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult, JsString,
};

#[cfg(feature = "instrumentation")]
use crate::instrumentation::EvaluationMode::BaseEvaluation;

/// `SetPropertyByName` implements the Opcode Operation for `Opcode::SetPropertyByName`
///
/// Operation:
///  - Sets a property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyByName;

impl Operation for SetPropertyByName {
    const NAME: &'static str = "SetPropertyByName";
    const INSTRUCTION: &'static str = "INST - SetPropertyByName";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        if let BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.set_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let index = context.vm.read::<u32>();

                        let value = context.vm.pop();
                        let object = context.vm.pop();

                        let name = context.vm.frame().code.names[index as usize];
                        let js_name = context
                            .interner()
                            .resolve_expect(name.sym())
                            .to_string()
                            .into();

                        let result = context.call(trap, &advice, &[object.into(), js_name, value]);

                        match result {
                            Ok(value) => {
                                context.instrumentation_conf.set_mode_base();

                                context.vm.stack.push(value);
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
        let object = context.vm.pop();
        let object = if let Some(object) = object.as_object() {
            object.clone()
        } else {
            object.to_object(context)?
        };

        let name = context.vm.frame().code.names[index as usize];
        let name: PropertyKey = context
            .interner()
            .resolve_expect(name.sym())
            .into_common::<JsString>(false)
            .into();

        object.set(name, value.clone(), context.vm.frame().code.strict, context)?;
        context.vm.stack.push(value);
        Ok(ShouldExit::False)
    }
}

/// `SetPropertyByValue` implements the Opcode Operation for `Opcode::SetPropertyByValue`
///
/// Operation:
///  - Sets a property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyByValue;

impl Operation for SetPropertyByValue {
    const NAME: &'static str = "SetPropertyByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertyByValue";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        if let BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.set_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let value = context.vm.pop();
                        let key = context.vm.pop();
                        let object = context.vm.pop();

                        let result = context.call(trap, &advice, &[object.into(), key, value]);

                        match result {
                            Ok(value) => {
                                context.instrumentation_conf.set_mode_base();

                                context.vm.stack.push(value);
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

        let value = context.vm.pop();
        let key = context.vm.pop();
        let object = context.vm.pop();
        let object = if let Some(object) = object.as_object() {
            object.clone()
        } else {
            object.to_object(context)?
        };

        let key = key.to_property_key(context)?;
        object.set(key, value.clone(), context.vm.frame().code.strict, context)?;
        context.vm.stack.push(value);
        Ok(ShouldExit::False)
    }
}

/// `SetPropertyGetterByName` implements the Opcode Operation for `Opcode::SetPropertyGetterByName`
///
/// Operation:
///  - Sets a getter property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyGetterByName;

impl Operation for SetPropertyGetterByName {
    const NAME: &'static str = "SetPropertyGetterByName";
    const INSTRUCTION: &'static str = "INST - SetPropertyGetterByName";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        let index = context.vm.read::<u32>();
        let value = context.vm.pop();
        let object = context.vm.pop();
        let object = object.to_object(context)?;
        let name = context.vm.frame().code.names[index as usize];
        let name = context
            .interner()
            .resolve_expect(name.sym())
            .into_common::<JsString>(false)
            .into();
        let set = object
            .__get_own_property__(&name, context)?
            .as_ref()
            .and_then(PropertyDescriptor::set)
            .cloned();
        object.__define_own_property__(
            name,
            PropertyDescriptor::builder()
                .maybe_get(Some(value))
                .maybe_set(set)
                .enumerable(true)
                .configurable(true)
                .build(),
            context,
        )?;
        Ok(ShouldExit::False)
    }
}

/// `SetPropertyGetterByValue` implements the Opcode Operation for `Opcode::SetPropertyGetterByValue`
///
/// Operation:
///  - Sets a getter property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyGetterByValue;

impl Operation for SetPropertyGetterByValue {
    const NAME: &'static str = "SetPropertyGetterByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertyGetterByValue";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        let value = context.vm.pop();
        let key = context.vm.pop();
        let object = context.vm.pop();
        let object = object.to_object(context)?;
        let name = key.to_property_key(context)?;
        let set = object
            .__get_own_property__(&name, context)?
            .as_ref()
            .and_then(PropertyDescriptor::set)
            .cloned();
        object.__define_own_property__(
            name,
            PropertyDescriptor::builder()
                .maybe_get(Some(value))
                .maybe_set(set)
                .enumerable(true)
                .configurable(true)
                .build(),
            context,
        )?;
        Ok(ShouldExit::False)
    }
}

/// `SetPropertySetterByName` implements the Opcode Operation for `Opcode::SetPropertySetterByName`
///
/// Operation:
///  - Sets a setter property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertySetterByName;

impl Operation for SetPropertySetterByName {
    const NAME: &'static str = "SetPropertySetterByName";
    const INSTRUCTION: &'static str = "INST - SetPropertySetterByName";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        let index = context.vm.read::<u32>();
        let value = context.vm.pop();
        let object = context.vm.pop();
        let object = object.to_object(context)?;
        let name = context.vm.frame().code.names[index as usize];
        let name = context
            .interner()
            .resolve_expect(name.sym())
            .into_common::<JsString>(false)
            .into();
        let get = object
            .__get_own_property__(&name, context)?
            .as_ref()
            .and_then(PropertyDescriptor::get)
            .cloned();
        object.__define_own_property__(
            name,
            PropertyDescriptor::builder()
                .maybe_set(Some(value))
                .maybe_get(get)
                .enumerable(true)
                .configurable(true)
                .build(),
            context,
        )?;
        Ok(ShouldExit::False)
    }
}

/// `SetPropertySetterByValue` implements the Opcode Operation for `Opcode::SetPropertySetterByValue`
///
/// Operation:
///  - Sets a setter property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertySetterByValue;

impl Operation for SetPropertySetterByValue {
    const NAME: &'static str = "SetPropertySetterByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertySetterByValue";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        let value = context.vm.pop();
        let key = context.vm.pop();
        let object = context.vm.pop();
        let object = object.to_object(context)?;
        let name = key.to_property_key(context)?;
        let get = object
            .__get_own_property__(&name, context)?
            .as_ref()
            .and_then(PropertyDescriptor::get)
            .cloned();
        object.__define_own_property__(
            name,
            PropertyDescriptor::builder()
                .maybe_set(Some(value))
                .maybe_get(get)
                .enumerable(true)
                .configurable(true)
                .build(),
            context,
        )?;
        Ok(ShouldExit::False)
    }
}
