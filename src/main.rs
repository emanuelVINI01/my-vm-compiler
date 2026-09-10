use my_vm_compiler::compile_file;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <arquivo.cvm> <saida.asm>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);

    match compile_file(input_path) {
        Ok((ir_dump, asm_code)) => {
            println!("✅ Arquivo parseado com sucesso!");

            let ir_file = input_path.with_extension("ir");
            fs::write(&ir_file, ir_dump).expect("Falha ao escrever o arquivo .ir");

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
