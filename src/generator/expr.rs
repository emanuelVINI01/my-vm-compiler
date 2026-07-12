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
            Rule::hex_num => {
                let without_prefix = &inner.as_str()[2..];
                let val = u32::from_str_radix(without_prefix, 16).unwrap();
                (val.to_string(), Type::Int)
            },
            Rule::float_num => {
                let val: f32 = inner.as_str().parse().unwrap();
                (val.to_bits().to_string(), Type::Float)
            },
            Rule::string_lit => (inner.into_inner().next().unwrap().as_str().to_string(), Type::String),
            Rule::deref_expr => {
                let ident = inner.into_inner().next().unwrap().as_str();
                let addr_reg = self.ctx.get_register(ident);
                let var_type = self.ctx.get_type(ident);
                let inner_type = match &var_type {
                    Type::Pointer(t) => *t.clone(),
                    _ => Type::Int,
                };
                let val_reg = self.ctx.allocate_register(
                    &format!("_tmp_{}", self.ctx.label_counter),
                    inner_type.clone()
                );
                self.ctx.label_counter += 1;
                self.emit("LOAD", &[&val_reg, &addr_reg]);
                (val_reg, inner_type)
            },
            Rule::field_access => self.visit_field_access_expr(inner),
            Rule::ident => {
                let name = inner.as_str();
                if name == "true" {
                    ("1".to_string(), Type::Bool)
                } else if name == "false" {
                    ("0".to_string(), Type::Bool)
                } else if self.functions_info.contains_key(name) {
                    let tmp = self.ctx.allocate_register(
                        &format!("_tmp_{}", self.ctx.label_counter), Type::Int);
                    self.ctx.label_counter += 1;
                    self.emit("SET", &[&tmp, "0"]);
                    eprintln!("Aviso: referência de função '{}' usada como valor", name);
                    (tmp, Type::Int)
                } else if self.ctx.variables.contains_key(name) || self.ctx.global_variables.contains_key(name) {
                    let reg = self.ctx.get_register(name);
                    let t = self.ctx.get_type(name);
                    (reg, t)
                } else {
                    let tmp = self.ctx.allocate_register(
                        &format!("_tmp_{}", self.ctx.label_counter), Type::Int);
                    self.ctx.label_counter += 1;
                    self.emit("SET", &[&tmp, "0"]);
                    eprintln!("Aviso: símbolo '{}' não declarado, assumindo 0", name);
                    (tmp, Type::Int)
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
        
        let mut operands = Vec::new();
        let mut operators = Vec::new();
        
        operands.push(self.visit_expr_node(inner.next().unwrap()));
        
        while let Some(op_pair) = inner.next() {
            operators.push(op_pair.as_str().to_string());
            operands.push(self.visit_expr_node(inner.next().unwrap()));
        }
        
        let precedence_levels = vec![
            vec!["*", "/", "%", "^"],
            vec!["+", "-"],
            vec!["<", ">", "<=", ">=", "==", "!="],
            vec!["&&", "||"],
        ];
        
        for level in precedence_levels {
            let mut i = 0;
            while i < operators.len() {
                if level.contains(&operators[i].as_str()) {
                    let op = operators.remove(i);
                    let left = operands[i].clone();
                    let right = operands.remove(i + 1);
                    
                    let res = self.emit_binary_op(&left, &op, &right);
                    operands[i] = res;
                } else {
                    i += 1;
                }
            }
        }
        
        operands[0].clone()
    }

    fn emit_binary_op(&mut self, left: &(String, Type), op: &str, right: &(String, Type)) -> (String, Type) {
        let is_float = left.1 == Type::Float || right.1 == Type::Float;
        let res_type = if op == "==" || op == "!=" || op == "<" || op == ">" || op == "<=" || op == ">=" || op == "&&" || op == "||" {
            Type::Bool
        } else if is_float {
            Type::Float
        } else {
            Type::Int
        };
        
        let target = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), res_type.clone());
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&target, &left.0]);
        
        match op {
            "+" => self.emit(if is_float { "FADD" } else { "ADD" }, &[&target, &right.0]),
            "-" => self.emit(if is_float { "FSUB" } else { "SUB" }, &[&target, &right.0]),
            "*" => self.emit(if is_float { "FMUL" } else { "MUL" }, &[&target, &right.0]),
            "/" => self.emit(if is_float { "FDIV" } else { "DIV" }, &[&target, &right.0]),
            "%" => self.emit("MOD", &[&target, &right.0]),
            "^" => self.emit("POW", &[&target, &right.0]),
            "&&" => self.emit("MUL", &[&target, &right.0]),
            "||" => {
                self.emit("ADD", &[&target, &right.0]);
                let label_skip = self.ctx.new_label("or_skip");
                self.emit("JEQ", &[&target, "0", &label_skip]);
                self.emit("SET", &[&target, "1"]);
                self.emit_label(&label_skip);
            },
            "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                let label_true = self.ctx.new_label("cmp_true");
                let label_end = self.ctx.new_label("cmp_end");
                let jmp_op = match op {
                    "==" => "JEQ", "!=" => "JNE", "<" => "JLT", ">" => "JGT", "<=" => "JLE", ">=" => "JGE",
                    _ => unreachable!()
                };
                self.emit(jmp_op, &[&left.0, &right.0, &label_true]);
                self.emit("SET", &[&target, "0"]);
                self.emit("JMP", &[&label_end]);
                self.emit_label(&label_true);
                self.emit("SET", &[&target, "1"]);
                self.emit_label(&label_end);
            },
            _ => panic!("Operador não suportado: {}", op),
        }
        
        if left.0.starts_with("_tmp") { self.ctx.free_register(&left.0); }
        if right.0.starts_with("_tmp") { self.ctx.free_register(&right.0); }
        
        (target, res_type)
    }

    pub fn visit_field_access_expr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        let (addr_reg, field_type) = self.visit_field_access_addr(pair);
        let val_reg = self.ctx.allocate_register(
            &format!("_tmp_{}", self.ctx.label_counter),
            field_type.clone()
        );
        self.ctx.label_counter += 1;

        self.emit("LOAD", &[&val_reg, &addr_reg]);
        if addr_reg.starts_with("_tmp") {
            self.ctx.free_register(&addr_reg);
        }
        (val_reg, field_type)
    }

    pub fn visit_field_access_addr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        let base_reg: String;
        let struct_name: String;

        if first.as_rule() == Rule::array_access {
            let mut aa = first.into_inner();
            let arr_ident = aa.next().unwrap().as_str();
            let index_expr = aa.next().unwrap();
            let arr_type = self.ctx.get_type(arr_ident);

            struct_name = match &arr_type {
                Type::Array(inner) => match inner.as_ref() {
                    Type::Struct(name) => name.clone(),
                    _ => panic!("Tentativa de acessar campo de tipo não-struct"),
                },
                _ => panic!("Tipo inválido para field access com array"),
            };

            let base_ident_reg = self.ctx.get_register(arr_ident);
            let (idx, _) = self.visit_expr(index_expr);

            let struct_size = self.get_type_size(&Type::Struct(struct_name.clone()));

            let stride_reg = self.ctx.allocate_register(
                &format!("_tmp_{}", self.ctx.label_counter),
                Type::Int
            );
            self.ctx.label_counter += 1;

            let addr_reg = self.ctx.allocate_register(
                &format!("_tmp_{}", self.ctx.label_counter),
                Type::Int
            );
            self.ctx.label_counter += 1;

            self.emit("SET", &[&stride_reg, &struct_size.to_string()]);
            self.emit("MUL", &[&stride_reg, &idx]);
            self.emit("SET", &[&addr_reg, &base_ident_reg]);
            self.emit("ADD", &[&addr_reg, &stride_reg]);

            if idx.starts_with("_tmp") { self.ctx.free_register(&idx); }
            if stride_reg.starts_with("_tmp") { self.ctx.free_register(&stride_reg); }

            base_reg = addr_reg;
        } else {
            let ident = first.as_str();
            let var_type = self.ctx.get_type(ident);
            struct_name = match &var_type {
                Type::Struct(name) => name.clone(),
                _ => panic!("Tentativa de acessar campo de variável não-struct: {}", ident),
            };
            base_reg = self.ctx.get_register(ident);
        }

        let field_name = inner.next().unwrap().as_str();

        let offset;
        let field_type;
        {
            let struct_info = self.ctx.structs.get(&struct_name)
                .expect(&format!("Struct '{}' não definida", struct_name));
            offset = *struct_info.offsets.get(field_name)
                .expect(&format!("Campo '{}' não existe na struct '{}'", field_name, struct_name));
            field_type = struct_info.fields.iter()
                .find(|(n, _)| n == field_name)
                .map(|(_, t)| t.clone())
                .unwrap_or(Type::Int);
        }

        let field_addr = self.ctx.allocate_register(
            &format!("_tmp_{}", self.ctx.label_counter),
            Type::Int
        );
        self.ctx.label_counter += 1;

        self.emit("SET", &[&field_addr, &base_reg]);
        if offset > 0 {
            let off_reg = self.ctx.allocate_register(
                &format!("_tmp_{}", self.ctx.label_counter),
                Type::Int
            );
            self.ctx.label_counter += 1;
            self.emit("SET", &[&off_reg, &offset.to_string()]);
            self.emit("ADD", &[&field_addr, &off_reg]);
            if off_reg.starts_with("_tmp") { self.ctx.free_register(&off_reg); }
        }

        if base_reg.starts_with("_tmp") { self.ctx.free_register(&base_reg); }

        (field_addr, field_type)
    }
}
