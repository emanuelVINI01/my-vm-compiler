use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;
use crate::context::Type;

impl CodeGenerator {
    pub fn visit_expr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        let inner = pair.into_inner().next().unwrap();
        self.visit_expr_node(inner)
    }

    pub fn visit_expr_node(&mut self, inner: Pair<Rule>) -> (String, Type) {
        match inner.as_rule() {
            Rule::func_call_expr => self.visit_func_call_expr(inner),
            Rule::int_num => (inner.as_str().to_string(), Type::Int),
            Rule::float_num => {
                let val: f32 = inner.as_str().parse().unwrap();
                (val.to_bits().to_string(), Type::Float)
            },
            Rule::string_lit => (inner.into_inner().next().unwrap().as_str().to_string(), Type::String),
            Rule::ident => {
                let name = inner.as_str();
                if name == "true" {
                    ("1".to_string(), Type::Bool)
                } else if name == "false" {
                    ("0".to_string(), Type::Bool)
                } else {
                    let reg = self.ctx.get_register(name);
                    let t = self.ctx.get_type(name);
                    (reg, t)
                }
            },
            Rule::array_access => {
                let mut aa_inner = inner.into_inner();
                let ident = aa_inner.next().unwrap().as_str();
                let index_expr = aa_inner.next().unwrap();
                let base_reg = self.ctx.get_register(ident);
                let (index_reg, _) = self.visit_expr(index_expr);
                
                let addr_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
                self.ctx.label_counter += 1;
                let val_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
                self.ctx.label_counter += 1;
                
                self.emit("SET", &[&addr_reg, &base_reg]);
                self.emit("ADD", &[&addr_reg, &index_reg]);
                self.emit("LOAD", &[&val_reg, &addr_reg]);
                
                if index_reg.starts_with("_tmp") { self.ctx.free_register(&index_reg); }
                self.ctx.free_register(&addr_reg);
                // Array elements are treated as Ints by default unless tracked properly
                (val_reg, Type::Int)
            },
            Rule::binary_expr => self.visit_binary_expr(inner),
            Rule::expr => self.visit_expr(inner),
            _ => panic!("Unknown expression {:?}", inner.as_rule()),
        }
    }

    pub fn visit_binary_expr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();
        let mut current_val = self.visit_expr_node(first);
        
        while let Some(op_pair) = inner.next() {
            let op = op_pair.as_str();
            let next_raw = inner.next().unwrap();
            let next_val = self.visit_expr_node(next_raw);
            
            let is_float = current_val.1 == Type::Float || next_val.1 == Type::Float;
            let res_type = if op == "==" || op == "!=" || op == "<" || op == ">" || op == "<=" || op == ">=" || op == "&&" || op == "||" {
                Type::Bool
            } else if is_float {
                Type::Float
            } else {
                Type::Int
            };
            
            let target = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), res_type.clone());
            self.ctx.label_counter += 1;
            
            self.emit("SET", &[&target, &current_val.0]);
            
            match op {
                "+" => self.emit(if is_float { "FADD" } else { "ADD" }, &[&target, &next_val.0]),
                "-" => self.emit(if is_float { "FSUB" } else { "SUB" }, &[&target, &next_val.0]),
                "*" => self.emit(if is_float { "FMUL" } else { "MUL" }, &[&target, &next_val.0]),
                "/" => self.emit(if is_float { "FDIV" } else { "DIV" }, &[&target, &next_val.0]),
                "%" => self.emit("MOD", &[&target, &next_val.0]),
                "^" => self.emit("POW", &[&target, &next_val.0]),
                "&&" => self.emit("MUL", &[&target, &next_val.0]), // boolean AND is multiplication
                "||" => {
                    self.emit("ADD", &[&target, &next_val.0]);
                    let label_skip = self.ctx.new_label("or_skip");
                    self.emit("JEQ", &[&target, "0", &label_skip]);
                    self.emit("SET", &[&target, "1"]);
                    self.emit_label(&label_skip);
                },
                "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                    let label_true = self.ctx.new_label("cmp_true");
                    let label_end = self.ctx.new_label("cmp_end");
                    let jmp_op = match op {
                        "==" => "JEQ",
                        "!=" => "JNE",
                        "<" => "JLT",
                        ">" => "JGT",
                        "<=" => "JLE",
                        ">=" => "JGE",
                        _ => unreachable!()
                    };
                    self.emit(jmp_op, &[&current_val.0, &next_val.0, &label_true]);
                    self.emit("SET", &[&target, "0"]);
                    self.emit("JMP", &[&label_end]);
                    self.emit_label(&label_true);
                    self.emit("SET", &[&target, "1"]);
                    self.emit_label(&label_end);
                },
                _ => panic!("Operador não suportado: {}", op),
            }
            
            current_val = (target, res_type);
        }
        
        current_val
    }
}
