use std::ops::Neg;

use gc::{Finalize, Trace};

use crate::{
    builtins::Number,
    object::ObjectInitializer,
    value::{Numeric, PreferredType},
    Context, JsBigInt, JsResult, JsValue,
};
use tap::Conv;

#[cfg(feature = "instrumentation")]
#[derive(Debug)]
pub struct InstrumentationConf {
    pub traps: Option<Traps>,

    pub advice: Option<Box<JsValue>>,

    evaluation_mode: EvaluationMode,
}

#[cfg(feature = "instrumentation")]
impl InstrumentationConf {
    pub fn mode(&self) -> EvaluationMode {
        self.evaluation_mode.clone()
    }

    pub fn set_mode(&mut self, mode: EvaluationMode) {
        self.evaluation_mode = mode;
    }

    pub fn set_mode_meta(&mut self) {
        self.evaluation_mode = EvaluationMode::MetaEvaluation;
    }

    pub fn set_mode_base(&mut self) {
        self.evaluation_mode = EvaluationMode::BaseEvaluation;
    }

    pub fn install_traps(&mut self, advice: Traps) {
        self.traps = Some(advice);
    }

    pub fn install_advice(&mut self, advice: JsValue) {
        self.advice = Some(Box::new(advice));
    }

    pub fn advice(&self) -> Option<Box<JsValue>> {
        self.advice.clone()
    }
}

#[cfg(feature = "instrumentation")]
impl Default for InstrumentationConf {
    fn default() -> Self {
        Self {
            traps: None,
            advice: None,
            evaluation_mode: EvaluationMode::BaseEvaluation,
        }
    }
}

#[cfg(feature = "instrumentation")]
#[derive(Trace, Finalize, Debug, Clone)]
pub enum EvaluationMode {
    BaseEvaluation,
    MetaEvaluation,
}

#[cfg(feature = "instrumentation")]
#[derive(Trace, Finalize, Debug, Clone)]
pub struct Traps {
    pub apply_trap: Option<Box<JsValue>>,
    pub get_trap: Option<Box<JsValue>>,
    pub set_trap: Option<Box<JsValue>>,
    pub read_trap: Option<Box<JsValue>>,
    pub write_trap: Option<Box<JsValue>>,
    pub unary_trap: Option<Box<JsValue>>,
    pub binary_trap: Option<Box<JsValue>>,
    pub primitive_trap: Option<Box<JsValue>>,
    pub to_primitive_trap: Option<Box<JsValue>>,
}

#[cfg(feature = "instrumentation")]
impl Traps {
    pub fn from(advice: &JsValue, context: &mut Context) -> Self {
        if let None = advice.as_object() {
            panic!("Analysis definition should return an object.")
        }
        Self {
            apply_trap: Self::extract_trap(advice, "apply", context),
            get_trap: Self::extract_trap(advice, "get", context),
            set_trap: Self::extract_trap(advice, "set", context),
            read_trap: Self::extract_trap(advice, "read", context),
            write_trap: Self::extract_trap(advice, "write", context),
            unary_trap: Self::extract_trap(advice, "unary", context),
            binary_trap: Self::extract_trap(advice, "binary", context),
            primitive_trap: Self::extract_trap(advice, "primitive", context),
            to_primitive_trap: Self::extract_trap(advice, "toPrimitive", context),
        }
    }

    fn extract_trap(advice: &JsValue, key: &str, context: &mut Context) -> Option<Box<JsValue>> {
        match advice.get_v(key, context) {
            Err(_) => panic!("Uncaught: error while fetching trap for key {}", key),
            Ok(value) => {
                if value.is_undefined() {
                    None
                } else {
                    Some(Box::new(value.clone()))
                }
            }
        }
    }
}

#[cfg(feature = "instrumentation")]
#[derive(Debug, Clone, Copy)]
pub struct Hooks;

#[cfg(feature = "instrumentation")]
impl Hooks {
    pub(crate) fn default(context: &mut Context) -> JsValue {
        ObjectInitializer::new(context)
            .function(Self::binary, "binary", 3)
            .function(Self::unary, "unary", 2)
            .function(Self::to_primitive, "toPrimitive", 2)
            .build()
            .conv::<JsValue>()
    }

    fn unary(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        context.instrumentation_conf.set_mode_base();
        let operator = args
            .get(0)
            .expect("Instrumentation: binary hook missing operator")
            .clone();
        let operand = args
            .get(1)
            .expect("Instrumentation: binary hook missing left operand")
            .clone();
        let result: JsValue = match operator
            .as_string()
            .expect("Operand type required to be string")
            .as_str()
        {
            "void" => JsValue::undefined(),
            "typeof" => operand.type_of().into(),
            "+" => operand.to_number(context)?.into(),
            "-" => match operand.to_numeric(context)? {
                Numeric::Number(number) => number.neg().into(),
                Numeric::BigInt(bigint) => JsBigInt::neg(&bigint).into(),
            },
            "++" => match operand.to_numeric(context)? {
                Numeric::Number(number) => (number + 1f64).into(),
                Numeric::BigInt(bigint) => JsBigInt::add(&bigint, &JsBigInt::one()).into(),
            },
            "--" => match operand.to_numeric(context)? {
                Numeric::Number(number) => (number - 1f64).into(),
                Numeric::BigInt(bigint) => JsBigInt::sub(&bigint, &JsBigInt::one()).into(),
            },
            "!" => (!operand.to_boolean()).into(),
            "~" => match operand.to_numeric(context)? {
                Numeric::Number(number) => Number::not(number).into(),
                Numeric::BigInt(bigint) => JsBigInt::not(&bigint).into(),
            },
            _op => {
                return context
                    .throw_error(format!("Unary hook operator should be known, got {}", _op))
            }
        };
        context.instrumentation_conf.set_mode_meta();
        Ok(result)
    }

    fn binary(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        context.instrumentation_conf.set_mode_base();
        let op = args
            .get(0)
            .expect("Instrumentation: binary hook missing operator")
            .clone();
        let l = args
            .get(1)
            .expect("Instrumentation: binary hook missing left operand")
            .clone();
        let r = args
            .get(2)
            .expect("Instrumentation: binary hook missing right operand")
            .clone();
        let result: JsValue = match op
            .as_string()
            .expect("Operand type required to be string")
            .as_str()
        {
            "+" => l.add(&r, context)?,
            "-" => l.sub(&r, context)?,
            "*" => l.mul(&r, context)?,
            "/" => l.div(&r, context)?,
            "**" => l.pow(&r, context)?,
            "%" => l.rem(&r, context)?,
            "&" => l.bitand(&r, context)?,
            "|" => l.bitor(&r, context)?,
            "^" => l.bitxor(&r, context)?,
            "<<" => l.shl(&r, context)?,
            ">>" => l.shr(&r, context)?,
            ">>>" => l.ushr(&r, context)?,
            "==" => l.equals(&r, context)?.into(),
            "!=" => l.equals(&r, context)?.into(),
            "===" => l.strict_equals(&r).into(),
            "!==" => (!l.strict_equals(&r)).into(),
            ">" => l.gt(&r, context)?.into(),
            ">=" => l.ge(&r, context)?.into(),
            "<" => l.lt(&r, context)?.into(),
            "<=" => l.le(&r, context)?.into(),
            "in" => {
                if !r.is_object() {
                    return context.throw_type_error(format!(
                        "right-hand side of 'in' should be an object, got {}",
                        r.type_of()
                    ));
                }
                let key = r.to_property_key(context)?;
                context.has_property(&r, &key)?.into()
            }
            "instanceof" => {
                let target = context.vm.pop();
                let v = context.vm.pop();
                v.instance_of(&target, context)?.into()
            }
            _op => {
                return context
                    .throw_error(format!("Binary hook operator should be known, got {}", _op))
            }
        };
        context.instrumentation_conf.set_mode_meta();
        Ok(result)
    }

    fn to_primitive(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        context.instrumentation_conf.set_mode_base();

        let sign_warning = "Instrumentation: Uncaught: to_primitive hook expects 2 arguments";
        let type_warning = "Instrumentation: to_primitive hook second argument should be a string";

        let value: &JsValue = args.get(0).expect(sign_warning);
        let preferred_type: &JsValue = args.get(1).expect(sign_warning);

        let preferred_type = match preferred_type.as_string() {
            Some(preferred_type_str) => match preferred_type_str.as_str() {
                "default" => PreferredType::Default,
                "string" => PreferredType::String,
                "number" => PreferredType::Number,
                _ => panic!("Instrumentation: Uncaught: to_primitive hook expects 2 arguments"),
            },
            None => return context.throw_type_error(type_warning),
        };

        let res = JsValue::to_primitive(value, context, preferred_type)?;

        context.instrumentation_conf.set_mode_meta();
        Ok(res)
    }
}
