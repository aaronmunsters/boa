use crate::{
    environments::{BindingLocator, Environment},
    vm::{opcode::Operation, CompletionType},
    Context, JsNativeError, JsResult,
};

/// `ThrowMutateImmutable` implements the Opcode Operation for `Opcode::ThrowMutateImmutable`
///
/// Operation:
///  - Throws an error because the binding access is illegal.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThrowMutateImmutable;

impl ThrowMutateImmutable {
    fn operation(context: &mut Context, index: usize) -> JsResult<CompletionType> {
        let name = context.vm.frame().code_block().constant_string(index);

        Err(JsNativeError::typ()
            .with_message(format!(
                "cannot mutate an immutable binding '{}'",
                name.to_std_string_escaped()
            ))
            .into())
    }
}

impl Operation for ThrowMutateImmutable {
    const NAME: &'static str = "ThrowMutateImmutable";
    const INSTRUCTION: &'static str = "INST - ThrowMutateImmutable";
    const COST: u8 = 2;

    fn execute(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u8>();
        Self::operation(context, index as usize)
    }

    fn execute_with_u16_operands(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u16>() as usize;
        Self::operation(context, index)
    }

    fn execute_with_u32_operands(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u32>();
        Self::operation(context, index as usize)
    }
}

/// `SetName` implements the Opcode Operation for `Opcode::SetName`
///
/// Operation:
///  - Find a binding on the environment chain and assign its value.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetName;

impl SetName {
    fn operation(context: &mut Context, index: usize) -> JsResult<CompletionType> {
        let mut binding_locator = context.vm.frame().code_block.bindings[index].clone();
        let value = context.vm.pop();

        context.find_runtime_binding(&mut binding_locator)?;

        verify_initialized(&binding_locator, context)?;

        context.set_binding(
            &binding_locator,
            value,
            context.vm.frame().code_block.strict(),
        )?;

        Ok(CompletionType::Normal)
    }
}

impl Operation for SetName {
    const NAME: &'static str = "SetName";
    const INSTRUCTION: &'static str = "INST - SetName";
    const COST: u8 = 4;

    fn execute(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u8>();
        Self::operation(context, index as usize)
    }

    fn execute_with_u16_operands(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u16>() as usize;
        Self::operation(context, index)
    }

    fn execute_with_u32_operands(context: &mut Context) -> JsResult<CompletionType> {
        let index = context.vm.read::<u32>();
        Self::operation(context, index as usize)
    }
}

/// `SetNameByLocator` implements the Opcode Operation for `Opcode::SetNameByLocator`
///
/// Operation:
///  - Assigns a value to the binding pointed by the `current_binding` of the current frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetNameByLocator;

impl Operation for SetNameByLocator {
    const NAME: &'static str = "SetNameByLocator";
    const INSTRUCTION: &'static str = "INST - SetNameByLocator";
    const COST: u8 = 4;

    fn execute(context: &mut Context) -> JsResult<CompletionType> {
        let binding_locator = context
            .vm
            .frame_mut()
            .binding_stack
            .pop()
            .expect("locator should have been popped before");
        let value = context.vm.pop();

        verify_initialized(&binding_locator, context)?;

        context.set_binding(
            &binding_locator,
            value,
            context.vm.frame().code_block.strict(),
        )?;

        Ok(CompletionType::Normal)
    }
}

/// Checks that the binding pointed by `locator` exists and is initialized.
fn verify_initialized(locator: &BindingLocator, context: &mut Context) -> JsResult<()> {
    if !context.is_initialized_binding(locator)? {
        let key = locator.name();
        let strict = context.vm.frame().code_block.strict();

        let message = if locator.is_global() {
            strict.then(|| {
                format!(
                    "cannot assign to uninitialized global property `{}`",
                    key.to_std_string_escaped()
                )
            })
        } else {
            match context.environment_expect(locator.environment_index()) {
                Environment::Declarative(_) => Some(format!(
                    "cannot assign to uninitialized binding `{}`",
                    key.to_std_string_escaped()
                )),
                Environment::Object(_) if strict => Some(format!(
                    "cannot assign to uninitialized property `{}`",
                    key.to_std_string_escaped()
                )),
                Environment::Object(_) => None,
            }
        };

        if let Some(message) = message {
            return Err(JsNativeError::reference().with_message(message).into());
        }
    }

    Ok(())
}
