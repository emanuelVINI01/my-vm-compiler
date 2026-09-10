use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;
use crate::context::Type;
use crate::context::StructInfo;
use std::collections::HashMap;

impl CodeGenerator {
    pub fn visit_asm_block(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let string_lit = inner.next().unwrap();
        let asm_text = string_lit.into_inner().next().unwrap().as_str().to_string();

        // Detecta todas as variáveis referenciadas no template asm
        let chars: Vec<char> = asm_text.chars().collect();
        let mut i = 0;
        let mut var_names: Vec<String> = Vec::new();
        while i < chars.len() {
            if chars[i] == '{' {
                let start = i + 1;
                if let Some(end) = chars[start..].iter().position(|&c| c == '}') {
                    let var_name: String = chars[start..start + end].iter().collect();
                    if !var_names.contains(&var_name) {
                        var_names.push(var_name);
                    }
                    i = start + end + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Registradores físicos disponíveis para uso temporário no asm (A..N são "seguros")
        // Evitamos W, X, Y, Z, V, U, T, S que são usados pelo codegen internamente
        let phys_regs = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N"];
        let mut var_to_phys: Vec<(String, String)> = Vec::new();
        let mut phys_idx = 0;

        // Para cada variável, emite load para registrador físico se for virtual (vX ou gX)
        for var_name in &var_names {
            let info_local = self.ctx.variables.get(var_name).cloned();
            let info_global = self.ctx.global_variables.get(var_name).cloned();
            
            let reg = if let Some(info) = &info_local {
                info.reg.clone()
            } else if let Some(info) = &info_global {
                info.reg.clone()
            } else {
                // Não é variável conhecida, deixa sem substituição
                continue;
            };
            
            if reg.starts_with('v') {
                // Registrador local virtual (stack-based) - emite LoadPhys(A, v5) etc.
                // O codegen vai converter isso em: SET Z offset; SET V Y; SUB V Z; LOAD W V; SET A W
                if phys_idx < phys_regs.len() {
                    let phys = phys_regs[phys_idx].to_string();
                    phys_idx += 1;
                    self.current_instructions.push(crate::ir::IROp::LoadPhys(phys.clone(), reg.clone()));
                    var_to_phys.push((var_name.clone(), phys));
                }
            } else if reg.starts_with('g') {
                // Global - emite LOAD do endereço global via RawLine
                if phys_idx < phys_regs.len() {
                    let phys = phys_regs[phys_idx].to_string();
                    phys_idx += 1;
                    let num: u32 = reg[1..].parse().unwrap_or(0);
                    self.current_instructions.push(crate::ir::IROp::RawLine(
                        format!("SET Z {};", num)
                    ));
                    self.current_instructions.push(crate::ir::IROp::RawLine(
                        format!("LOAD {}, Z;", phys)
                    ));
                    var_to_phys.push((var_name.clone(), phys));
                }
            } else {
                // Já é um registrador físico ou literal numérico
                var_to_phys.push((var_name.clone(), reg));
            }
        }

        // Substitui as variáveis no template com os destinos resolvidos
        let mut result = asm_text.clone();
        for (var_name, phys) in &var_to_phys {
            result = result.replace(&format!("{{{}}}", var_name), phys);
        }

        // Emite o asm resultado como RawLine(s)
        for line in result.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                // Normaliza a instrução
                if !trimmed.ends_with(';') {
                    self.emit_raw(&format!("{};", trimmed));
                } else {
                    self.emit_raw(trimmed);
                }
            }
        }
    }

    pub fn visit_struct_decl(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let struct_name = inner.next().unwrap().as_str().to_string();

        let mut fields = Vec::new();
        let mut offsets = HashMap::new();
        let mut offset = 0usize;

        for field in inner {
            if field.as_rule() == Rule::struct_field {
                let mut f_inner = field.into_inner();
                let type_pair = f_inner.next().unwrap();
                let field_type = self.parse_type_decl(type_pair);
                let field_name = f_inner.next().unwrap().as_str().to_string();
                offsets.insert(field_name.clone(), offset);
                fields.push((field_name, field_type));
                offset += 1;
            }
        }

        self.ctx.structs.insert(struct_name, StructInfo { fields, offsets });
    }

    pub fn visit_field_assign(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let field_access = inner.next().unwrap();
        let expr = inner.next().unwrap();

        let (addr_reg, _field_type) = self.visit_field_access_addr(field_access);
        let (val_reg, _) = self.visit_expr(expr);

        self.emit("STORE", &[&addr_reg, &val_reg]);
        if val_reg.starts_with("_tmp") {
            self.ctx.free_register(&val_reg);
        }
        if addr_reg.starts_with("_tmp") {
            self.ctx.free_register(&addr_reg);
        }
    }

    pub fn visit_array_decl(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let type_name = inner.next().unwrap().as_str().to_string();
        let size_str = inner.next().unwrap().as_str();
        let size: u32 = size_str.parse().unwrap();
        let ident = inner.next().unwrap().as_str();

        let parsed_type = match type_name.as_str() {
            "int" => Type::Int,
            "float" => Type::Float,
            "string" => Type::String,
            "bool" => Type::Bool,
            "void" => Type::Void,
            _ => {
                if self.ctx.structs.contains_key(&type_name) {
                    Type::Struct(type_name)
                } else {
                    Type::Int
                }
            }
        };

        let elem_size = self.get_type_size(&parsed_type);
        let total_size = size * elem_size;

        let target_reg = if self.current_function_name.is_empty() {
            self.ctx.allocate_global_register(ident, Type::Array(Box::new(parsed_type)))
        } else {
            self.ctx.allocate_register(ident, Type::Array(Box::new(parsed_type)))
        };

        let heap_addr = self.ctx.heap_ptr;
        self.ctx.heap_ptr += total_size;
        self.emit("SET", &[&target_reg, &heap_addr.to_string()]);

        let counter = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        let addr = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        let zero = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        let size_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;

        self.emit("SET", &[&zero, "0"]);
        self.emit("SET", &[&counter, "0"]);
        self.emit("SET", &[&size_reg, &total_size.to_string()]);

        let loop_start = self.ctx.new_label("arr_init");
        let loop_end = self.ctx.new_label("arr_end");

        self.emit_label(&loop_start);
        self.emit("JEQ", &[&counter, &size_reg, &loop_end]);

        self.emit("SET", &[&addr, &target_reg]);
        self.emit("ADD", &[&addr, &counter]);
        self.emit("STORE", &[&addr, &zero]);

        let tmp = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        self.emit("SET", &[&tmp, "1"]);
        self.emit("ADD", &[&counter, &tmp]);
        self.emit("JMP", &[&loop_start]);
        self.emit_label(&loop_end);

        self.ctx.free_register(&tmp);
        self.ctx.free_register(&counter);
        self.ctx.free_register(&addr);
        self.ctx.free_register(&zero);
        self.ctx.free_register(&size_reg);
    }

    pub fn visit_var_decl(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner().next().unwrap().into_inner();
        let type_pair = inner.next().unwrap(); 
        let parsed_type = self.parse_type_decl(type_pair);
        let ident = inner.next().unwrap().as_str();
        let expr = inner.next().unwrap();

        let target_reg = if self.current_function_name.is_empty() {
            self.ctx.allocate_global_register(ident, parsed_type)
        } else {
            self.ctx.allocate_register(ident, parsed_type)
        };
        let (val_reg, _) = self.visit_expr(expr);
        
        self.emit("SET", &[&target_reg, &val_reg]);
        if val_reg.starts_with("_tmp") {
            self.ctx.free_register(&val_reg);
        }
    }

    pub fn visit_pointer_assign(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let deref = inner.next().unwrap();
        let ident = deref.into_inner().next().unwrap().as_str();
        let expr = inner.next().unwrap();

        let addr_reg = self.ctx.get_register(ident);
        let (val_reg, _) = self.visit_expr(expr);

        self.emit("STORE", &[&addr_reg, &val_reg]);
        if val_reg.starts_with("_tmp") {
            self.ctx.free_register(&val_reg);
        }
    }

    pub fn visit_assign(&mut self, pair: Pair<Rule>) {
        self.visit_assign_expr(pair.into_inner().next().unwrap());
    }
    
    pub fn visit_array_assign(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let array_access = inner.next().unwrap().into_inner();
        let mut aa_inner = array_access;
        let ident = aa_inner.next().unwrap().as_str();
        let index_expr = aa_inner.next().unwrap();
        
        let expr = inner.next().unwrap();

        let base_reg = self.ctx.get_register(ident);
        let (index_reg, _) = self.visit_expr(index_expr);
        let (val_reg, _) = self.visit_expr(expr);
        
        let addr_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&addr_reg, &base_reg]);
        self.emit("ADD", &[&addr_reg, &index_reg]);
        
        self.emit("STORE", &[&addr_reg, &val_reg]);
        
        if val_reg.starts_with("_tmp") { self.ctx.free_register(&val_reg); }
        if index_reg.starts_with("_tmp") { self.ctx.free_register(&index_reg); }
        self.ctx.free_register(&addr_reg);
    }

    pub fn visit_assign_expr(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let ident = inner.next().unwrap().as_str();
        let expr = inner.next().unwrap();

        let target_reg = self.ctx.get_register(ident);
        let (val_reg, _) = self.visit_expr(expr);
        
        self.emit("SET", &[&target_reg, &val_reg]);
        if val_reg.starts_with("_tmp") {
            self.ctx.free_register(&val_reg);
        }
    }

    pub fn visit_increment_stmt(&mut self, pair: Pair<Rule>) {
        self.visit_increment(pair.into_inner().next().unwrap());
    }

    pub fn visit_increment(&mut self, pair: Pair<Rule>) {
        let op = if pair.as_str().ends_with("++") { "++" } else { "--" };
        let mut inner = pair.into_inner();
        let ident = inner.next().unwrap().as_str();
        
        let target_reg = self.ctx.get_register(ident);
        let one_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&one_reg, "1"]);
        
        if op == "++" {
            self.emit("ADD", &[&target_reg, &one_reg]);
        } else {
            self.emit("SUB", &[&target_reg, &one_reg]);
        }
        
        self.ctx.free_register(&one_reg);
    }

    pub fn visit_macro(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str();
        let args_pair = inner.next();

        let mut arg_vals = Vec::new();
        if let Some(expr_list) = args_pair {
            for expr in expr_list.into_inner() {
                arg_vals.push(self.visit_expr(expr).0);
            }
        }

        match name {
            "print" => {
                let val = &arg_vals[0];
                self.emit("PRINT", &[val]);
            }
            "println" => {
                let val = &arg_vals[0];
                self.emit("PRINTLN", &[val]);
            }
            "video_update" => {
                self.emit("UPDATEGUI", &[]);
            }
            "video_draw_pixel" => {
                self.emit("DRAWPIXEL", &[&arg_vals[0], &arg_vals[1], &arg_vals[2]]);
            }
            _ => panic!("Macro desconhecida: {}", name),
        }

        for arg in arg_vals {
            if arg.starts_with("_tmp") {
                self.ctx.free_register(&arg);
            }
        }
    }
}
