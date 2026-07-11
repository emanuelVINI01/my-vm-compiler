pub mod expr;
pub mod control_flow;
pub mod function;
pub mod stmt;

use pest::iterators::Pair;
use crate::Rule;
use crate::context::{CompilationContext, Type};
use std::collections::HashMap;

pub struct CodeGenerator {
    pub ctx: CompilationContext,
    pub lines: Vec<String>,
    pub functions: HashMap<String, Vec<String>>,
    pub current_function: String,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            ctx: CompilationContext::new(),
            lines: Vec::new(),
            functions: HashMap::new(),
            current_function: "".to_string(),
        }
    }

    pub fn emit(&mut self, opcode: &str, args: &[&str]) {
        let mut parts = vec![opcode.to_string()];
        parts.extend(args.iter().map(|s| s.to_string()));
        self.lines.push(format!("{};", parts.join(" ")));
    }

    pub fn emit_label(&mut self, name: &str) {
        self.lines.push(format!("{}: ;", name));
    }

    pub fn generate(&mut self, pair: Pair<Rule>) {
        match pair.as_rule() {
            Rule::program => {
                self.emit("JMP", &["main"]);
                for statement in pair.into_inner() {
                    if statement.as_rule() != Rule::EOI {
                        self.visit_statement(statement);
                    }
                }
            }
            _ => panic!("Expected program rule"),
        }
    }

    pub fn visit_statement(&mut self, pair: Pair<Rule>) {
        let stmt = pair.into_inner().next().unwrap();
        match stmt.as_rule() {
            Rule::var_decl => self.visit_var_decl(stmt),
            Rule::array_decl => self.visit_array_decl(stmt),
            Rule::array_assign => self.visit_array_assign(stmt),
            Rule::assign => self.visit_assign(stmt),
            Rule::increment_stmt => self.visit_increment_stmt(stmt),
            Rule::return_stmt => {
                let mut inner = stmt.into_inner();
                if let Some(expr_pair) = inner.next() {
                    let (val, _) = self.visit_expr(expr_pair);
                    // Retorno salvo no registrador Z
                    self.emit("SET", &["Z", &val]);
                }
                
                if self.current_function == "main" {
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
            Rule::import_stmt => (), // Handle by main.rs
            _ => panic!("Unknown statement {:?}", stmt.as_rule()),
        }
    }

    pub fn parse_type_decl(&self, pair: Pair<Rule>) -> Type {
        match pair.as_str() {
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "bool" => Type::Bool,
            "void" => Type::Void,
            _ => Type::Int,
        }
    }
}
