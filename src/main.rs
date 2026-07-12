pub mod context;
pub mod generator;
pub mod ir;
pub mod optimizer;
pub mod codegen;
use crate::generator::CodeGenerator;
use pest::Parser;
use pest_derive::Parser;
use std::env;
use std::fs;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CVMParser;

use std::path::Path;

fn preprocess(content: &str, base_dir: &Path) -> String {
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <arquivo.cvm> <saida.asm>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let base_dir = input_path.parent().unwrap_or(Path::new(""));

    let unparsed_file = fs::read_to_string(&input_path).expect("Falha ao ler o arquivo");
    let preprocessed_file = preprocess(&unparsed_file, base_dir);

    let parse_result = CVMParser::parse(Rule::program, &preprocessed_file);

    match parse_result {
        Ok(mut pairs) => {
            println!("✅ Arquivo parseado com sucesso!");
            let program = pairs.next().unwrap();

            let mut ir_builder = CodeGenerator::new();
            ir_builder.generate(program);
            
            let mut ir_program = ir_builder.program;
            ir_program = crate::optimizer::optimize(ir_program);
            
            // DUMP DO IR PARA DEBUG
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
            let ir_file = input_path.with_extension("ir");
            fs::write(&ir_file, ir_dump).expect("Falha ao escrever o arquivo .ir");
            
            let asm_lines = crate::codegen::generate_asm(ir_program);
            let asm_code = asm_lines.join("\n");
            
            fs::write(&args[2], asm_code).expect("Falha ao escrever o arquivo .asm");
            println!("🚀 Compilação concluída: {}", args[2]);
            println!("🔍 IR Intermediário salvo em: {}", ir_file.display());
        }
        Err(e) => {
            eprintln!("❌ Erro de Sintaxe:\n{}", e);
            std::process::exit(1);
        }
    }
}
