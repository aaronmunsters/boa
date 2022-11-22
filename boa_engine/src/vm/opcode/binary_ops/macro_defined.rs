use crate::{
    vm::{opcode::Operation, ShouldExit},
    Context, JsResult,
};

#[cfg(feature = "instrumentation")]
#[macro_export]
macro_rules! attempt_binary_instr {
    ($context: expr, $op_string: literal) => {
        use crate::instrumentation::EvaluationMode;

        if let EvaluationMode::BaseEvaluation = $context.instrumentation_conf.mode() {
            if let Some(traps) = &mut $context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.binary_trap {
                    if let Some(advice) = $context.instrumentation_conf.advice() {
                        $context.instrumentation_conf.set_mode_meta();

                        let rhs = $context.vm.pop();
                        let lhs = $context.vm.pop();

                        let result = $context.call(trap, &advice, &[$op_string.into(), lhs, rhs]);

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

macro_rules! implement_bin_ops {
    ($name:ident, $op:ident, $doc_string:literal,  $instr_string:literal) => {
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
                attempt_binary_instr!(context, $instr_string);

                let rhs = context.vm.pop();
                let lhs = context.vm.pop();
                let value = lhs.$op(&rhs, context)?;
                context.vm.push(value);
                Ok(ShouldExit::False)
            }
        }
    };
}

implement_bin_ops!(Add, add, "Binary `+` operator.", "+");
implement_bin_ops!(Sub, sub, "Binary `-` operator.", "-");
implement_bin_ops!(Mul, mul, "Binary `*` operator.", "*");
implement_bin_ops!(Div, div, "Binary `/` operator.", "/");
implement_bin_ops!(Pow, pow, "Binary `**` operator.", "**");
implement_bin_ops!(Mod, rem, "Binary `%` operator.", "%");
implement_bin_ops!(BitAnd, bitand, "Binary `&` operator.", "&");
implement_bin_ops!(BitOr, bitor, "Binary `|` operator.", "|");
implement_bin_ops!(BitXor, bitxor, "Binary `^` operator.", "^");
implement_bin_ops!(ShiftLeft, shl, "Binary `<<` operator.", "<<");
implement_bin_ops!(ShiftRight, shr, "Binary `>>` operator.", ">>");
implement_bin_ops!(UnsignedShiftRight, ushr, "Binary `>>>` operator.", ">>>");
implement_bin_ops!(Eq, equals, "Binary `==` operator.", "==");
implement_bin_ops!(GreaterThan, gt, "Binary `>` operator.", ">");
implement_bin_ops!(GreaterThanOrEq, ge, "Binary `>=` operator.", ">=");
implement_bin_ops!(LessThan, lt, "Binary `<` operator.", "<");
implement_bin_ops!(LessThanOrEq, le, "Binary `<=` operator.", "<=");
