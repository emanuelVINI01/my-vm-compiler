pub mod control_flow;
pub mod expr;
pub mod function;
pub mod stmt;

use crate::Rule;
use crate::context::{CompilationContext, Type};
use pest::iterators::Pair;
use std::collections::HashMap;

use crate::ir::{IROp, IRFunction, IRProgram};

pub struct CodeGenerator {
    pub ctx: CompilationContext,
    pub program: IRProgram,
    pub current_function_name: String,
    pub current_instructions: Vec<IROp>,
    pub functions_info: HashMap<String, Vec<String>>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            ctx: CompilationContext::new(),
            program: IRProgram { functions: Vec::new(), main_code: Vec::new() },
            current_function_name: "".to_string(),
            current_instructions: Vec::new(),
            functions_info: HashMap::new(),
        }
    }

    pub fn emit(&mut self, opcode: &str, args: &[&str]) {
        let op = match opcode {
            "ADD" => IROp::Add(args[0].to_string(), args[1].to_string()),
            "SUB" => IROp::Sub(args[0].to_string(), args[1].to_string()),
            "MUL" => IROp::Mul(args[0].to_string(), args[1].to_string()),
            "DIV" => IROp::Div(args[0].to_string(), args[1].to_string()),
            "MOD" => IROp::Mod(args[0].to_string(), args[1].to_string()),
            "POW" => IROp::Pow(args[0].to_string(), args[1].to_string()),
            
            "FADD" => IROp::FAdd(args[0].to_string(), args[1].to_string()),
            "FSUB" => IROp::FSub(args[0].to_string(), args[1].to_string()),
            "FMUL" => IROp::FMul(args[0].to_string(), args[1].to_string()),
            "FDIV" => IROp::FDiv(args[0].to_string(), args[1].to_string()),
            
            "SET" => IROp::Set(args[0].to_string(), args[1].to_string()),
            "LOAD" => IROp::Load(args[0].to_string(), args[1].to_string()),
            "STORE" => IROp::Store(args[0].to_string(), args[1].to_string()),
            "GETLASTADDR" => IROp::GetLastAddr(args[0].to_string()),
            
            "JMP" => IROp::Jmp(args[0].to_string()),
            "JEQ" => IROp::Jeq(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            "JNE" => IROp::Jne(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            "JLT" => IROp::Jlt(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            "JGT" => IROp::Jgt(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            "JLE" => IROp::Jle(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            "JGE" => IROp::Jge(args[0].to_string(), args[1].to_string(), args[2].to_string()),
            
            "CALL" => IROp::Call(args[0].to_string()),
            "RET" => IROp::Ret,
            "HALT" => IROp::Halt,
            
            "PUSH" => IROp::Push(args[0].to_string()),
            "POP" => IROp::Pop(args[0].to_string()),
            "IRET" => IROp::IRet,
            
            "OUT" => IROp::Out(args[0].to_string(), args[1].to_string()),
            "IN" => IROp::In(args[0].to_string(), args[1].to_string()),
            "GETSP" => IROp::GetSp(args[0].to_string()),
            "SETSP" => IROp::SetSp(args[0].to_string()),
            "CLI" => IROp::Cli,
            "STI" => IROp::Sti,
            "YIELD" => IROp::Yield,
            
            _ => panic!("Opcode desconhecido no emit do IR: {}", opcode),
        };
        self.current_instructions.push(op);
    }

    pub fn emit_raw(&mut self, line: &str) {
        // Raw emits (used in inline assembly)
        let trimmed = line.trim().trim_end_matches(';');
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() { return; }
        
        let opcode = parts[0];
        let mut args = Vec::new();
        let joined = parts[1..].join("");
        for arg in joined.split(',') {
            let a = arg.trim();
            if !a.is_empty() { args.push(a); }
        }
        
        match opcode {
            "IN" => self.emit("IN", &args),
            "OUT" => self.emit("OUT", &args),
            "GETSP" => self.emit("GETSP", &args),
            "SETSP" => self.emit("SETSP", &args),
            "CLI" => self.emit("CLI", &[]),
            "STI" => self.emit("STI", &[]),
            "YIELD" => self.emit("YIELD", &[]),
            "HLT" | "HALT" => self.emit("HALT", &[]),
            // Todos os outros opcodes (incluindo novos GUI) são emitidos como RawLine
            _ => {
                self.current_instructions.push(crate::ir::IROp::RawLine(format!("{};" , trimmed)));
            }
        }
    }

    pub fn get_type_size(&self, ty: &Type) -> u32 {
        match ty {
            Type::Struct(name) => {
                if let Some(info) = self.ctx.structs.get(name) {
                    info.fields.len() as u32
                } else {
                    1
                }
            }
            _ => 1,
        }
    }

    pub fn emit_label(&mut self, name: &str) {
        self.current_instructions.push(IROp::Label(name.to_string()));
    }

    pub fn generate(&mut self, pair: Pair<Rule>) {
        match pair.as_rule() {
            Rule::program => {
                for statement in pair.into_inner() {
                    if statement.as_rule() != Rule::EOI {
                        self.visit_statement(statement);
                    }
                }
                self.emit("CALL", &["main"]);
                self.emit("HALT", &[]);
                
                self.program.main_code = std::mem::take(&mut self.current_instructions);
            }
            _ => panic!("Expected program rule"),
        }
    }

    pub fn visit_statement(&mut self, pair: Pair<Rule>) {
        let stmt = pair.into_inner().next().unwrap();
        match stmt.as_rule() {
            Rule::struct_decl => self.visit_struct_decl(stmt),
            Rule::var_decl => self.visit_var_decl(stmt),
            Rule::array_decl => self.visit_array_decl(stmt),
            Rule::array_assign => self.visit_array_assign(stmt),
            Rule::pointer_assign => self.visit_pointer_assign(stmt),
            Rule::field_assign => self.visit_field_assign(stmt),
            Rule::assign => self.visit_assign(stmt),
            Rule::increment_stmt => self.visit_increment_stmt(stmt),
            Rule::return_stmt => {
                let mut inner = stmt.into_inner();
                if let Some(expr_pair) = inner.next() {
                    let (val, _) = self.visit_expr(expr_pair);
                    // Retorno salvo no registrador Z
                    self.emit("SET", &["Z", &val]);
                }

                if self.current_function_name == "main" {
                    self.emit("HALT", &[]);
                } else {
                    self.emit("RET", &[]);
                }
            }
            Rule::if_stmt => self.visit_if(stmt),
            Rule::while_stmt => self.visit_while(stmt),
            Rule::for_stmt => self.visit_for(stmt),
            Rule::macro_call => self.visit_macro(stmt),
            Rule::func_call => self.visit_func_call(stmt),
            Rule::function => self.visit_function(stmt),
            Rule::block => self.visit_block(stmt),
            Rule::asm_block => self.visit_asm_block(stmt),
            Rule::import_stmt => (), // Handle by main.rs
            _ => panic!("Unknown statement {:?}", stmt.as_rule()),
        }
    }

    pub fn parse_type_decl(&self, pair: Pair<Rule>) -> Type {
        match pair.as_rule() {
            Rule::pointer_type => {
                let mut inner = pair.into_inner();
                let inner_type = inner.next().unwrap();
                Type::Pointer(Box::new(self.parse_type_decl(inner_type)))
            }
            Rule::type_decl => {
                let inner = pair.into_inner().next().unwrap();
                self.parse_type_decl(inner)
            }
            _ => {
                let name = pair.as_str();
                match name {
                    "int" => Type::Int,
                    "float" => Type::Float,
                    "string" => Type::String,
                    "bool" => Type::Bool,
                    "void" => Type::Void,
                    _ => {
                        if self.ctx.structs.contains_key(name) {
                            Type::Struct(name.to_string())
                        } else {
                            Type::Int
                        }
                    }
                }
            }
        }
    }
}
