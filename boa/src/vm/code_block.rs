//! `CodeBlock`
//!
//! This module is for the `CodeBlock` which implements a function representation in the VM

use crate::{
    builtins::function::{
        arguments::Arguments, Captures, ClosureFunctionSignature, Function,
        NativeFunctionSignature, ThisMode,
    },
    context::StandardObjects,
    environments::{BindingLocator, DeclarativeEnvironmentStack},
    gc::{Finalize, Gc, Trace},
    object::{internal_methods::get_prototype_from_constructor, JsObject, ObjectData},
    profiler::BoaProfiler,
    property::PropertyDescriptor,
    syntax::ast::node::FormalParameter,
    vm::{call_frame::FinallyReturn, CallFrame, Opcode},
    Context, JsResult, JsValue,
};
use boa_interner::{Interner, Sym, ToInternedString};
use std::{convert::TryInto, mem::size_of};

#[cfg(feature = "instrumentation")]
use crate::{builtins::Array, instrumentation::EvaluationMode};

/// This represents whether a value can be read from [`CodeBlock`] code.
///
/// # Safety
///
/// This trait is safe to implement as long as the type doesn't implement `Drop`.
/// At some point, if [negative impls][negative_impls] are stabilized, we might be able to remove
/// the unsafe bound.
///
/// [negative_impls]: https://doc.rust-lang.org/beta/unstable-book/language-features/negative-impls.html
pub(crate) unsafe trait Readable {}

unsafe impl Readable for u8 {}
unsafe impl Readable for i8 {}
unsafe impl Readable for u16 {}
unsafe impl Readable for i16 {}
unsafe impl Readable for u32 {}
unsafe impl Readable for i32 {}
unsafe impl Readable for u64 {}
unsafe impl Readable for i64 {}
unsafe impl Readable for f32 {}
unsafe impl Readable for f64 {}

/// The internal representation of a JavaScript function.
///
/// A `CodeBlock` is generated for each function compiled by the
/// [`ByteCompiler`](crate::bytecompiler::ByteCompiler). It stores the bytecode and the other
/// attributes of the function.
#[derive(Clone, Debug, Trace, Finalize)]
pub struct CodeBlock {
    /// Name of this function
    pub(crate) name: Sym,

    /// The number of arguments expected.
    pub(crate) length: u32,

    /// Is this function in strict mode.
    pub(crate) strict: bool,

    /// Is this function a constructor.
    pub(crate) constructor: bool,

    /// [[ThisMode]]
    pub(crate) this_mode: ThisMode,

    /// Parameters passed to this function.
    pub(crate) params: Box<[FormalParameter]>,

    /// Bytecode
    pub(crate) code: Vec<u8>,

    /// Literals
    pub(crate) literals: Vec<JsValue>,

    /// Property field names.
    pub(crate) variables: Vec<Sym>,

    /// Locators for all bindings in the codeblock.
    #[unsafe_ignore_trace]
    pub(crate) bindings: Vec<BindingLocator>,

    /// Number of binding for the function environment.
    pub(crate) num_bindings: usize,

    /// Functions inside this function
    pub(crate) functions: Vec<Gc<CodeBlock>>,

    /// Indicates if the codeblock contains a lexical name `arguments`
    pub(crate) lexical_name_argument: bool,

    /// The `arguments` binding location of the function, if set.
    #[unsafe_ignore_trace]
    pub(crate) arguments_binding: Option<BindingLocator>,
}

impl CodeBlock {
    /// Constructs a new `CodeBlock`.
    pub fn new(name: Sym, length: u32, strict: bool, constructor: bool) -> Self {
        Self {
            code: Vec::new(),
            literals: Vec::new(),
            variables: Vec::new(),
            bindings: Vec::new(),
            num_bindings: 0,
            functions: Vec::new(),
            name,
            length,
            strict,
            constructor,
            this_mode: ThisMode::Global,
            params: Vec::new().into_boxed_slice(),
            lexical_name_argument: false,
            arguments_binding: None,
        }
    }

    /// Read type T from code.
    ///
    /// # Safety
    ///
    /// Does not check if read happens out-of-bounds.
    pub(crate) unsafe fn read_unchecked<T>(&self, offset: usize) -> T
    where
        T: Readable,
    {
        // This has to be an unaligned read because we can't guarantee that
        // the types are aligned.
        self.code.as_ptr().add(offset).cast::<T>().read_unaligned()
    }

    /// Read type T from code.
    #[track_caller]
    pub(crate) fn read<T>(&self, offset: usize) -> T
    where
        T: Readable,
    {
        assert!(offset + size_of::<T>() - 1 < self.code.len());

        // Safety: We checked that it is not an out-of-bounds read,
        // so this is safe.
        unsafe { self.read_unchecked(offset) }
    }

    /// Get the operands after the `Opcode` pointed to by `pc` as a `String`.
    /// Modifies the `pc` to point to the next instruction.
    ///
    /// Returns an empty `String` if no operands are present.
    pub(crate) fn instruction_operands(&self, pc: &mut usize, interner: &Interner) -> String {
        let opcode: Opcode = self.code[*pc].try_into().expect("invalid opcode");
        *pc += size_of::<Opcode>();
        match opcode {
            Opcode::PushInt8 => {
                let result = self.read::<i8>(*pc).to_string();
                *pc += size_of::<i8>();
                result
            }
            Opcode::PushInt16 => {
                let result = self.read::<i16>(*pc).to_string();
                *pc += size_of::<i16>();
                result
            }
            Opcode::PushInt32 => {
                let result = self.read::<i32>(*pc).to_string();
                *pc += size_of::<i32>();
                result
            }
            Opcode::PushRational => {
                let operand = self.read::<f64>(*pc);
                *pc += size_of::<f64>();
                ryu_js::Buffer::new().format(operand).to_string()
            }
            Opcode::PushLiteral
            | Opcode::Jump
            | Opcode::JumpIfFalse
            | Opcode::JumpIfNotUndefined
            | Opcode::CatchStart
            | Opcode::FinallySetJump
            | Opcode::Case
            | Opcode::Default
            | Opcode::LogicalAnd
            | Opcode::LogicalOr
            | Opcode::Coalesce
            | Opcode::Call
            | Opcode::CallWithRest
            | Opcode::New
            | Opcode::NewWithRest
            | Opcode::ForInLoopInitIterator
            | Opcode::ForInLoopNext
            | Opcode::ConcatToString
            | Opcode::CopyDataProperties
            | Opcode::PushDeclarativeEnvironment => {
                let result = self.read::<u32>(*pc).to_string();
                *pc += size_of::<u32>();
                result
            }
            Opcode::TryStart => {
                let operand1 = self.read::<u32>(*pc);
                *pc += size_of::<u32>();
                let operand2 = self.read::<u32>(*pc);
                *pc += size_of::<u32>();
                format!("{operand1}, {operand2}")
            }
            Opcode::GetFunction => {
                let operand = self.read::<u32>(*pc);
                *pc += size_of::<u32>();
                format!(
                    "{operand:04}: '{:?}' (length: {})",
                    interner.resolve_expect(self.functions[operand as usize].name),
                    self.functions[operand as usize].length
                )
            }
            Opcode::DefInitArg
            | Opcode::DefVar
            | Opcode::DefInitVar
            | Opcode::DefLet
            | Opcode::DefInitLet
            | Opcode::DefInitConst
            | Opcode::GetName
            | Opcode::GetNameOrUndefined
            | Opcode::SetName => {
                let operand = self.read::<u32>(*pc);
                *pc += size_of::<u32>();
                format!(
                    "{:04}: '{}'",
                    operand,
                    interner.resolve_expect(self.bindings[operand as usize].name()),
                )
            }
            Opcode::GetPropertyByName
            | Opcode::SetPropertyByName
            | Opcode::DefineOwnPropertyByName
            | Opcode::SetPropertyGetterByName
            | Opcode::SetPropertySetterByName
            | Opcode::DeletePropertyByName => {
                let operand = self.read::<u32>(*pc);
                *pc += size_of::<u32>();
                format!(
                    "{operand:04}: '{}'",
                    interner.resolve_expect(self.variables[operand as usize]),
                )
            }
            Opcode::Pop
            | Opcode::Dup
            | Opcode::Swap
            | Opcode::PushZero
            | Opcode::PushOne
            | Opcode::PushNaN
            | Opcode::PushPositiveInfinity
            | Opcode::PushNegativeInfinity
            | Opcode::PushNull
            | Opcode::PushTrue
            | Opcode::PushFalse
            | Opcode::PushUndefined
            | Opcode::PushEmptyObject
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Div
            | Opcode::Mul
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::ShiftRight
            | Opcode::ShiftLeft
            | Opcode::UnsignedShiftRight
            | Opcode::BitOr
            | Opcode::BitAnd
            | Opcode::BitXor
            | Opcode::BitNot
            | Opcode::In
            | Opcode::Eq
            | Opcode::StrictEq
            | Opcode::NotEq
            | Opcode::StrictNotEq
            | Opcode::GreaterThan
            | Opcode::GreaterThanOrEq
            | Opcode::LessThan
            | Opcode::LessThanOrEq
            | Opcode::InstanceOf
            | Opcode::TypeOf
            | Opcode::Void
            | Opcode::LogicalNot
            | Opcode::Pos
            | Opcode::Neg
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::GetPropertyByValue
            | Opcode::SetPropertyByValue
            | Opcode::DefineOwnPropertyByValue
            | Opcode::SetPropertyGetterByValue
            | Opcode::SetPropertySetterByValue
            | Opcode::DeletePropertyByValue
            | Opcode::ToBoolean
            | Opcode::Throw
            | Opcode::TryEnd
            | Opcode::CatchEnd
            | Opcode::CatchEnd2
            | Opcode::FinallyStart
            | Opcode::FinallyEnd
            | Opcode::This
            | Opcode::Return
            | Opcode::PushFunctionEnvironment
            | Opcode::PopEnvironment
            | Opcode::LoopStart
            | Opcode::LoopContinue
            | Opcode::LoopEnd
            | Opcode::InitIterator
            | Opcode::IteratorNext
            | Opcode::IteratorNextFull
            | Opcode::IteratorClose
            | Opcode::IteratorToArray
            | Opcode::RequireObjectCoercible
            | Opcode::ValueNotNullOrUndefined
            | Opcode::RestParameterInit
            | Opcode::RestParameterPop
            | Opcode::PushValueToArray
            | Opcode::PushIteratorToArray
            | Opcode::PushNewArray
            | Opcode::PopOnReturnAdd
            | Opcode::PopOnReturnSub
            | Opcode::Nop => String::new(),
        }
    }
}

impl ToInternedString for CodeBlock {
    fn to_interned_string(&self, interner: &Interner) -> String {
        let name = interner.resolve_expect(self.name);
        let mut f = if self.name == Sym::MAIN {
            String::new()
        } else {
            "\n".to_owned()
        };

        f.push_str(&format!(
            "{:-^70}\n    Location  Count   Opcode                     Operands\n\n",
            format!("Compiled Output: '{name}'"),
        ));

        let mut pc = 0;
        let mut count = 0;
        while pc < self.code.len() {
            let opcode: Opcode = self.code[pc].try_into().expect("invalid opcode");
            let operands = self.instruction_operands(&mut pc, interner);
            f.push_str(&format!(
                "    {pc:06}    {count:04}    {:<27}\n{operands}",
                opcode.as_str(),
            ));
            count += 1;
        }

        f.push_str("\nLiterals:\n");

        if self.literals.is_empty() {
            f.push_str("    <empty>");
        } else {
            for (i, value) in self.literals.iter().enumerate() {
                f.push_str(&format!(
                    "    {i:04}: <{}> {}\n",
                    value.type_of(),
                    value.display()
                ));
            }
        }

        f.push_str("\nNames:\n");
        if self.variables.is_empty() {
            f.push_str("    <empty>");
        } else {
            for (i, value) in self.variables.iter().enumerate() {
                f.push_str(&format!(
                    "    {i:04}: {}\n",
                    interner.resolve_expect(*value)
                ));
            }
        }

        f.push_str("\nFunctions:\n");
        if self.functions.is_empty() {
            f.push_str("    <empty>");
        } else {
            for (i, code) in self.functions.iter().enumerate() {
                f.push_str(&format!(
                    "    {i:04}: name: '{}' (length: {})\n",
                    interner.resolve_expect(code.name),
                    code.length
                ));
            }
        }

        f
    }
}

#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct JsVmFunction {}

impl JsVmFunction {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(code: Gc<CodeBlock>, context: &mut Context) -> JsObject {
        let _timer = BoaProfiler::global().start_event("Identifier", "vm");

        let function_prototype = context.standard_objects().function_object().prototype();

        let prototype = context.construct_object();

        let name_property = PropertyDescriptor::builder()
            .value(context.interner().resolve_expect(code.name))
            .writable(false)
            .enumerable(false)
            .configurable(true)
            .build();

        let length_property = PropertyDescriptor::builder()
            .value(code.length)
            .writable(false)
            .enumerable(false)
            .configurable(true)
            .build();

        let function = Function::VmOrdinary {
            code,
            environments: context.realm.environments.clone(),
            #[cfg(feature = "instrumentation")]
            evaluation_mode: context.instrumentation_conf.mode(),
        };

        let constructor =
            JsObject::from_proto_and_data(function_prototype, ObjectData::function(function));

        let constructor_property = PropertyDescriptor::builder()
            .value(constructor.clone())
            .writable(true)
            .enumerable(false)
            .configurable(true)
            .build();

        prototype
            .define_property_or_throw("constructor", constructor_property, context)
            .expect("failed to define the constructor property of the function");

        let prototype_property = PropertyDescriptor::builder()
            .value(prototype)
            .writable(true)
            .enumerable(false)
            .configurable(false)
            .build();

        constructor
            .define_property_or_throw("prototype", prototype_property, context)
            .expect("failed to define the prototype property of the function");
        constructor
            .define_property_or_throw("name", name_property, context)
            .expect("failed to define the name property of the function");
        constructor
            .define_property_or_throw("length", length_property, context)
            .expect("failed to define the length property of the function");

        constructor
    }
}

pub(crate) enum FunctionBody {
    Ordinary {
        code: Gc<CodeBlock>,
        environments: DeclarativeEnvironmentStack,
        #[cfg(feature = "instrumentation")]
        evaluation_mode: EvaluationMode,
    },
    Native {
        function: NativeFunctionSignature,
    },
    Closure {
        function: Box<dyn ClosureFunctionSignature>,
        captures: Captures,
    },
}

// TODO: this should be modified to not take `exit_on_return` and then moved to `internal_methods`
impl JsObject {
    pub(crate) fn call_internal(
        &self,
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let this_function_object = self.clone();
        // let mut has_parameter_expressions = false;

        if !self.is_callable() {
            return context.throw_type_error("not a callable function");
        }

        let mut construct = false;

        #[cfg(feature = "instrumentation")]
        if let EvaluationMode::BaseEvaluation = context.instrumentation_conf.mode() {
            if let Some(traps) = &mut context.instrumentation_conf.traps {
                let traps = traps.clone();
                if let Some(ref trap) = traps.apply_trap {
                    if let Some(advice) = context.instrumentation_conf.advice() {
                        context.instrumentation_conf.set_mode_meta();

                        let js_args = Array::create_array_from_list(args.to_owned(), context);

                        let result = context.call(
                            trap,
                            &advice,
                            &[
                                JsValue::from(this_function_object),
                                this.clone(),
                                JsValue::from(js_args),
                            ],
                        );

                        context.instrumentation_conf.set_mode_base();

                        return result;
                    }
                }
            }
        }

        let body = {
            let object = self.borrow();
            let function = object.as_function().expect("not a function");

            match function {
                Function::Native {
                    function,
                    constructor,
                } => {
                    if *constructor {
                        construct = true;
                    }

                    FunctionBody::Native {
                        function: *function,
                    }
                }
                Function::Closure {
                    function, captures, ..
                } => FunctionBody::Closure {
                    function: function.clone(),
                    captures: captures.clone(),
                },
                Function::VmOrdinary {
                    code,
                    environments,
                    #[cfg(feature = "instrumentation")]
                    evaluation_mode,
                } => FunctionBody::Ordinary {
                    code: code.clone(),
                    environments: environments.clone(),
                    #[cfg(feature = "instrumentation")]
                    evaluation_mode: evaluation_mode.clone(),
                },
            }
        };

        match body {
            FunctionBody::Native { function } if construct => {
                function(&JsValue::undefined(), args, context)
            }
            FunctionBody::Native { function } => function(this, args, context),
            FunctionBody::Closure { function, captures } => {
                (function)(this, args, captures, context)
            }
            FunctionBody::Ordinary {
                code,
                mut environments,
                #[cfg(feature = "instrumentation")]
                evaluation_mode,
            } => {
                std::mem::swap(&mut environments, &mut context.realm.environments);

                let lexical_this_mode = code.this_mode == ThisMode::Lexical;

                let this = if lexical_this_mode {
                    if let Some(this) = context.realm.environments.get_last_this() {
                        this
                    } else {
                        context.global_object().clone().into()
                    }
                } else if (!code.strict && !context.strict()) && this.is_null_or_undefined() {
                    context.global_object().clone().into()
                } else {
                    this.clone()
                };

                context
                    .realm
                    .environments
                    .push_function(code.num_bindings, this.clone());

                let mut arguments_in_parameter_names = false;
                let mut is_simple_parameter_list = true;
                let mut has_parameter_expressions = false;

                for param in code.params.iter() {
                    has_parameter_expressions = has_parameter_expressions || param.init().is_some();
                    arguments_in_parameter_names =
                        arguments_in_parameter_names || param.names().contains(&Sym::ARGUMENTS);
                    is_simple_parameter_list = is_simple_parameter_list
                        && !param.is_rest_param()
                        && param.is_identifier()
                        && param.init().is_none();
                }

                if let Some(binding) = code.arguments_binding {
                    let arguments_obj =
                        if context.strict() || code.strict || !is_simple_parameter_list {
                            Arguments::create_unmapped_arguments_object(args, context)
                        } else {
                            let env = context.realm.environments.current();
                            Arguments::create_mapped_arguments_object(
                                &this_function_object,
                                &code.params,
                                args,
                                &env,
                                context,
                            )
                        };
                    context.realm.environments.put_value(
                        binding.environment_index(),
                        binding.binding_index(),
                        arguments_obj.into(),
                    );
                }

                let arg_count = args.len();

                // Push function arguments to the stack.
                let args = if code.params.len() > args.len() {
                    let mut v = args.to_vec();
                    v.extend(vec![JsValue::Undefined; code.params.len() - args.len()]);
                    v
                } else {
                    args.to_vec()
                };

                for arg in args.iter().rev() {
                    context.vm.push(arg);
                }

                let param_count = code.params.len();

                context.vm.push_frame(CallFrame {
                    prev: None,
                    code,
                    this,
                    pc: 0,
                    catch: Vec::new(),
                    finally_return: FinallyReturn::None,
                    finally_jump: Vec::new(),
                    pop_on_return: 0,
                    loop_env_stack: vec![0],
                    try_env_stack: vec![crate::vm::TryStackEntry {
                        num_env: 0,
                        num_loop_stack_entries: 0,
                    }],
                    param_count,
                    arg_count,
                });

                let outer_evaluation_mode = context.instrumentation_conf.mode();
                context.instrumentation_conf.set_mode(evaluation_mode);

                let result = context.run();
                context.vm.pop_frame().expect("must have frame");

                context.instrumentation_conf.set_mode(outer_evaluation_mode);

                context.realm.environments.pop();
                if has_parameter_expressions {
                    context.realm.environments.pop();
                }

                std::mem::swap(&mut environments, &mut context.realm.environments);

                result
            }
        }
    }

    pub(crate) fn construct_internal(
        &self,
        args: &[JsValue],
        this_target: &JsValue,
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let this_function_object = self.clone();
        // let mut has_parameter_expressions = false;

        if !self.is_constructor() {
            return context.throw_type_error("not a constructor function");
        }

        let body = {
            let object = self.borrow();
            let function = object.as_function().expect("not a function");

            match function {
                Function::Native { function, .. } => FunctionBody::Native {
                    function: *function,
                },
                Function::Closure {
                    function, captures, ..
                } => FunctionBody::Closure {
                    function: function.clone(),
                    captures: captures.clone(),
                },
                Function::VmOrdinary {
                    code,
                    environments,
                    #[cfg(feature = "instrumentation")]
                    evaluation_mode,
                } => FunctionBody::Ordinary {
                    code: code.clone(),
                    environments: environments.clone(),
                    #[cfg(feature = "instrumentation")]
                    evaluation_mode: evaluation_mode.clone(),
                },
            }
        };

        match body {
            FunctionBody::Native { function, .. } => function(this_target, args, context),
            FunctionBody::Closure { function, captures } => {
                (function)(this_target, args, captures, context)
            }
            FunctionBody::Ordinary {
                code,
                mut environments,
                #[cfg(feature = "instrumentation")]
                    evaluation_mode: _,
            } => {
                std::mem::swap(&mut environments, &mut context.realm.environments);

                let this: JsValue = {
                    // If the prototype of the constructor is not an object, then use the default object
                    // prototype as prototype for the new object
                    // see <https://tc39.es/ecma262/#sec-ordinarycreatefromconstructor>
                    // see <https://tc39.es/ecma262/#sec-getprototypefromconstructor>
                    let prototype = get_prototype_from_constructor(
                        this_target,
                        StandardObjects::object_object,
                        context,
                    )?;
                    Self::from_proto_and_data(prototype, ObjectData::ordinary()).into()
                };

                context
                    .realm
                    .environments
                    .push_function(code.num_bindings, this.clone());

                let mut arguments_in_parameter_names = false;
                let mut is_simple_parameter_list = true;
                let mut has_parameter_expressions = false;

                for param in code.params.iter() {
                    has_parameter_expressions = has_parameter_expressions || param.init().is_some();
                    arguments_in_parameter_names =
                        arguments_in_parameter_names || param.names().contains(&Sym::ARGUMENTS);
                    is_simple_parameter_list = is_simple_parameter_list
                        && !param.is_rest_param()
                        && param.is_identifier()
                        && param.init().is_none();
                }

                if let Some(binding) = code.arguments_binding {
                    let arguments_obj =
                        if context.strict() || code.strict || !is_simple_parameter_list {
                            Arguments::create_unmapped_arguments_object(args, context)
                        } else {
                            let env = context.realm.environments.current();
                            Arguments::create_mapped_arguments_object(
                                &this_function_object,
                                &code.params,
                                args,
                                &env,
                                context,
                            )
                        };
                    context.realm.environments.put_value(
                        binding.environment_index(),
                        binding.binding_index(),
                        arguments_obj.into(),
                    );
                }

                let arg_count = args.len();

                // Push function arguments to the stack.
                let args = if code.params.len() > args.len() {
                    let mut v = args.to_vec();
                    v.extend(vec![JsValue::Undefined; code.params.len() - args.len()]);
                    v
                } else {
                    args.to_vec()
                };

                for arg in args.iter().rev() {
                    context.vm.push(arg);
                }

                let param_count = code.params.len();

                let this = if (!code.strict && !context.strict()) && this.is_null_or_undefined() {
                    context.global_object().clone().into()
                } else {
                    this
                };

                context.vm.push_frame(CallFrame {
                    prev: None,
                    code,
                    this,
                    pc: 0,
                    catch: Vec::new(),
                    finally_return: FinallyReturn::None,
                    finally_jump: Vec::new(),
                    pop_on_return: 0,
                    loop_env_stack: vec![0],
                    try_env_stack: vec![crate::vm::TryStackEntry {
                        num_env: 0,
                        num_loop_stack_entries: 0,
                    }],
                    param_count,
                    arg_count,
                });

                let result = context.run();

                let frame = context.vm.pop_frame().expect("must have frame");

                let this = frame.this;

                context.realm.environments.pop();
                if has_parameter_expressions {
                    context.realm.environments.pop();
                }

                std::mem::swap(&mut environments, &mut context.realm.environments);

                let result = result?;

                if result.is_object() {
                    Ok(result)
                } else {
                    Ok(this)
                }
            }
        }
    }
}
