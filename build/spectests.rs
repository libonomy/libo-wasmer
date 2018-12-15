//! This file will run at build time to autogenerate Rust tests based on
//! WebAssembly spec tests. It will convert the files indicated in TESTS
//! from "/spectests/{MODULE}.wast" to "/src/spectests/{MODULE}.rs".
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use wabt::script::{Action, Command, CommandKind, ModuleBinary, ScriptParser, Value};
use wabt::wasm2wat;

static BANNER: &str = "// Rust test file autogenerated with cargo build (build/spectests.rs).
// Please do NOT modify it by hand, as it will be reseted on next build.\n";

const TESTS: [&str; 60] = [
    "spectests/address.wast",
    "spectests/align.wast",
    "spectests/binary.wast",
    "spectests/block.wast",
    "spectests/br.wast",
    "spectests/br_if.wast",
    "spectests/br_table.wast",
    "spectests/break_drop.wast",
    "spectests/call.wast",
    "spectests/call_indirect.wast",
    "spectests/comments.wast",
    "spectests/const_.wast",
    "spectests/conversions.wast",
    "spectests/custom.wast",
    "spectests/data.wast",
    "spectests/elem.wast",
    "spectests/endianness.wast",
    "spectests/exports.wast",
    "spectests/f32_.wast",
    "spectests/f32_bitwise.wast",
    "spectests/f32_cmp.wast",
    "spectests/f64_.wast",
    "spectests/f64_bitwise.wast",
    "spectests/f64_cmp.wast",
    "spectests/fac.wast",
    "spectests/float_exprs.wast",
    "spectests/float_literals.wast",
    "spectests/float_memory.wast",
    "spectests/float_misc.wast",
    "spectests/forward.wast",
    "spectests/func.wast",
    "spectests/func_ptrs.wast",
    "spectests/get_local.wast",
    "spectests/globals.wast",
    "spectests/i32_.wast",
    "spectests/i64_.wast",
    "spectests/if_.wast",
    "spectests/int_exprs.wast",
    "spectests/int_literals.wast",
    "spectests/labels.wast",
    "spectests/left_to_right.wast",
    "spectests/loop_.wast",
    "spectests/memory.wast",
    "spectests/memory_grow.wast",
    "spectests/memory_redundancy.wast",
    "spectests/memory_trap.wast",
    "spectests/nop.wast",
    "spectests/return_.wast",
    "spectests/select.wast",
    "spectests/set_local.wast",
    "spectests/stack.wast",
    "spectests/start.wast",
    "spectests/store_retval.wast",
    "spectests/switch.wast",
    "spectests/tee_local.wast",
    "spectests/token.wast",
    "spectests/traps.wast",
    "spectests/typecheck.wast",
    "spectests/types.wast",
    "spectests/unwind.wast",
];

fn wabt2rust_type(v: &Value) -> String {
    match v {
        Value::I32(_v) => format!("i32"),
        Value::I64(_v) => format!("i64"),
        Value::F32(_v) => format!("f32"),
        Value::F64(_v) => format!("f64"),
    }
}

fn is_nan(v: &Value) -> bool {
    if let Value::F32(v) = v {
        return v.is_nan();
    } else if let Value::F64(v) = v {
        return v.is_nan();
    }
    return false;
}

fn wabt2rust_value(v: &Value) -> String {
    match v {
        Value::I32(v) => format!("{:?} as i32", v),
        Value::I64(v) => format!("{:?} as i64", v),
        Value::F32(v) => {
            if v.is_infinite() {
                if v.is_sign_negative() {
                    "f32::NEG_INFINITY".to_string()
                } else {
                    "f32::INFINITY".to_string()
                }
            } else if v.is_nan() {
                // Support for non-canonical NaNs
                format!("f32::from_bits({:?})", v.to_bits())
            } else {
                format!("{:?} as f32", v)
            }
        }
        Value::F64(v) => {
            if v.is_infinite() {
                if v.is_sign_negative() {
                    "f64::NEG_INFINITY".to_string()
                } else {
                    "f64::INFINITY".to_string()
                }
            } else if v.is_nan() {
                format!("f64::from_bits({:?})", v.to_bits())
            } else {
                format!("{:?} as f64", v)
            }
        }
    }
}

struct WastTestGenerator {
    last_module: i32,
    last_line: u64,
    command_no: i32,
    filename: String,
    script_parser: ScriptParser,
    module_calls: HashMap<i32, Vec<String>>,
    buffer: String,
}

impl WastTestGenerator {
    fn new(path: &PathBuf) -> Self {
        let filename = path.file_name().unwrap().to_str().unwrap();
        let source = fs::read(&path).unwrap();
        let script: ScriptParser = ScriptParser::from_source_and_name(&source, filename).unwrap();
        let buffer = String::new();
        WastTestGenerator {
            last_module: 0,
            last_line: 0,
            command_no: 0,
            filename: filename.to_string(),
            script_parser: script,
            buffer: buffer,
            module_calls: HashMap::new(),
        }
    }

    fn consume(&mut self) {
        self.buffer.push_str(BANNER);
        self.buffer.push_str(&format!(
            "// Test based on spectests/{}
#![allow(
    warnings,
    dead_code
)]
use wabt::wat2wasm;

use crate::webassembly::{{instantiate, compile, ImportObject, ResultObject, Instance, Export}};
use super::_common::{{
    spectest_importobject,
    NaNCheck,
}};\n\n",
            self.filename
        ));
        while let Some(Command { line, kind }) = &self.script_parser.next().unwrap() {
            self.last_line = line.clone();
            self.buffer
                .push_str(&format!("\n// Line {}\n", self.last_line));
            self.visit_command(&kind);
            self.command_no = self.command_no + 1;
        }
        for n in 1..self.last_module + 1 {
            self.flush_module_calls(n);
        }
    }

    fn command_name(&self) -> String {
        format!("c{}_l{}", self.command_no, self.last_line)
    }

    fn flush_module_calls(&mut self, module: i32) {
        let calls: Vec<String> = self
            .module_calls
            .entry(module)
            .or_insert(Vec::new())
            .iter()
            .map(|call_str| format!("{}(&result_object);", call_str))
            .collect();
        if calls.len() > 0 {
            self.buffer.push_str(
                format!(
                    "\n#[test]
fn test_module_{}() {{
    let result_object = create_module_{}();
    // We group the calls together
    {}
}}\n",
                    module,
                    module,
                    calls.join("\n    ")
                )
                .as_str(),
            );
        }
        self.module_calls.remove(&module);
    }

    fn visit_module(&mut self, module: &ModuleBinary, _name: &Option<String>) {
        let wasm_binary: Vec<u8> = module.clone().into_vec();
        let wast_string = wasm2wat(wasm_binary).expect("Can't convert back to wasm");
        let last_module = self.last_module;
        self.flush_module_calls(last_module);
        self.last_module = self.last_module + 1;
        // self.module_calls.insert(self.last_module, vec![]);
        self.buffer.push_str(
            format!(
                "fn create_module_{}() -> ResultObject {{
    let module_str = \"{}\";
    let wasm_binary = wat2wasm(module_str.as_bytes()).expect(\"WAST not valid or malformed\");
    instantiate(wasm_binary, spectest_importobject(), None).expect(\"WASM can't be instantiated\")
}}\n",
                self.last_module,
                // We do this to ident four spaces, so it looks aligned to the function body
                wast_string
                    .replace("\n", "\n    ")
                    .replace("\\", "\\\\")
                    .replace("\"", "\\\""),
            )
            .as_str(),
        );

        // We set the start call to the module
        let start_module_call = format!("start_module_{}", self.last_module);
        self.buffer.push_str(
            format!(
                "\nfn {}(result_object: &ResultObject) {{
    result_object.instance.start();
}}\n",
                start_module_call
            )
            .as_str(),
        );
        self.module_calls
            .entry(self.last_module)
            .or_insert(Vec::new())
            .push(start_module_call);
    }

    fn visit_assert_invalid(&mut self, module: &ModuleBinary) {
        let wasm_binary: Vec<u8> = module.clone().into_vec();
        // let wast_string = wasm2wat(wasm_binary).expect("Can't convert back to wasm");
        let command_name = self.command_name();
        self.buffer.push_str(
            format!(
                "#[test]
fn {}_assert_invalid() {{
    let wasm_binary = {:?};
    let compilation = compile(wasm_binary.to_vec());
    assert!(compilation.is_err(), \"WASM should not compile as is invalid\");
}}\n",
                command_name,
                wasm_binary,
                // We do this to ident four spaces back
                // String::from_utf8_lossy(&wasm_binary),
                // wast_string.replace("\n", "\n    "),
            )
            .as_str(),
        );
    }

    // TODO: Refactor repetitive code
    fn visit_assert_return_arithmetic_nan(&mut self, action: &Action) {
        match action {
            Action::Invoke {
                module: _,
                field,
                args,
            } => {
                let return_type = wabt2rust_type(&args[0]);
                let func_return = format!(" -> {}", return_type);
                let assertion = String::from("assert!(result.is_quiet_nan())");

                // We map the arguments provided into the raw Arguments provided
                // to libffi
                let mut args_types: Vec<String> = args.iter().map(wabt2rust_type).collect();
                args_types.push("&Instance".to_string());
                let mut args_values: Vec<String> = args.iter().map(wabt2rust_value).collect();
                args_values.push("&result_object.instance".to_string());
                let func_name = format!("{}_assert_return_arithmetic_nan", self.command_name());
                self.buffer.push_str(
                    format!(
                        "fn {}(result_object: &ResultObject) {{
    println!(\"Executing function {{}}\", \"{}\");
    let func_index = match result_object.module.info.exports.get({:?}) {{
        Some(&Export::Function(index)) => index,
        _ => panic!(\"Function not found\"),
    }};
    let invoke_fn: fn({}){} = get_instance_function!(result_object.instance, func_index);
    let result = invoke_fn({});
    {}
}}\n",
                        func_name,
                        func_name,
                        field,
                        args_types.join(", "),
                        func_return,
                        args_values.join(", "),
                        assertion,
                    )
                    .as_str(),
                );
                self.module_calls
                    .entry(self.last_module)
                    .or_insert(Vec::new())
                    .push(func_name);
                // let mut module_calls = self.module_calls.get(&self.last_module).unwrap();
                // module_calls.push(func_name);
            }
            _ => {}
        };
    }

    // PROBLEM: Im assuming the return type from the first argument type
    // and wabt does gives us the `expected` result
    // TODO: Refactor repetitive code
    fn visit_assert_return_canonical_nan(&mut self, action: &Action) {
        match action {
            Action::Invoke {
                module: _,
                field,
                args,
            } => {
                let return_type = match &field.as_str() {
                    &"f64.promote_f32" => String::from("f64"),
                    &"f32.promote_f64" => String::from("f32"),
                    _ => wabt2rust_type(&args[0]),
                };
                let func_return = format!(" -> {}", return_type);
                let assertion = String::from("assert!(result.is_quiet_nan())");

                // We map the arguments provided into the raw Arguments provided
                // to libffi
                let mut args_types: Vec<String> = args.iter().map(wabt2rust_type).collect();
                args_types.push("&Instance".to_string());
                let mut args_values: Vec<String> = args.iter().map(wabt2rust_value).collect();
                args_values.push("&result_object.instance".to_string());
                let func_name = format!("{}_assert_return_canonical_nan", self.command_name());
                self.buffer.push_str(
                    format!(
                        "fn {}(result_object: &ResultObject) {{
    println!(\"Executing function {{}}\", \"{}\");
    let func_index = match result_object.module.info.exports.get({:?}) {{
        Some(&Export::Function(index)) => index,
        _ => panic!(\"Function not found\"),
    }};
    let invoke_fn: fn({}){} = get_instance_function!(result_object.instance, func_index);
    let result = invoke_fn({});
    {}
}}\n",
                        func_name,
                        func_name,
                        field,
                        args_types.join(", "),
                        func_return,
                        args_values.join(", "),
                        assertion,
                    )
                    .as_str(),
                );
                self.module_calls
                    .entry(self.last_module)
                    .or_insert(Vec::new())
                    .push(func_name);
                // let mut module_calls = self.module_calls.get(&self.last_module).unwrap();
                // module_calls.push(func_name);
            }
            _ => {}
        };
    }

    fn visit_assert_malformed(&mut self, module: &ModuleBinary) {
        let wasm_binary: Vec<u8> = module.clone().into_vec();
        let command_name = self.command_name();
        // let wast_string = wasm2wat(wasm_binary).expect("Can't convert back to wasm");
        self.buffer.push_str(
            format!(
                "#[test]
fn {}_assert_malformed() {{
    let wasm_binary = {:?};
    let compilation = compile(wasm_binary.to_vec());
    assert!(compilation.is_err(), \"WASM should not compile as is malformed\");
}}\n",
                command_name,
                wasm_binary,
                // We do this to ident four spaces back
                // String::from_utf8_lossy(&wasm_binary),
                // wast_string.replace("\n", "\n    "),
            )
            .as_str(),
        );
    }

    // TODO: Refactor repetitive code
    fn visit_action(&mut self, action: &Action, expected: Option<&Vec<Value>>) -> Option<String> {
        match action {
            Action::Invoke {
                module: _,
                field,
                args,
            } => {
                let (func_return, assertion) = match expected {
                    Some(expected) => {
                        let func_return = if expected.len() > 0 {
                            format!(" -> {}", wabt2rust_type(&expected[0]))
                        } else {
                            "".to_string()
                        };
                        let expected_result = if expected.len() > 0 {
                            wabt2rust_value(&expected[0])
                        } else {
                            "()".to_string()
                        };
                        let assertion = if expected.len() > 0 && is_nan(&expected[0]) {
                            format!(
                                "assert!(result.is_nan());
            assert_eq!(result.is_sign_positive(), ({}).is_sign_positive());",
                                expected_result
                            )
                        } else {
                            format!("assert_eq!(result, {});", expected_result)
                        };
                        (func_return, assertion)
                    }
                    None => ("".to_string(), "".to_string()),
                };

                // We map the arguments provided into the raw Arguments provided
                // to libffi
                let mut args_types: Vec<String> = args.iter().map(wabt2rust_type).collect();
                args_types.push("&Instance".to_string());
                let mut args_values: Vec<String> = args.iter().map(wabt2rust_value).collect();
                args_values.push("&result_object.instance".to_string());
                let func_name = format!("{}_action_invoke", self.command_name());
                self.buffer.push_str(
                    format!(
                        "fn {}(result_object: &ResultObject) {{
    println!(\"Executing function {{}}\", \"{}\");
    let func_index = match result_object.module.info.exports.get({:?}) {{
        Some(&Export::Function(index)) => index,
        _ => panic!(\"Function not found\"),
    }};
    let invoke_fn: fn({}){} = get_instance_function!(result_object.instance, func_index);
    let result = invoke_fn({});
    {}
}}\n",
                        func_name,
                        func_name,
                        field,
                        args_types.join(", "),
                        func_return,
                        args_values.join(", "),
                        assertion,
                    )
                    .as_str(),
                );
                Some(func_name)
                // let mut module_calls = self.module_calls.get(&self.last_module).unwrap();
                // module_calls.push(func_name);
            }
            _ => None,
        }
    }

    fn visit_assert_return(&mut self, action: &Action, expected: &Vec<Value>) {
        let action_fn_name = self.visit_action(action, Some(expected));

        if action_fn_name.is_none() {
            return;
        }
        self.module_calls
            .entry(self.last_module)
            .or_insert(Vec::new())
            .push(action_fn_name.unwrap());
    }

    fn visit_perform_action(&mut self, action: &Action) {
        let action_fn_name = self.visit_action(action, None);

        if action_fn_name.is_none() {
            return;
        }
        self.module_calls
            .entry(self.last_module)
            .or_insert(Vec::new())
            .push(action_fn_name.unwrap());
    }

    fn visit_assert_trap(&mut self, action: &Action) {
        let action_fn_name = self.visit_action(action, None);

        if action_fn_name.is_none() {
            return;
        }
        let trap_func_name = format!("{}_assert_trap", self.command_name());
        self.buffer.push_str(
            format!(
                "
#[test]
fn {}() {{
    let result_object = create_module_{}();
    let result = call_protected!({}(&result_object));
    assert!(result.is_err());
}}\n",
                trap_func_name,
                self.last_module,
                action_fn_name.unwrap(),
            )
            .as_str(),
        );

        // We don't group trap calls as they may cause memory faults
        // on the instance memory. So we test them alone.
        // self.module_calls
        //     .entry(self.last_module)
        //     .or_insert(Vec::new())
        //     .push(trap_func_name);
    }

    fn visit_command(&mut self, cmd: &CommandKind) {
        match cmd {
            CommandKind::Module { module, name } => {
                self.visit_module(module, name);
            }
            CommandKind::AssertReturn { action, expected } => {
                self.visit_assert_return(action, expected)
            }
            CommandKind::AssertReturnCanonicalNan { action } => {
                self.visit_assert_return_canonical_nan(action);
            }
            CommandKind::AssertReturnArithmeticNan { action } => {
                self.visit_assert_return_arithmetic_nan(action);
            }
            CommandKind::AssertTrap { action, message: _ } => {
                self.visit_assert_trap(action);
            }
            CommandKind::AssertInvalid { module, message: _ } => {
                self.visit_assert_invalid(module);
            }
            CommandKind::AssertMalformed { module, message: _ } => {
                self.visit_assert_malformed(module);
            }
            CommandKind::AssertUninstantiable {
                module: _,
                message: _,
            } => {
                // Do nothing for now
            }
            CommandKind::AssertExhaustion { action: _ } => {
                // Do nothing for now
            }
            CommandKind::AssertUnlinkable {
                module: _,
                message: _,
            } => {
                // Do nothing for now
            }
            CommandKind::Register {
                name: _,
                as_name: _,
            } => {
                // Do nothing for now
            }
            CommandKind::PerformAction(action) => {
                self.visit_perform_action(action);
            }
        }
    }
    fn finalize(&self) -> &String {
        &self.buffer
    }
}

fn wast_to_rust(wast_filepath: &str) -> (String, i32) {
    let wast_filepath = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), wast_filepath);
    let path = PathBuf::from(&wast_filepath);
    let script_name: String = String::from(path.file_stem().unwrap().to_str().unwrap());
    let rust_test_filepath = format!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/spectests/{}.rs"),
        script_name.clone().as_str()
    );
    if script_name == "_common" {
        panic!("_common is a reserved name for the _common module. Please use other name for the spectest.");
    }

    let wast_modified = fs::metadata(&wast_filepath)
        .expect("Can't get wast file metadata")
        .modified()
        .expect("Can't get wast file modified date");
    let _should_modify = match fs::metadata(&rust_test_filepath) {
        Ok(m) => {
            m.modified()
                .expect("Can't get rust test file modified date")
                < wast_modified
        }
        Err(_) => true,
    };

    // panic!("SOULD MODIFY {:?} {:?}", should_modify, rust_test_filepath);

    // if true {
    // should_modify
    let mut generator = WastTestGenerator::new(&path);
    generator.consume();
    let generated_script = generator.finalize();
    fs::write(&rust_test_filepath, generated_script.as_bytes()).unwrap();
    // }
    (script_name, generator.command_no)
}

pub fn build() {
    let rust_test_modpath = concat!(env!("CARGO_MANIFEST_DIR"), "/src/spectests/mod.rs");

    let mut modules: Vec<String> = Vec::new();
    // modules.reserve_exact(TESTS.len());

    for test in TESTS.iter() {
        let (module_name, number_commands) = wast_to_rust(test);
        if number_commands > 200 {
            modules.push(format!(
                "#[cfg(not(feature = \"fast-tests\"))]
mod {};",
                module_name
            ));
        } else {
            modules.push(format!("mod {};", module_name));
        }
    }

    modules.insert(0, BANNER.to_string());
    modules.insert(1, "// The _common module is not autogenerated, as it provides common functions for the spectests\nmod _common;".to_string());
    // We add an empty line
    modules.push("".to_string());

    let modfile: String = modules.join("\n");
    let source = fs::read(&rust_test_modpath).unwrap();
    // We only modify the mod file if has changed
    if source != modfile.as_bytes() {
        fs::write(&rust_test_modpath, modfile.as_bytes()).unwrap();
    }
}
