use crate::{
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult,
};

#[cfg(feature = "instrumentation")]
use crate::attempt_push_instr;

/// `PushLiteral` implements the Opcode Operation for `Opcode::PushLiteral`
///
/// Operation:
///  - Push literal value on the stack.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PushLiteral;

impl Operation for PushLiteral {
    const NAME: &'static str = "PushLiteral";
    const INSTRUCTION: &'static str = "INST - PushLiteral";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        attempt_push_instr!(context);

        let index = context.vm.read::<u32>() as usize;
        let value = context.vm.frame().code.literals[index].clone();
        context.vm.push(value);
        Ok(ShouldExit::False)
    }
}
