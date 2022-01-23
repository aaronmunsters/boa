//! CallFrame
//! This module will provides everything needed to implement the CallFrame

use crate::{
    gc::{Finalize, Gc, Trace},
    vm::CodeBlock,
    JsValue,
};

#[derive(Clone, Debug, Finalize, Trace)]
pub struct CallFrame {
    pub(crate) prev: Option<Box<Self>>,
    pub(crate) code: Gc<CodeBlock>,
    pub(crate) pc: usize,
    pub(crate) this: JsValue,
    #[unsafe_ignore_trace]
    pub(crate) catch: Vec<CatchAddresses>,
    #[unsafe_ignore_trace]
    pub(crate) finally_return: FinallyReturn,
    pub(crate) finally_jump: Vec<Option<u32>>,
    pub(crate) pop_on_return: usize,
    pub(crate) pop_env_on_return: usize,
    pub(crate) param_count: usize,
    pub(crate) arg_count: usize,
    #[unsafe_ignore_trace]
    pub(crate) generator_resume_kind: GeneratorResumeKind,
}

#[derive(Clone, Debug)]
pub(crate) struct CatchAddresses {
    pub(crate) next: u32,
    pub(crate) finally: Option<u32>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum FinallyReturn {
    None,
    Ok,
    Err,
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum GeneratorResumeKind {
    Normal,
    Throw,
    Return,
}
