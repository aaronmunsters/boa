use crate::{
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult, JsValue,
};

#[cfg(feature = "instrumentation")]
use crate::attempt_unary_instr;

/// `Void` implements the Opcode Operation for `Opcode::Void`
///
/// Operation:
///  - Unary `void` operator.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Void;

impl Operation for Void {
    const NAME: &'static str = "Void";
    const INSTRUCTION: &'static str = "INST - Void";

    fn execute(context: &mut Context) -> JsResult<ShouldExit> {
        #[cfg(feature = "instrumentation")]
        attempt_unary_instr!(context, "void");

        let _old = context.vm.pop();
        context.vm.push(JsValue::undefined());
        Ok(ShouldExit::False)
    }
}
