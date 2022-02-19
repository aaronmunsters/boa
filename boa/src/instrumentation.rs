use gc::{Finalize, Trace};

use crate::{context::Context, JsValue};

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
}

#[cfg(feature = "instrumentation")]
impl Traps {
    pub fn from(advice: &JsValue, context: &mut Context) -> Self {
        Self {
            apply_trap: Self::extract_trap(advice, "apply", context),
            get_trap: Self::extract_trap(advice, "get", context),
            set_trap: Self::extract_trap(advice, "set", context),
            read_trap: Self::extract_trap(advice, "read", context),
            write_trap: Self::extract_trap(advice, "write", context),
            unary_trap: Self::extract_trap(advice, "unary", context),
            binary_trap: Self::extract_trap(advice, "binary", context),
        }
    }

    fn extract_trap(advice: &JsValue, key: &str, context: &mut Context) -> Option<Box<JsValue>> {
        let trap = advice
            .get_field(key, context)
            .expect("Analysis definition should return an object.");
        if trap.is_undefined() {
            None
        } else {
            Some(Box::new(trap))
        }
    }
}
