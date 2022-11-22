use crate::{
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult, JsValue,
};

pub(crate) mod array;
pub(crate) mod class;
pub(crate) mod environment;
pub(crate) mod literal;
pub(crate) mod new_target;
pub(crate) mod numbers;
pub(crate) mod object;

pub(crate) use array::*;
pub(crate) use class::*;
pub(crate) use environment::*;
pub(crate) use literal::*;
pub(crate) use new_target::*;
pub(crate) use numbers::*;
pub(crate) use object::*;

#[cfg(feature = "instrumentation")]
#[macro_export]
macro_rules! attempt_push_instr {
    ($context: expr) => {
        use crate::instrumentation::EvaluationMode;

        if let EvaluationMode::BaseEvaluation = $context.instrumentation_conf.mode() {
            if let Some(traps) = &mut $context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.primitive_trap {
                    if let Some(advice) = $context.instrumentation_conf.advice() {
                        $context.instrumentation_conf.set_mode_meta();
                        $context.vm.frame_mut().pc -= 1;
                        let _ = $context.execute_instruction();
                        let value = $context.vm.pop();
                        let result = $context.call(trap, &advice, &[value]);

                        match result {
                            Ok(result) => {
                                $context.instrumentation_conf.set_mode_base();
                                $context.vm.push(result);
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
    };
}

macro_rules! implement_push_generics {
    ($name:ident, $push_value:expr, $doc_string:literal) => {
        #[doc= concat!("`", stringify!($name), "` implements the OpCode Operation for `Opcode::", stringify!($name), "`\n")]
        #[doc= "\n"]
        #[doc="Operation:\n"]
        #[doc= concat!(" - ", $doc_string)]
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $name;

        impl Operation for $name {
            const NAME: &'static str = stringify!($name);
            const INSTRUCTION: &'static str = stringify!("INST - " + $name);

            fn execute(context: &mut Context) -> JsResult<ShouldExit> {
                #[cfg(feature = "instrumentation")]
                attempt_push_instr!(context);

                context.vm.push($push_value);
                Ok(ShouldExit::False)
            }
        }
    };
}

implement_push_generics!(
    PushUndefined,
    JsValue::undefined(),
    "Push integer `undefined` on the stack."
);
implement_push_generics!(
    PushNull,
    JsValue::null(),
    "Push integer `null` on the stack."
);
implement_push_generics!(PushTrue, true, "Push integer `true` on the stack.");
implement_push_generics!(PushFalse, false, "Push integer `false` on the stack.");
implement_push_generics!(PushZero, 0, "Push integer `0` on the stack.");
implement_push_generics!(PushOne, 1, "Push integer `1` on the stack.");
implement_push_generics!(PushNaN, JsValue::nan(), "Push integer `NaN` on the stack.");
implement_push_generics!(
    PushPositiveInfinity,
    JsValue::positive_infinity(),
    "Push integer `Infinity` on the stack."
);
implement_push_generics!(
    PushNegativeInfinity,
    JsValue::negative_infinity(),
    "Push integer `-Infinity` on the stack."
);
