use crate::ir::IRProgram;

pub fn optimize(mut program: IRProgram) -> IRProgram {
    // Para essa primeira etapa, o Otimizador é um pass-through.
    // Futuras otimizações como Constant Folding e Dead Code Elimination entrarão aqui.
    program
}
