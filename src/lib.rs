pub mod context;
pub mod generator;
pub mod ir;
pub mod optimizer;
pub mod codegen;

use crate::generator::CodeGenerator;
use pest::Parser;
use pest_derive::Parser;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CVMParser;

pub fn preprocess(content: &str, base_dir: &Path) -> String {
    let mut result = content.to_string();

    while let Some(start) = result.find("import \"") {
        let end_rel = result[start..]
            .find("\";")
            .expect("Missing closing \"; on import statement");
        let end = start + end_rel;

        let stmt = &result[start..end + 2];
        let filename = &result[start + 8..end];

        let file_path = base_dir.join(filename);
        let imported_content = fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to import {}", file_path.display()));

        let new_base_dir = file_path.parent().unwrap_or(Path::new(""));
        let processed_import = preprocess(&imported_content, new_base_dir);

        // We replace the first occurrence
        result = result.replacen(stmt, &processed_import, 1);
    }

    result
}

/// Compiles already-read CVM source, resolving `import "..."` statements relative
/// to `base_dir`. Returns `(ir_dump, asm_code)` on success.
pub fn compile_source(source: &str, base_dir: &Path) -> Result<(String, String), String> {
    let preprocessed_file = preprocess(source, base_dir);

    let parse_result = CVMParser::parse(Rule::program, &preprocessed_file);

    match parse_result {
        Ok(mut pairs) => {
            let program = pairs.next().unwrap();

            let mut ir_builder = CodeGenerator::new();
            ir_builder.generate(program);

            let mut ir_program = ir_builder.program;
            ir_program = crate::optimizer::optimize(ir_program);

            let mut ir_dump = String::new();
            ir_dump.push_str("--- MAIN CODE ---\n");
            for op in &ir_program.main_code {
                ir_dump.push_str(&format!("{:?}\n", op));
            }
            for func in &ir_program.functions {
                ir_dump.push_str(&format!("\n--- FUNC {} ---\n", func.name));
                for op in &func.instructions {
                    ir_dump.push_str(&format!("{:?}\n", op));
                }
            }

            let asm_lines = crate::codegen::generate_asm(ir_program);
            let asm_code = asm_lines.join("\n");

            Ok((ir_dump, asm_code))
        }
        Err(e) => Err(format!("{}", e)),
    }
}

/// Reads a `.cvm` file from disk and compiles it, using its parent directory to
/// resolve `import "..."` statements.
pub fn compile_file(input_path: &Path) -> Result<(String, String), String> {
    let base_dir = input_path.parent().unwrap_or(Path::new(""));
    let unparsed_file = fs::read_to_string(input_path)
        .map_err(|e| format!("Falha ao ler o arquivo: {e}"))?;
    compile_source(&unparsed_file, base_dir)
}
