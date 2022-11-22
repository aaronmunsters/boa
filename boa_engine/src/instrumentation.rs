use std::ops::Neg;

use crate::{
    builtins::Number,
    object::ObjectInitializer,
    symbol::WellKnownSymbols,
    value::{Numeric, PreferredType},
    Context, JsBigInt, JsError, JsNativeError, JsResult, JsValue,
};
use boa_gc::{Finalize, Trace};
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
            .expect("Instrumentation: binary hook missing operand")
            .clone();
        let result: JsValue = match operator
            .as_string()
            .expect("Operand type required to be string")
            .to_std_string_escaped()
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
                context.instrumentation_conf.set_mode_meta();
                return Err(JsError::from_native(JsNativeError::typ().with_message(
                    format!("Unary hook operator should be known, got {}", _op),
                )));
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
            .to_std_string_escaped()
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
            "!=" => (!l.equals(&r, context)?).into(),
            "===" => l.strict_equals(&r).into(),
            "!==" => (!l.strict_equals(&r)).into(),
            ">" => l.gt(&r, context)?.into(),
            ">=" => l.ge(&r, context)?.into(),
            "<" => l.lt(&r, context)?.into(),
            "<=" => l.le(&r, context)?.into(),
            "in" => {
                if !r.is_object() {
                    context.instrumentation_conf.set_mode_meta();
                    return Err(JsError::from_native(JsNativeError::typ().with_message(
                        format!(
                            "right-hand side of 'in' should be an object, got {}",
                            r.type_of()
                        ),
                    )));
                }
                let key = r.to_property_key(context)?;
                context.has_property(&r, &key)?.into()
            }
            "instanceof" => l.instance_of(&r, context)?.into(),
            _op => {
                context.instrumentation_conf.set_mode_meta();
                return Err(JsError::from_native(JsNativeError::typ().with_message(
                    format!("Binary hook operator should be known, got {}", _op),
                )));
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
            Some(preferred_type_str) => match preferred_type_str.to_std_string_escaped().as_str() {
                "default" => PreferredType::Default,
                "string" => PreferredType::String,
                "number" => PreferredType::Number,
                _ => panic!("Instrumentation: Uncaught: to_primitive hook expects 2 arguments"),
            },
            None => {
                context.instrumentation_conf.set_mode_meta();
                return Err(JsError::from_native(
                    JsNativeError::typ().with_message(type_warning),
                ));
            }
        };

        // 1. Assert: input is an ECMAScript language value. (always a value not need to check)
        // 2. If Type(input) is Object, then
        let res = if value.is_object() {
            // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
            let exotic_to_prim = value.get_method(WellKnownSymbols::to_primitive(), context)?;

            // b. If exoticToPrim is not undefined, then
            if let Some(exotic_to_prim) = exotic_to_prim {
                // i. If preferredType is not present, let hint be "default".
                // ii. Else if preferredType is string, let hint be "string".
                // iii. Else,
                //     1. Assert: preferredType is number.
                //     2. Let hint be "number".
                let hint = match preferred_type {
                    PreferredType::Default => "default",
                    PreferredType::String => "string",
                    PreferredType::Number => "number",
                }
                .into();

                // iv. Let result be ? Call(exoticToPrim, input, « hint »).
                let result = exotic_to_prim.call(value, &[hint], context)?;
                // v. If Type(result) is not Object, return result.
                // vi. Throw a TypeError exception.
                return if result.is_object() {
                    context.instrumentation_conf.set_mode_base();
                    return Err(JsError::from_native(
                        JsNativeError::typ()
                            .with_message("Symbol.toPrimitive cannot return an object"),
                    ));
                } else {
                    Ok(result)
                };
            }

            // c. If preferredType is not present, let preferredType be number.
            let preferred_type = match preferred_type {
                PreferredType::Default | PreferredType::Number => PreferredType::Number,
                PreferredType::String => PreferredType::String,
            };

            // d. Return ? OrdinaryToPrimitive(input, preferredType).
            value
                .as_object()
                .expect("self was not an object")
                .ordinary_to_primitive(context, preferred_type)
        } else {
            // 3. Return input.
            Ok(value.clone())
        };

        context.instrumentation_conf.set_mode_meta();
        res
    }
}
