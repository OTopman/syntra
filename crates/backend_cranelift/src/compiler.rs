use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Value as ClValue};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use syntra_air::ir::*;
use syntra_frontend::Type;
use syntra_syntax::ast::BinaryOp;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("Cranelift target error: {0}")]
    Target(String),
    #[error("Cranelift module error: {0}")]
    Module(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Linker error: {0}")]
    Linker(String),
}

fn to_cl_type(ty: &Type, ptr_type: types::Type) -> types::Type {
    match ty {
        Type::Int => types::I64,
        Type::Float => types::F64,
        Type::Bool => types::I8,
        Type::String
        | Type::Class(_)
        | Type::Interface(_)
        | Type::Array(_)
        | Type::GenericInstance(_, _)
        | Type::GenericParam(_)
        | Type::Function { .. } => ptr_type,
        Type::Void | Type::Unknown => types::I64,
    }
}

pub fn compile_module_to_object(
    air_module: &AirModule,
    output_obj: &Path,
) -> Result<(), BackendError> {
    compile_module_to_object_target(air_module, output_obj, None)
}

pub fn compile_module_to_object_target(
    air_module: &AirModule,
    output_obj: &Path,
    target: Option<&str>,
) -> Result<(), BackendError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "speed").unwrap();
    flag_builder.set("is_pic", "true").unwrap();

    let isa = if let Some(t_str) = target {
        let triple: target_lexicon::Triple =
            t_str.parse().map_err(|e: target_lexicon::ParseError| {
                BackendError::Target(format!("Invalid target triple '{t_str}': {e}"))
            })?;
        let isa_builder = cranelift_codegen::isa::lookup(triple).map_err(|e| {
            BackendError::Target(format!("Unsupported target ISA for '{t_str}': {e}"))
        })?;
        let flags = settings::Flags::new(flag_builder);
        isa_builder
            .finish(flags)
            .map_err(|e| BackendError::Target(format!("Failed to build ISA: {e}")))?
    } else {
        let isa_builder = cranelift_native::builder()
            .map_err(|e| BackendError::Target(format!("Failed to get host ISA: {e}")))?;
        let flags = settings::Flags::new(flag_builder);
        isa_builder
            .finish(flags)
            .map_err(|e| BackendError::Target(format!("Failed to build ISA: {e}")))?
    };

    let ptr_type = isa.pointer_type();
    let obj_builder = ObjectBuilder::new(
        isa,
        "syntra_native",
        cranelift_module::default_libcall_names(),
    )
    .map_err(|e| BackendError::Module(e.to_string()))?;
    let mut cl_module = ObjectModule::new(obj_builder);

    // 1. Declare C Runtime extern functions
    let mut func_ids: HashMap<String, FuncId> = HashMap::new();

    // syntra_println_str(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_println_str", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_println_str".to_string(), fid);
    func_ids.insert("println".to_string(), fid);

    // syntra_print_str(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_print_str", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_print_str".to_string(), fid);
    func_ids.insert("print".to_string(), fid);

    // syntra_println_int(i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_println_int", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_println_int".to_string(), fid);

    // syntra_concat_str(ptr, ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_concat_str", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_concat_str".to_string(), fid);

    // syntra_int_to_str(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_int_to_str", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_int_to_str".to_string(), fid);

    // syntra_bool_to_str(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_bool_to_str", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_bool_to_str".to_string(), fid);

    // syntra_alloc(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_alloc", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_alloc".to_string(), fid);

    // syntra_free(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_free", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_free".to_string(), fid);

    // syntra_alloc_array(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_alloc_array", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_alloc_array".to_string(), fid);

    // syntra_array_get(ptr, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_get", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_get".to_string(), fid);

    // syntra_array_set(ptr, i64, i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_set", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_set".to_string(), fid);

    // syntra_array_len(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_len", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_len".to_string(), fid);

    // syntra_array_push(ptr, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_push", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_push".to_string(), fid);

    // syntra_math_abs(i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_abs", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_abs".to_string(), fid);
    func_ids.insert("abs".to_string(), fid);

    // syntra_math_min(i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_min", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_min".to_string(), fid);
    func_ids.insert("min".to_string(), fid);

    // syntra_math_max(i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_max", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_max".to_string(), fid);
    func_ids.insert("max".to_string(), fid);

    // syntra_math_clamp(i64, i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_clamp", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_clamp".to_string(), fid);

    // syntra_math_pow(i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_pow", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_pow".to_string(), fid);

    // syntra_math_sqrt(i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_math_sqrt", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_math_sqrt".to_string(), fid);

    // syntra_array_min(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_min", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_min".to_string(), fid);

    // syntra_array_max(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_max", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_max".to_string(), fid);

    // String Runtime Functions
    // syntra_str_len(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_len", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_len".to_string(), fid);

    // syntra_str_is_empty(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_is_empty", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_is_empty".to_string(), fid);

    // syntra_str_contains(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_contains", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_contains".to_string(), fid);

    // syntra_str_index_of(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_index_of", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_index_of".to_string(), fid);

    // syntra_str_starts_with(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_starts_with", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_starts_with".to_string(), fid);

    // syntra_str_ends_with(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_ends_with", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_ends_with".to_string(), fid);

    // syntra_str_to_upper(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_to_upper", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_to_upper".to_string(), fid);

    // syntra_str_to_lower(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_to_lower", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_to_lower".to_string(), fid);

    // syntra_str_trim(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_trim", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_trim".to_string(), fid);

    // syntra_str_slice(ptr, i64, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_slice", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_slice".to_string(), fid);

    // syntra_str_char_at(ptr, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_char_at", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_char_at".to_string(), fid);

    // syntra_str_char_code_at(ptr, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_char_code_at", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_char_code_at".to_string(), fid);

    // syntra_str_replace(ptr, ptr, ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_replace", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_replace".to_string(), fid);

    // syntra_str_split(ptr, ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_str_split", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_split".to_string(), fid);

    // syntra_str_to_int(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_to_int", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_to_int".to_string(), fid);

    // syntra_array_join(ptr, ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_array_join", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_join".to_string(), fid);

    // syntra_str_eq(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_str_eq", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_str_eq".to_string(), fid);

    // File Runtime Functions
    // syntra_file_exists(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_file_exists", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_exists".to_string(), fid);

    // syntra_file_read_text(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_file_read_text", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_read_text".to_string(), fid);

    // syntra_file_write_text(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_file_write_text", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_write_text".to_string(), fid);

    // syntra_file_append_text(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_file_append_text", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_append_text".to_string(), fid);

    // syntra_file_delete(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_file_delete", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_delete".to_string(), fid);

    // syntra_file_size(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_file_size", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_size".to_string(), fid);

    // syntra_file_read_lines(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_file_read_lines", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_file_read_lines".to_string(), fid);

    // Class File bindings
    // File_new(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("File_new", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_new".to_string(), fid);

    // File_readText(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("File_readText", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_readText".to_string(), fid);

    // File_writeText(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("File_writeText", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_writeText".to_string(), fid);

    // File_appendText(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("File_appendText", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_appendText".to_string(), fid);

    // File_exists(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("File_exists", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_exists".to_string(), fid);

    // File_delete(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("File_delete", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_delete".to_string(), fid);

    // File_size(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("File_size", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_size".to_string(), fid);

    // File_getPath(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("File_getPath", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_getPath".to_string(), fid);

    // File_readLines(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("File_readLines", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("File_readLines".to_string(), fid);

    // syntra_system_args() -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_system_args", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_system_args".to_string(), fid);

    // syntra_system_exit(i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_system_exit", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_system_exit".to_string(), fid);

    // syntra_system_exec(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_system_exec", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_system_exec".to_string(), fid);

    // syntra_system_getenv(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_system_getenv", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_system_getenv".to_string(), fid);

    // syntra_system_time_ms() -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_system_time_ms", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_system_time_ms".to_string(), fid);

    // syntra_string_from_char_code(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_string_from_char_code", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_string_from_char_code".to_string(), fid);

    // syntra_set_error(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_set_error", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_set_error".to_string(), fid);

    // syntra_get_error() -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_get_error", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_get_error".to_string(), fid);

    // syntra_has_error() -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_has_error", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_has_error".to_string(), fid);

    // syntra_clear_error() -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    let fid = cl_module
        .declare_function("syntra_clear_error", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_clear_error".to_string(), fid);

    // syntra_crypto_sha256(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_crypto_sha256", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_crypto_sha256".to_string(), fid);

    // syntra_array_set_len(ptr, i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_array_set_len", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_array_set_len".to_string(), fid);

    // syntra_map_new() -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_map_new", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_new".to_string(), fid);

    // syntra_map_set(ptr, ptr, i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_map_set", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_set".to_string(), fid);

    // syntra_map_get(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_map_get", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_get".to_string(), fid);

    // syntra_map_has(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_map_has", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_has".to_string(), fid);

    // syntra_map_remove(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_map_remove", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_remove".to_string(), fid);

    // syntra_map_len(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_map_len", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_len".to_string(), fid);

    // syntra_map_keys(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_map_keys", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_keys".to_string(), fid);

    // syntra_map_values(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_map_values", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_values".to_string(), fid);

    // syntra_map_clear(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_map_clear", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_map_clear".to_string(), fid);

    // syntra_json_parse(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_json_parse", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_json_parse".to_string(), fid);

    // syntra_json_stringify(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_json_stringify", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_json_stringify".to_string(), fid);

    // syntra_schema_register(ptr, ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_schema_register", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_schema_register".to_string(), fid);

    // syntra_schema_describe(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_schema_describe", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_schema_describe".to_string(), fid);

    // syntra_invoke_fn0(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_invoke_fn0", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_invoke_fn0".to_string(), fid);

    // syntra_invoke_fn1(ptr, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_invoke_fn1", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_invoke_fn1".to_string(), fid);

    // syntra_invoke_fn2(ptr, i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_invoke_fn2", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_invoke_fn2".to_string(), fid);

    // syntra_task_spawn(ptr, ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_task_spawn", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_task_spawn".to_string(), fid);

    // syntra_task_spawn_n(ptr, i64, i64, i64, i64, i64, i64, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    for _ in 0..6 {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_task_spawn_n", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_task_spawn_n".to_string(), fid);

    // syntra_task_join(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_task_join", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_task_join".to_string(), fid);

    // syntra_task_sleep(i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_task_sleep", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_task_sleep".to_string(), fid);

    // syntra_channel_new(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_channel_new", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_new".to_string(), fid);

    // syntra_channel_send(ptr, i64) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_send", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_send".to_string(), fid);

    // syntra_channel_recv(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_recv", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_recv".to_string(), fid);

    // syntra_channel_close(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_channel_close", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_close".to_string(), fid);

    // syntra_channel_len(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_len", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_len".to_string(), fid);

    // syntra_task_is_done(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_task_is_done", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_task_is_done".to_string(), fid);

    // syntra_threadpool_new(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_threadpool_new", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_threadpool_new".to_string(), fid);

    // syntra_threadpool_submit(ptr, ptr, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_threadpool_submit", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_threadpool_submit".to_string(), fid);

    // syntra_arena_create(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_arena_create", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_arena_create".to_string(), fid);

    // syntra_arena_alloc(ptr, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_arena_alloc", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_arena_alloc".to_string(), fid);

    // syntra_arena_reset(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_arena_reset", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_arena_reset".to_string(), fid);

    // syntra_arena_destroy(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_arena_destroy", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_arena_destroy".to_string(), fid);

    // syntra_arena_bytes_allocated(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_arena_bytes_allocated", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_arena_bytes_allocated".to_string(), fid);

    // syntra_threadpool_shutdown(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_threadpool_shutdown", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_threadpool_shutdown".to_string(), fid);

    // syntra_channel_try_recv(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_try_recv", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_try_recv".to_string(), fid);

    // syntra_channel_try_send(ptr, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_try_send", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_try_send".to_string(), fid);

    // syntra_channel_select(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_channel_select", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_channel_select".to_string(), fid);

    // syntra_tcp_listen(i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_listen", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_listen".to_string(), fid);

    // syntra_tcp_accept(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_accept", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_accept".to_string(), fid);

    // syntra_tcp_read(ptr, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_read", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_read".to_string(), fid);

    // syntra_tcp_write(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_tcp_write", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_write".to_string(), fid);

    // syntra_tcp_listener_port(ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_tcp_listener_port", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_listener_port".to_string(), fid);

    // syntra_tcp_close(ptr) -> void
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_close", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_close".to_string(), fid);

    // syntra_ws_compute_accept(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_ws_compute_accept", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_ws_compute_accept".to_string(), fid);

    // syntra_tcp_read_ws_frame(ptr) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_read_ws_frame", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_read_ws_frame".to_string(), fid);

    // syntra_tcp_write_ws_frame(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_tcp_write_ws_frame", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_write_ws_frame".to_string(), fid);

    // syntra_tcp_write_ws_client_frame(ptr, ptr) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_tcp_write_ws_client_frame", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_write_ws_client_frame".to_string(), fid);

    // syntra_http_serve_reactor(i64, i64) -> i64
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(types::I64));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = cl_module
        .declare_function("syntra_http_serve_reactor", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_http_serve_reactor".to_string(), fid);

    // syntra_tcp_connect(ptr, i64) -> ptr
    let mut sig = cl_module.make_signature();
    sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(ptr_type));
    let fid = cl_module
        .declare_function("syntra_tcp_connect", Linkage::Import, &sig)
        .map_err(|e| BackendError::Module(e.to_string()))?;
    func_ids.insert("syntra_tcp_connect".to_string(), fid);

    // 2. Define static string literals in rodata
    let mut string_data_ids: HashMap<String, DataId> = HashMap::new();
    let mut str_counter = 0;
    for func in &air_module.functions {
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Op::ConstString(s) = &inst.op {
                    if !string_data_ids.contains_key(s) {
                        let mut desc = DataDescription::new();
                        let mut bytes = s.as_bytes().to_vec();
                        bytes.push(0); // Null terminator
                        desc.define(bytes.into_boxed_slice());
                        let sym_name = format!("__ae_str_{str_counter}");
                        str_counter += 1;
                        let did = cl_module
                            .declare_data(&sym_name, Linkage::Local, false, false)
                            .map_err(|e| BackendError::Module(e.to_string()))?;
                        cl_module
                            .define_data(did, &desc)
                            .map_err(|e| BackendError::Module(e.to_string()))?;
                        string_data_ids.insert(s.clone(), did);
                    }
                }
            }
        }
    }

    // 3. Forward declare all user-defined functions
    for func in &air_module.functions {
        let mut sig = cl_module.make_signature();
        sig.call_conv = CallConv::triple_default(cl_module.isa().triple());

        for param in &func.params {
            sig.params
                .push(AbiParam::new(to_cl_type(&param.ty, ptr_type)));
        }

        if func.return_type != Type::Void {
            sig.returns
                .push(AbiParam::new(to_cl_type(&func.return_type, ptr_type)));
        } else if func.name == "main" {
            sig.returns.push(AbiParam::new(types::I32));
        }

        let cl_func_name = if func.name == "main" {
            "main".to_string()
        } else {
            format!("syn_fn_{}", func.name)
        };

        let fid = cl_module
            .declare_function(&cl_func_name, Linkage::Export, &sig)
            .map_err(|e| BackendError::Module(e.to_string()))?;
        func_ids.insert(func.name.clone(), fid);
    }

    // 3.5 Forward-declare any external C functions or intrinsics called but not yet declared
    for func in &air_module.functions {
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Op::Call { func_name, args } = &inst.op {
                    if !func_ids.contains_key(func_name) {
                        let mut sig = cl_module.make_signature();
                        sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
                        for _ in args {
                            sig.params.push(AbiParam::new(ptr_type));
                        }
                        if inst.ty != Type::Void {
                            sig.returns
                                .push(AbiParam::new(to_cl_type(&inst.ty, ptr_type)));
                        }
                        if let Ok(fid) =
                            cl_module.declare_function(func_name, Linkage::Import, &sig)
                        {
                            func_ids.insert(func_name.clone(), fid);
                        }
                    }
                }
            }
        }
    }

    let mut ctx = cl_module.make_context();
    let mut func_ctx = FunctionBuilderContext::new();

    // 4. Compile each function
    for func in &air_module.functions {
        let fid = func_ids[&func.name];

        let mut sig = cl_module.make_signature();
        sig.call_conv = CallConv::triple_default(cl_module.isa().triple());
        for param in &func.params {
            sig.params
                .push(AbiParam::new(to_cl_type(&param.ty, ptr_type)));
        }
        if func.return_type != Type::Void {
            sig.returns
                .push(AbiParam::new(to_cl_type(&func.return_type, ptr_type)));
        } else if func.name == "main" {
            sig.returns.push(AbiParam::new(types::I32));
        }

        ctx.func.signature = sig;
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut func_ctx);

        let mut block_map = HashMap::new();
        for b in &func.blocks {
            let cl_block = builder.create_block();
            block_map.insert(b.id, cl_block);
        }

        // Setup entry block
        let entry_cl_block = block_map[&func.entry_block];
        builder.append_block_params_for_function_params(entry_cl_block);
        builder.switch_to_block(entry_cl_block);

        let mut value_map: HashMap<ValueId, ClValue> = HashMap::new();

        // Map function parameters
        for (i, param) in func.params.iter().enumerate() {
            let cl_val = builder.block_params(entry_cl_block)[i];
            value_map.insert(param.id, cl_val);
        }

        // Emit blocks
        for block in &func.blocks {
            let cl_block = block_map[&block.id];
            builder.switch_to_block(cl_block);

            // Instructions
            for inst in &block.instructions {
                let cl_val = match &inst.op {
                    Op::ConstInt(i) => builder.ins().iconst(types::I64, *i),
                    Op::ConstFloat(f) => builder.ins().f64const(*f),
                    Op::ConstBool(b) => builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
                    Op::ConstString(s) => {
                        let did = string_data_ids[s];
                        let local_data = cl_module.declare_data_in_func(did, builder.func);
                        builder.ins().symbol_value(ptr_type, local_data)
                    }
                    Op::Binary { op, left, right } => {
                        let mut l = value_map[left];
                        let mut r = value_map[right];
                        let l_ty = builder.func.dfg.value_type(l);
                        let r_ty = builder.func.dfg.value_type(r);

                        let is_float = l_ty == types::F64 || r_ty == types::F64;
                        if is_float {
                            if l_ty != types::F64 {
                                l = builder.ins().fcvt_from_sint(types::F64, l);
                            }
                            if r_ty != types::F64 {
                                r = builder.ins().fcvt_from_sint(types::F64, r);
                            }
                            match op {
                                BinaryOp::Add => builder.ins().fadd(l, r),
                                BinaryOp::Sub => builder.ins().fsub(l, r),
                                BinaryOp::Mul => builder.ins().fmul(l, r),
                                BinaryOp::Div => builder.ins().fdiv(l, r),
                                BinaryOp::EqualEqual => builder.ins().fcmp(FloatCC::Equal, l, r),
                                BinaryOp::NotEqual => builder.ins().fcmp(FloatCC::NotEqual, l, r),
                                BinaryOp::Less => builder.ins().fcmp(FloatCC::LessThan, l, r),
                                BinaryOp::LessEqual => {
                                    builder.ins().fcmp(FloatCC::LessThanOrEqual, l, r)
                                }
                                BinaryOp::Greater => builder.ins().fcmp(FloatCC::GreaterThan, l, r),
                                BinaryOp::GreaterEqual => {
                                    builder.ins().fcmp(FloatCC::GreaterThanOrEqual, l, r)
                                }
                                _ => builder.ins().iconst(types::I64, 0),
                            }
                        } else {
                            if l_ty != r_ty {
                                if l_ty == types::I8 && r_ty == types::I64 {
                                    l = builder.ins().uextend(types::I64, l);
                                } else if l_ty == types::I64 && r_ty == types::I8 {
                                    r = builder.ins().uextend(types::I64, r);
                                }
                            }
                            match op {
                                BinaryOp::Add => builder.ins().iadd(l, r),
                                BinaryOp::Sub => builder.ins().isub(l, r),
                                BinaryOp::Mul => builder.ins().imul(l, r),
                                BinaryOp::Div => builder.ins().sdiv(l, r),
                                BinaryOp::EqualEqual => builder.ins().icmp(IntCC::Equal, l, r),
                                BinaryOp::NotEqual => builder.ins().icmp(IntCC::NotEqual, l, r),
                                BinaryOp::Less => builder.ins().icmp(IntCC::SignedLessThan, l, r),
                                BinaryOp::LessEqual => {
                                    builder.ins().icmp(IntCC::SignedLessThanOrEqual, l, r)
                                }
                                BinaryOp::Greater => {
                                    builder.ins().icmp(IntCC::SignedGreaterThan, l, r)
                                }
                                BinaryOp::GreaterEqual => {
                                    builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r)
                                }
                                BinaryOp::And => builder.ins().band(l, r),
                                BinaryOp::Or => builder.ins().bor(l, r),
                                BinaryOp::Mod => builder.ins().srem(l, r),
                                BinaryOp::BitwiseAnd => builder.ins().band(l, r),
                                BinaryOp::BitwiseOr => builder.ins().bor(l, r),
                                BinaryOp::BitwiseXor => builder.ins().bxor(l, r),
                                BinaryOp::Shl => builder.ins().ishl(l, r),
                                BinaryOp::Shr => builder.ins().ushr(l, r),
                            }
                        }
                    }
                    Op::Call { func_name, args } => {
                        if let Some(callee_id) = func_ids.get(func_name) {
                            let callee_ref =
                                cl_module.declare_func_in_func(*callee_id, builder.func);
                            let sig_ref = builder.func.dfg.ext_funcs[callee_ref].signature;
                            let arg_vals: Vec<ClValue> = args
                                .iter()
                                .enumerate()
                                .map(|(i, a)| {
                                    let val = value_map[a];
                                    let val_ty = builder.func.dfg.value_type(val);
                                    let expected_ty = builder.func.dfg.signatures[sig_ref]
                                        .params
                                        .get(i)
                                        .map(|p| p.value_type)
                                        .unwrap_or(val_ty);
                                    if val_ty == types::I8 && expected_ty == types::I64 {
                                        builder.ins().uextend(types::I64, val)
                                    } else if val_ty == types::I64 && expected_ty == types::I8 {
                                        builder.ins().ireduce(types::I8, val)
                                    } else {
                                        val
                                    }
                                })
                                .collect();
                            let call_inst = builder.ins().call(callee_ref, &arg_vals);
                            let results = builder.inst_results(call_inst);
                            if results.is_empty() {
                                builder.ins().iconst(types::I64, 0)
                            } else {
                                let mut res = results[0];
                                let res_ty = builder.func.dfg.value_type(res);
                                let expected_ret_ty = to_cl_type(&inst.ty, ptr_type);
                                if res_ty == types::I64 && expected_ret_ty == types::I8 {
                                    res = builder.ins().ireduce(types::I8, res);
                                } else if res_ty == types::I8 && expected_ret_ty == types::I64 {
                                    res = builder.ins().uextend(types::I64, res);
                                }
                                res
                            }
                        } else {
                            builder.ins().iconst(types::I64, 0)
                        }
                    }
                    Op::NewObject { class_name, .. } => {
                        let alloc_fid = func_ids["syntra_alloc"];
                        let callee_ref = cl_module.declare_func_in_func(alloc_fid, builder.func);
                        let alloc_size = air_module
                            .class_sizes
                            .get(class_name)
                            .copied()
                            .unwrap_or(64) as i64;
                        let size_val = builder.ins().iconst(types::I64, alloc_size.max(64));
                        let call_inst = builder.ins().call(callee_ref, &[size_val]);
                        builder.inst_results(call_inst)[0]
                    }
                    Op::FuncAddr { func_name } => {
                        if let Some(&callee_id) = func_ids.get(func_name) {
                            let callee_ref =
                                cl_module.declare_func_in_func(callee_id, builder.func);
                            builder.ins().func_addr(ptr_type, callee_ref)
                        } else {
                            builder.ins().iconst(ptr_type, 0)
                        }
                    }
                    Op::Alloc { domain, ty } => {
                        let class_name = match &inst.ty {
                            Type::Class(name) | Type::GenericInstance(name, _) => {
                                Some(name.as_str())
                            }
                            _ => match ty {
                                Type::Class(name) | Type::GenericInstance(name, _) => {
                                    Some(name.as_str())
                                }
                                _ => None,
                            },
                        };
                        let alloc_size = class_name
                            .and_then(|n| air_module.class_sizes.get(n))
                            .copied()
                            .unwrap_or(8) as i64;

                        match domain {
                            MemoryDomain::Stack => {
                                let slot = builder.create_sized_stack_slot(
                                    cranelift_codegen::ir::StackSlotData::new(
                                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                        alloc_size.max(8) as u32,
                                        3, // 2^3 = 8-byte alignment
                                    ),
                                );
                                builder.ins().stack_addr(ptr_type, slot, 0)
                            }
                            _ => {
                                let alloc_fid = func_ids["syntra_alloc"];
                                let callee_ref =
                                    cl_module.declare_func_in_func(alloc_fid, builder.func);
                                let size_val = builder.ins().iconst(types::I64, alloc_size.max(64));
                                let call_inst = builder.ins().call(callee_ref, &[size_val]);
                                builder.inst_results(call_inst)[0]
                            }
                        }
                    }
                    Op::Load { ptr } => {
                        let p = value_map[ptr];
                        let cl_ty = to_cl_type(&inst.ty, ptr_type);
                        builder
                            .ins()
                            .load(cl_ty, cranelift_codegen::ir::MemFlags::trusted(), p, 0)
                    }
                    Op::Store { ptr, val } => {
                        let p = value_map[ptr];
                        let v = value_map[val];
                        builder
                            .ins()
                            .store(cranelift_codegen::ir::MemFlags::trusted(), v, p, 0);
                        builder.ins().iconst(types::I64, 0)
                    }
                    Op::GetField { object, offset, .. } => {
                        let p = value_map[object];
                        let cl_ty = to_cl_type(&inst.ty, ptr_type);
                        builder.ins().load(
                            cl_ty,
                            cranelift_codegen::ir::MemFlags::trusted(),
                            p,
                            *offset as i32,
                        )
                    }
                    Op::SetField {
                        object,
                        offset,
                        val,
                        ..
                    } => {
                        let p = value_map[object];
                        let v = value_map[val];
                        builder.ins().store(
                            cranelift_codegen::ir::MemFlags::trusted(),
                            v,
                            p,
                            *offset as i32,
                        );
                        builder.ins().iconst(types::I64, 0)
                    }
                    Op::AllocArray { len, .. } => {
                        let alloc_fid = func_ids["syntra_alloc_array"];
                        let callee_ref = cl_module.declare_func_in_func(alloc_fid, builder.func);
                        let len_val = value_map[len];
                        let call_inst = builder.ins().call(callee_ref, &[len_val]);
                        builder.inst_results(call_inst)[0]
                    }
                    Op::GetIndex { array, index } => {
                        let get_fid = func_ids["syntra_array_get"];
                        let callee_ref = cl_module.declare_func_in_func(get_fid, builder.func);
                        let arr_val = value_map[array];
                        let idx_val = value_map[index];
                        let call_inst = builder.ins().call(callee_ref, &[arr_val, idx_val]);
                        builder.inst_results(call_inst)[0]
                    }
                    Op::SetIndex { array, index, val } => {
                        let set_fid = func_ids["syntra_array_set"];
                        let callee_ref = cl_module.declare_func_in_func(set_fid, builder.func);
                        let arr_val = value_map[array];
                        let idx_val = value_map[index];
                        let v_val = value_map[val];
                        builder.ins().call(callee_ref, &[arr_val, idx_val, v_val]);
                        builder.ins().iconst(types::I64, 0)
                    }
                    Op::ArrayLength { array } => {
                        let len_fid = func_ids["syntra_array_len"];
                        let callee_ref = cl_module.declare_func_in_func(len_fid, builder.func);
                        let arr_val = value_map[array];
                        let call_inst = builder.ins().call(callee_ref, &[arr_val]);
                        builder.inst_results(call_inst)[0]
                    }
                    Op::Guard {
                        cond,
                        deopt_target,
                        deopt_args: _,
                        reason: _,
                    } => {
                        let c_val = value_map[cond];
                        let target_block = block_map[deopt_target];
                        let cont_block = builder.create_block();

                        // If cond is true, continue fast-path; if false, transfer execution to deopt fallback
                        builder
                            .ins()
                            .brif(c_val, cont_block, &[], target_block, &[]);
                        builder.seal_block(cont_block);
                        builder.switch_to_block(cont_block);
                        builder.ins().iconst(types::I64, 0)
                    }
                };

                value_map.insert(inst.result, cl_val);
            }

            // Terminator
            match &block.terminator {
                Terminator::Return(val, _) => {
                    if let Some(v) = val {
                        let ret_val = value_map[v];
                        builder.ins().return_(&[ret_val]);
                    } else if func.name == "main" {
                        let zero = builder.ins().iconst(types::I32, 0);
                        builder.ins().return_(&[zero]);
                    } else if func.return_type != Type::Void {
                        let zero = builder.ins().iconst(types::I64, 0);
                        builder.ins().return_(&[zero]);
                    } else {
                        builder.ins().return_(&[]);
                    }
                }
                Terminator::Branch(target, _args, _) => {
                    let target_block = block_map[target];
                    builder.ins().jump(target_block, &[]);
                }
                Terminator::CondBranch {
                    cond,
                    then_block,
                    else_block,
                    ..
                } => {
                    let cond_val = value_map[cond];
                    let tb = block_map[then_block];
                    let eb = block_map[else_block];
                    builder.ins().brif(cond_val, tb, &[], eb, &[]);
                }
            }
        }

        builder.seal_all_blocks();
        builder.finalize();

        cl_module.define_function(fid, &mut ctx).map_err(|e| {
            BackendError::Module(format!(
                "Function '{}' compilation error: {}\nFunc IR:\n{}",
                func.name,
                e,
                ctx.func.display()
            ))
        })?;
        cl_module.clear_context(&mut ctx);
    }

    let product = cl_module.finish();
    let obj_bytes = product
        .emit()
        .map_err(|e| BackendError::Module(e.to_string()))?;

    fs::write(output_obj, obj_bytes)?;
    Ok(())
}

/// Links an object file into a standalone executable using the system C compiler (`cc` or `clang`).
pub fn link_executable(obj_path: &Path, output_binary: &Path) -> Result<(), BackendError> {
    link_executable_target(obj_path, output_binary, None)
}

pub fn link_executable_target(
    obj_path: &Path,
    output_binary: &Path,
    target: Option<&str>,
) -> Result<(), BackendError> {
    const RT_SOURCE: &str = include_str!("../../runtime/native/syntra_rt.c");
    let rt_c_path = obj_path.with_file_name("syntra_rt_tmp.c");
    fs::write(&rt_c_path, RT_SOURCE)?;

    let mut cmd = Command::new("cc");
    cmd.arg(obj_path)
        .arg(&rt_c_path)
        .arg("-O3")
        .arg("-o")
        .arg(output_binary);

    if let Some(t) = target {
        cmd.arg(format!("--target={t}"));
    }

    #[cfg(target_os = "macos")]
    {
        cmd.arg("-dead_strip");
    }
    #[cfg(target_os = "linux")]
    {
        cmd.arg("-Wl,--gc-sections");
    }

    let output = cmd
        .output()
        .map_err(|e| BackendError::Linker(format!("Failed to execute 'cc': {e}")))?;

    let _ = fs::remove_file(&rt_c_path);

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(BackendError::Linker(format!("Linker failed:\n{err_msg}")));
    }

    Ok(())
}
