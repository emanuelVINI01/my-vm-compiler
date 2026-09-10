//! Testes de regressão (golden/snapshot tests) contra o comportamento ATUAL do
//! compilador — não uma validação abstrata de "correção". Os arquivos soltos em
//! `test/*.asm`/`test/*.ir` já estavam desatualizados frente ao codegen atual
//! (não são regenerados automaticamente), então os snapshots usados aqui vivem
//! em `tests/snapshots/` e foram capturados a partir do compilador em
//! 2026-08-29. Se um destes testes falhar depois de uma mudança intencional no
//! codegen, regenere o snapshot correspondente e revise o diff manualmente
//! antes de commitar.

use my_vm_compiler::compile_file;
use std::path::Path;

#[test]
fn main_cvm_matches_snapshot() {
    let (ir_dump, asm_code) =
        compile_file(Path::new("test/main.cvm")).expect("main.cvm deveria compilar sem erro");

    let expected_asm = std::fs::read_to_string("tests/snapshots/main.asm.snap")
        .expect("snapshot main.asm.snap ausente");
    let expected_ir = std::fs::read_to_string("tests/snapshots/main.ir.snap")
        .expect("snapshot main.ir.snap ausente");

    assert_eq!(asm_code, expected_asm.trim_end(), "Assembly gerado para main.cvm mudou");
    assert_eq!(ir_dump.trim_end(), expected_ir.trim_end(), "IR gerado para main.cvm mudou");
}

#[test]
fn memory_cvm_compiles_without_error() {
    // memory.cvm não tem um `.asm` de referência confiável (o `test/output.asm`
    // existente no repo corresponde a uma versão mais antiga/diferente do
    // compilador, com outra estratégia de alocação de registradores — não é um
    // golden file válido para comparação exata). Este é um smoke test: garante
    // que o arquivo continua compilando e produzindo saída não vazia.
    let (_ir_dump, asm_code) =
        compile_file(Path::new("test/memory.cvm")).expect("memory.cvm deveria compilar sem erro");

    assert!(!asm_code.trim().is_empty(), "memory.cvm deveria gerar Assembly não vazio");
}

// math.cvm está vazio (0 bytes) no repositório — não há programa para compilar,
// então não existe um teste de regressão significativo para ele.
