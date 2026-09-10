use crate::ir::{IROp, IRProgram, IRFunction};
use std::collections::{HashMap, VecDeque};

// O CodeGen será responsável por transformar IRProgram -> String final (.asm)
// E também fará a Alocação de Registradores na RAM (Stack-Based Register Allocation)

pub struct StackAllocator {
    pub vreg_to_offset: HashMap<String, usize>,
    pub current_offset: usize,
}

impl StackAllocator {
    pub fn new() -> Self {
        StackAllocator {
            vreg_to_offset: HashMap::new(),
            current_offset: 1, // 0 is saved BP
        }
    }

    pub fn get_offset(&mut self, vreg: &str) -> usize {
        if let Some(&off) = self.vreg_to_offset.get(vreg) {
            off
        } else {
            let off = self.current_offset;
            self.vreg_to_offset.insert(vreg.to_string(), off);
            self.current_offset += 1;
            off
        }
    }

    pub fn load_operand(&mut self, op: &str, scratch_reg: &str, out: &mut Vec<String>) -> String {
        if op.starts_with("v") {
            let offset = self.get_offset(op);
            out.push(format!("SET Z {};", offset));
            out.push("SET V Y;".to_string()); // V = BP
            out.push("SUB V Z;".to_string()); // V = BP - offset
            out.push(format!("LOAD {} V;", scratch_reg));
            scratch_reg.to_string()
        } else if op.starts_with("g") {
            let num: u32 = op[1..].parse().unwrap();
            out.push(format!("SET Z {};", num));
            out.push(format!("LOAD {} Z;", scratch_reg));
            scratch_reg.to_string()
        } else {
            op.to_string() // literal
        }
    }

    pub fn store_result(&mut self, vreg: &str, scratch_reg: &str, out: &mut Vec<String>) {
        if vreg.starts_with("v") {
            let offset = self.get_offset(vreg);
            out.push(format!("SET Z {};", offset));
            out.push("SET V Y;".to_string()); // V = BP
            out.push("SUB V Z;".to_string()); // V = BP - offset
            out.push(format!("STORE V {};", scratch_reg));
        } else if vreg.starts_with("g") {
            let num: u32 = vreg[1..].parse().unwrap();
            out.push(format!("SET Z {};", num));
            out.push(format!("STORE Z {};", scratch_reg));
        } else {
            if vreg != scratch_reg {
                out.push(format!("SET {} {};", vreg, scratch_reg));
            }
        }
    }
}

pub fn generate_asm(program: IRProgram) -> Vec<String> {
    let mut output = Vec::new();

    // MAIN CODE / GLOBAIS
    let mut main_alloc = StackAllocator::new();
    let mut main_out = Vec::new();
    for op in &program.main_code {
        process_op(op, &mut main_alloc, &mut main_out, false);
    }
    
    let main_frame_size = main_alloc.current_offset;
    output.push(format!("SET Z {};", main_frame_size));
    output.push("GETSP W;".to_string());
    output.push("SUB W Z;".to_string());
    output.push("SETSP W;".to_string());
    
    output.extend(main_out);

    // FUNCOES
    for func in &program.functions {
        output.push(format!("{}: ;", func.name));
        
        let mut alloc = StackAllocator::new();
        
        let is_interrupt = func.name.ends_with("_interrupt_handler") || func.name.starts_with("isr_");
        
        if is_interrupt {
            output.push("PUSH W;".to_string());
            output.push("PUSH X;".to_string());
            output.push("PUSH Y;".to_string());
            output.push("PUSH Z;".to_string());
            output.push("PUSH V;".to_string());
        }
        
        let mut func_out = Vec::new();
        for op in &func.instructions {
            process_op(op, &mut alloc, &mut func_out, is_interrupt);
        }
        
        let frame_size = alloc.current_offset;
        
        output.push("PUSH Y;".to_string()); // Save caller's BP
        output.push("GETSP Y;".to_string()); // BP = SP
        
        // Allocate space for local variables by subtracting from SP
        output.push(format!("SET Z {};", frame_size));
        output.push("GETSP W;".to_string());
        output.push("SUB W Z;".to_string());
        output.push("SETSP W;".to_string());
        
        output.extend(func_out);
        
        // Epilogue
        output.push("SETSP Y;".to_string()); // Deallocate locals
        output.push("POP Y;".to_string());
        if is_interrupt {
            output.push("POP V;".to_string());
            output.push("POP Z;".to_string());
            output.push("POP Y;".to_string());
            output.push("POP X;".to_string());
            output.push("POP W;".to_string());
            output.push("IRET;".to_string());
        } else {
            output.push("RET;".to_string());
        }
    }

    output
}

fn process_op(op: &IROp, alloc: &mut StackAllocator, out: &mut Vec<String>, is_interrupt: bool) {
    match op {
        IROp::Add(dest, src) | IROp::Sub(dest, src) | IROp::Mul(dest, src) | IROp::Div(dest, src) | 
        IROp::Mod(dest, src) | IROp::Pow(dest, src) | IROp::FAdd(dest, src) | IROp::FSub(dest, src) | 
        IROp::FMul(dest, src) | IROp::FDiv(dest, src) => {
            let p_dest = alloc.load_operand(dest, "W", out);
            let p_src = alloc.load_operand(src, "X", out);
            let opcode = match op {
                IROp::Add(_,_) => "ADD", IROp::Sub(_,_) => "SUB", IROp::Mul(_,_) => "MUL",
                IROp::Div(_,_) => "DIV", IROp::Mod(_,_) => "MOD", IROp::Pow(_,_) => "POW",
                IROp::FAdd(_,_) => "FADD", IROp::FSub(_,_) => "FSUB", IROp::FMul(_,_) => "FMUL",
                IROp::FDiv(_,_) => "FDIV",
                _ => unreachable!()
            };
            out.push(format!("{} {} {};", opcode, p_dest, p_src));
            alloc.store_result(dest, "W", out);
        }
        IROp::Set(dest, val) => {
            let p_val = alloc.load_operand(val, "X", out);
            let p_dest = "W"; // Sempre carrega o literal ou registrador
            out.push(format!("SET {} {};", p_dest, p_val));
            alloc.store_result(dest, p_dest, out);
        }
        IROp::Load(dest, addr) => {
            let p_addr = alloc.load_operand(addr, "X", out);
            out.push(format!("LOAD W {};", p_addr));
            alloc.store_result(dest, "W", out);
        }
        IROp::Store(addr, src) => {
            let p_addr = alloc.load_operand(addr, "X", out);
            let p_src = alloc.load_operand(src, "W", out);
            out.push(format!("STORE {} {};", p_addr, p_src));
        }
        IROp::GetLastAddr(dest) => {
            out.push("GETLASTADDR W;".to_string());
            alloc.store_result(dest, "W", out);
        }
        IROp::Jmp(label) => out.push(format!("JMP {};", label)),
        IROp::Jeq(l, r, label) | IROp::Jne(l, r, label) | IROp::Jlt(l, r, label) | IROp::Jgt(l, r, label) |
        IROp::Jle(l, r, label) | IROp::Jge(l, r, label) => {
            let p_l = alloc.load_operand(l, "W", out);
            let p_r = alloc.load_operand(r, "X", out);
            let opcode = match op {
                IROp::Jeq(..) => "JEQ", IROp::Jne(..) => "JNE", IROp::Jlt(..) => "JLT", IROp::Jgt(..) => "JGT",
                IROp::Jle(..) => "JLE", IROp::Jge(..) => "JGE",
                _ => unreachable!()
            };
            out.push(format!("{} {} {} {};", opcode, p_l, p_r, label));
        }
        IROp::Call(func) => out.push(format!("CALL {};", func)),
        IROp::Ret | IROp::IRet => {
            out.push("SETSP Y;".to_string());
            out.push("POP Y;".to_string());
            if is_interrupt {
                out.push("POP V;".to_string());
                out.push("POP Z;".to_string());
                out.push("POP Y;".to_string());
                out.push("POP X;".to_string());
                out.push("POP W;".to_string());
                out.push("IRET;".to_string());
            } else {
                out.push("RET;".to_string());
            }
        }
        IROp::Halt => out.push("HALT;".to_string()),
        IROp::Push(reg) => {
            let p = alloc.load_operand(reg, "W", out);
            out.push(format!("PUSH {};", p));
        }
        IROp::Pop(reg) => {
            out.push("POP W;".to_string());
            alloc.store_result(reg, "W", out);
        }

        IROp::Out(p, v) => {
            let p_p = alloc.load_operand(p, "W", out);
            let p_v = alloc.load_operand(v, "X", out);
            out.push(format!("OUT {}, {};", p_p, p_v));
        }
        IROp::In(d, p) => {
            let p_p = alloc.load_operand(p, "W", out);
            out.push(format!("IN X, {};", p_p));
            alloc.store_result(d, "X", out);
        }
        IROp::GetSp(dest) => {
            out.push("GETSP W;".to_string());
            alloc.store_result(dest, "W", out);
        }
        IROp::SetSp(src) => {
            let p_src = alloc.load_operand(src, "W", out);
            out.push(format!("SETSP {};", p_src));
        }
        IROp::Cli => out.push("CLI;".to_string()),
        IROp::Sti => out.push("STI;".to_string()),
        IROp::Yield => out.push("YIELD;".to_string()),
        IROp::Label(l) => out.push(format!("{}: ;", l)),
        IROp::RawLine(line) => out.push(line.clone()),
        IROp::FuncAddr(dest, func_name) => {
            out.push(format!("LABELADDR W, {};", func_name));
            alloc.store_result(dest, "W", out);
        }
        IROp::LoadPhys(phys, vreg) => {
            // Carrega registrador virtual (stack-based) para registrador físico A-N
            // Usa o mesmo método que load_operand mas com destino específico
            let src = alloc.load_operand(vreg, "W", out);
            if src != *phys {
                out.push(format!("SET {}, {};", phys, src));
            }
        }
        IROp::StorePhys(dest, phys) => {
            // Escreve o registrador físico de volta na variável real (stack/global)
            alloc.store_result(dest, phys, out);
        }
        _ => panic!("IROp não implementado no CodeGen {:?}", op),
    }
}
