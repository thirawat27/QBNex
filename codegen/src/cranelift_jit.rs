use core_types::{QError, QResult, QType};
use cranelift_codegen::ir::types;
use cranelift_codegen::ir::{AbiParam, InstBuilder, UserFuncName};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use syntax_tree::ast_nodes::{BinaryOp, Expression, Program, Statement, UnaryOp};

use std::collections::HashMap;

struct RuntimeStrings {
    values: Vec<String>,
}

extern "C" fn qb_print_number(value: f64) {
    print!("{value}");
}

extern "C" fn qb_print_newline() {
    println!();
}

extern "C" fn qb_print_string(ctx: *mut RuntimeStrings, index: i64) {
    let Some(runtime) = (unsafe { ctx.as_ref() }) else {
        return;
    };
    if let Some(value) = runtime.values.get(index.max(0) as usize) {
        print!("{value}");
    }
}

#[derive(Default)]
struct CraneliftSupport {
    strings: Vec<String>,
}

impl CraneliftSupport {
    fn validate_program(&mut self, program: &Program) -> QResult<()> {
        if !program.functions.is_empty()
            || !program.subs.is_empty()
            || !program.user_types.is_empty()
            || !program.data_statements.is_empty()
        {
            return Err(QError::UnsupportedFeature(
                "experimental cranelift-jit backend currently supports only simple top-level programs".to_string(),
            ));
        }

        for statement in &program.statements {
            self.validate_statement(statement)?;
        }

        Ok(())
    }

    fn validate_statement(&mut self, statement: &Statement) -> QResult<()> {
        match statement {
            Statement::Print {
                expressions,
                separators,
                ..
            } => {
                if separators.iter().any(|separator| separator.is_some()) {
                    return Err(QError::UnsupportedFeature(
                        "experimental cranelift-jit backend does not yet support PRINT separators"
                            .to_string(),
                    ));
                }
                for expression in expressions {
                    self.validate_print_expression(expression)?;
                }
                Ok(())
            }
            Statement::Assignment { target, value } => {
                let Expression::Variable(variable) = target else {
                    return Err(QError::UnsupportedFeature(
                        "experimental cranelift-jit backend supports only simple variable assignments".to_string(),
                    ));
                };
                if variable.name.ends_with('$') || variable.type_suffix == Some('$') {
                    return Err(QError::UnsupportedFeature(
                        "experimental cranelift-jit backend does not yet support string variables"
                            .to_string(),
                    ));
                }
                self.validate_numeric_expression(value)
            }
            Statement::End => Ok(()),
            other => Err(QError::UnsupportedFeature(format!(
                "experimental cranelift-jit backend does not yet support {:?}",
                other
            ))),
        }
    }

    fn validate_print_expression(&mut self, expression: &Expression) -> QResult<()> {
        match expression {
            Expression::Literal(QType::String(value)) => {
                self.strings.push(value.clone());
                Ok(())
            }
            _ => self.validate_numeric_expression(expression),
        }
    }

    fn validate_numeric_expression(&self, expression: &Expression) -> QResult<()> {
        match expression {
            Expression::Literal(QType::Integer(_))
            | Expression::Literal(QType::Long(_))
            | Expression::Literal(QType::Single(_))
            | Expression::Literal(QType::Double(_))
            | Expression::Variable(_) => Ok(()),
            Expression::UnaryOp {
                op: UnaryOp::Negate,
                operand,
            } => self.validate_numeric_expression(operand),
            Expression::BinaryOp { op, left, right }
                if matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
                ) =>
            {
                self.validate_numeric_expression(left)?;
                self.validate_numeric_expression(right)
            }
            other => Err(QError::UnsupportedFeature(format!(
                "experimental cranelift-jit backend does not yet support expression {:?}",
                other
            ))),
        }
    }
}

struct CraneliftProgram {
    module: JITModule,
    func_id: FuncId,
    strings: Vec<String>,
}

impl CraneliftProgram {
    fn run(mut self) -> QResult<()> {
        self.module
            .finalize_definitions()
            .map_err(|err| QError::Internal(format!("cranelift finalize failed: {err}")))?;

        let code = self.module.get_finalized_function(self.func_id);
        let function =
            unsafe { std::mem::transmute::<_, extern "C" fn(*mut RuntimeStrings) -> i32>(code) };

        let mut runtime = RuntimeStrings {
            values: self.strings,
        };
        let status = function(&mut runtime);
        if status == 0 {
            Ok(())
        } else {
            Err(QError::Runtime(format!(
                "cranelift-jit program exited with status {status}"
            )))
        }
    }
}

pub fn supports_cranelift_jit(program: &Program) -> QResult<()> {
    let mut support = CraneliftSupport::default();
    support.validate_program(program)
}

pub fn run_with_cranelift_jit(program: &Program) -> QResult<()> {
    let compiled = compile_program(program)?;
    compiled.run()
}

fn compile_program(program: &Program) -> QResult<CraneliftProgram> {
    let mut support = CraneliftSupport::default();
    support.validate_program(program)?;

    let mut flags = settings::builder();
    flags
        .set("use_colocated_libcalls", "false")
        .map_err(|err| QError::Internal(format!("cranelift flag setup failed: {err}")))?;
    flags
        .set("is_pic", "false")
        .map_err(|err| QError::Internal(format!("cranelift flag setup failed: {err}")))?;
    let isa_builder = cranelift_native::builder().map_err(|msg| {
        QError::Internal(format!("host machine is not supported by cranelift: {msg}"))
    })?;
    let isa = isa_builder
        .finish(settings::Flags::new(flags))
        .map_err(|err| QError::Internal(format!("cranelift ISA init failed: {err}")))?;

    let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
    builder.symbol("qb_print_number", qb_print_number as *const u8);
    builder.symbol("qb_print_newline", qb_print_newline as *const u8);
    builder.symbol("qb_print_string", qb_print_string as *const u8);
    let mut module = JITModule::new(builder);

    let pointer_type = module.target_config().pointer_type();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));

    let func_id = module
        .declare_function("main", Linkage::Export, &signature)
        .map_err(|err| QError::Internal(format!("cranelift declare failed: {err}")))?;

    let mut context = module.make_context();
    context.func.signature = signature.clone();
    context.func.name = UserFuncName::user(0, func_id.as_u32());

    let print_number = {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(types::F64));
        let id = module
            .declare_function("qb_print_number", Linkage::Import, &sig)
            .map_err(|err| {
                QError::Internal(format!("cranelift declare print_number failed: {err}"))
            })?;
        (id, sig)
    };
    let print_newline = {
        let sig = module.make_signature();
        let id = module
            .declare_function("qb_print_newline", Linkage::Import, &sig)
            .map_err(|err| {
                QError::Internal(format!("cranelift declare print_newline failed: {err}"))
            })?;
        (id, sig)
    };
    let print_string = {
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        let id = module
            .declare_function("qb_print_string", Linkage::Import, &sig)
            .map_err(|err| {
                QError::Internal(format!("cranelift declare print_string failed: {err}"))
            })?;
        (id, sig)
    };

    let mut func_ctx = FunctionBuilderContext::new();
    {
        let mut function_builder = FunctionBuilder::new(&mut context.func, &mut func_ctx);
        let entry = function_builder.create_block();
        function_builder.switch_to_block(entry);
        function_builder.append_block_params_for_function_params(entry);
        let runtime_ptr = function_builder.block_params(entry)[0];

        let imported_print_number =
            module.declare_func_in_func(print_number.0, &mut function_builder.func);
        let imported_print_newline =
            module.declare_func_in_func(print_newline.0, &mut function_builder.func);
        let imported_print_string =
            module.declare_func_in_func(print_string.0, &mut function_builder.func);

        let mut variables = HashMap::new();
        let mut next_variable = 0u32;

        for statement in &program.statements {
            compile_statement(
                &mut function_builder,
                &mut variables,
                &mut next_variable,
                runtime_ptr,
                imported_print_number,
                imported_print_newline,
                imported_print_string,
                &support.strings,
                statement,
            )?;
        }

        let zero = function_builder.ins().iconst(types::I32, 0);
        function_builder.ins().return_(&[zero]);
        function_builder.seal_all_blocks();
        function_builder.finalize();
    }

    module
        .define_function(func_id, &mut context)
        .map_err(|err| QError::Internal(format!("cranelift define failed: {err}")))?;

    Ok(CraneliftProgram {
        module,
        func_id,
        strings: support.strings,
    })
}

#[allow(clippy::too_many_arguments)]
fn compile_statement(
    builder: &mut FunctionBuilder<'_>,
    variables: &mut HashMap<String, Variable>,
    next_variable: &mut u32,
    runtime_ptr: cranelift_codegen::ir::Value,
    print_number: cranelift_codegen::ir::FuncRef,
    print_newline: cranelift_codegen::ir::FuncRef,
    print_string: cranelift_codegen::ir::FuncRef,
    strings: &[String],
    statement: &Statement,
) -> QResult<()> {
    match statement {
        Statement::Print {
            expressions,
            separators,
            newline,
        } => {
            if separators.iter().any(|separator| separator.is_some()) {
                return Err(QError::UnsupportedFeature(
                    "experimental cranelift-jit backend does not yet support PRINT separators"
                        .to_string(),
                ));
            }
            for expression in expressions {
                match expression {
                    Expression::Literal(QType::String(value)) => {
                        let index = strings
                            .iter()
                            .position(|candidate| candidate == value)
                            .ok_or_else(|| {
                                QError::Internal(
                                    "missing string literal in cranelift string table".to_string(),
                                )
                            })?;
                        let index_value = builder.ins().iconst(types::I64, index as i64);
                        builder
                            .ins()
                            .call(print_string, &[runtime_ptr, index_value]);
                    }
                    _ => {
                        let value = compile_numeric_expression(
                            builder,
                            variables,
                            next_variable,
                            expression,
                        )?;
                        builder.ins().call(print_number, &[value]);
                    }
                }
            }
            if *newline {
                builder.ins().call(print_newline, &[]);
            }
            Ok(())
        }
        Statement::Assignment { target, value } => {
            let Expression::Variable(variable) = target else {
                return Err(QError::UnsupportedFeature(
                    "experimental cranelift-jit backend supports only simple variable assignments"
                        .to_string(),
                ));
            };
            let compiled_value =
                compile_numeric_expression(builder, variables, next_variable, value)?;
            let variable_ref =
                ensure_variable(builder, variables, next_variable, variable.name.as_str());
            builder.def_var(variable_ref, compiled_value);
            Ok(())
        }
        Statement::End => Ok(()),
        other => Err(QError::UnsupportedFeature(format!(
            "experimental cranelift-jit backend does not yet support {:?}",
            other
        ))),
    }
}

fn ensure_variable(
    builder: &mut FunctionBuilder<'_>,
    variables: &mut HashMap<String, Variable>,
    next_variable: &mut u32,
    name: &str,
) -> Variable {
    if let Some(variable) = variables.get(name) {
        return *variable;
    }

    let variable = builder.declare_var(types::F64);
    *next_variable += 1;
    let zero = builder.ins().f64const(0.0);
    builder.def_var(variable, zero);
    variables.insert(name.to_string(), variable);
    variable
}

fn compile_numeric_expression(
    builder: &mut FunctionBuilder<'_>,
    variables: &mut HashMap<String, Variable>,
    next_variable: &mut u32,
    expression: &Expression,
) -> QResult<cranelift_codegen::ir::Value> {
    match expression {
        Expression::Literal(QType::Integer(value)) => Ok(builder.ins().f64const(*value as f64)),
        Expression::Literal(QType::Long(value)) => Ok(builder.ins().f64const(*value as f64)),
        Expression::Literal(QType::Single(value)) => Ok(builder.ins().f64const(*value as f64)),
        Expression::Literal(QType::Double(value)) => Ok(builder.ins().f64const(*value)),
        Expression::Variable(variable) => {
            let slot = ensure_variable(builder, variables, next_variable, &variable.name);
            Ok(builder.use_var(slot))
        }
        Expression::UnaryOp {
            op: UnaryOp::Negate,
            operand,
        } => {
            let value = compile_numeric_expression(builder, variables, next_variable, operand)?;
            Ok(builder.ins().fneg(value))
        }
        Expression::BinaryOp { op, left, right }
            if matches!(
                op,
                BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
            ) =>
        {
            let left_value = compile_numeric_expression(builder, variables, next_variable, left)?;
            let right_value = compile_numeric_expression(builder, variables, next_variable, right)?;
            Ok(match op {
                BinaryOp::Add => builder.ins().fadd(left_value, right_value),
                BinaryOp::Subtract => builder.ins().fsub(left_value, right_value),
                BinaryOp::Multiply => builder.ins().fmul(left_value, right_value),
                BinaryOp::Divide => builder.ins().fdiv(left_value, right_value),
                _ => unreachable!(),
            })
        }
        other => Err(QError::UnsupportedFeature(format!(
            "experimental cranelift-jit backend does not yet support expression {:?}",
            other
        ))),
    }
}
