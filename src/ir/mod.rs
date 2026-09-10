use std::fmt;

#[derive(Clone, Debug)]
pub enum IROp {
    // Arithmetic & Logic
    Add(String, String), // dest, src
    Sub(String, String),
    Mul(String, String),
    Div(String, String),
    Mod(String, String),
    Pow(String, String),
    
    // Float
    FAdd(String, String),
    FSub(String, String),
    FMul(String, String),
    FDiv(String, String),

    // Memory / Assignment
    Set(String, String), // dest, val
    Load(String, String), // dest, addr
    Store(String, String), // addr, src
    GetLastAddr(String),

    // Control Flow
    Jmp(String),
    Jeq(String, String, String), // left, right, label
    Jne(String, String, String),
    Jlt(String, String, String),
    Jgt(String, String, String),
    Jle(String, String, String),
    Jge(String, String, String),
    Call(String),
    Ret,
    Halt,

    // Stack (Internal)
    Push(String),
    Pop(String),
    IRet,
    GetSp(String),
    SetSp(String),

    // IO / Macros
    Out(String, String), // port, val
    In(String, String), // dest, port
    Write(String, String, String), // fd, addr, len
    Read(String, String, String),  // fd, addr, len
    VideoUpdate,
    Cli,
    Sti,
    Yield,
    
    // Labels
    Label(String),
    
    // Raw assembly line emitida diretamente (para novos opcodes GUI, etc.)
    RawLine(String),
    
    // Carrega registrador virtual para registrador físico específico (para inline asm)
    // LoadPhys(phys_reg: String, vreg: String)
    LoadPhys(String, String),

    // Carrega o endereço (índice de instrução) de uma função/label num registrador virtual
    // FuncAddr(dest_vreg, func_name)
    FuncAddr(String, String),
}

#[derive(Clone)]
pub struct IRFunction {
    pub name: String,
    pub instructions: Vec<IROp>,
}

#[derive(Clone)]
pub struct IRProgram {
    pub functions: Vec<IRFunction>,
    pub main_code: Vec<IROp>,
}
