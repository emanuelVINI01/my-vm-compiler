use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;
use crate::context::Type;

impl CodeGenerator {
    pub fn visit_function(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let _type_decl = inner.next().unwrap();
        let name = inner.next().unwrap().as_str();
        
        let mut param_names = Vec::new();
        let mut next_pair = inner.next().unwrap();
        
        if next_pair.as_rule() == Rule::param_list {
            let mut param_inner = next_pair.into_inner();
            while let Some(type_pair) = param_inner.next() {
                let p_type = self.parse_type_decl(type_pair);
                let p_name = param_inner.next().unwrap().as_str();
                param_names.push((p_name.to_string(), p_type));
            }
            next_pair = inner.next().unwrap(); // this will be the block
        }
        
        let just_names: Vec<String> = param_names.iter().map(|(n, _)| n.clone()).collect();
        self.functions.insert(name.to_string(), just_names);
        self.current_function = name.to_string();
        
        self.ctx.reset_for_function();
        
        self.emit_label(name);
        
        if name != "main" && !param_names.is_empty() {
            // Guarda PC do retorno
            self.emit("POP", &["Z"]);
            
            for (p_name, p_type) in param_names.iter().rev() {
                let reg = self.ctx.allocate_register(p_name, p_type.clone());
                self.emit("POP", &[&reg]);
            }
            
            // Poe PC de volta
            self.emit("PUSH", &["Z"]);
        }
        
        // The block is next_pair
        self.visit_block(next_pair);
        
        if name == "main" {
            self.emit("HALT", &[]);
        } else {
            self.emit("RET", &[]);
        }
    }

    pub fn visit_func_call(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str();
        let args_pair = inner.next(); // expr_list
        
        let mut arg_vals = Vec::new();
        if let Some(expr_list) = args_pair {
            for expr in expr_list.into_inner() {
                arg_vals.push(self.visit_expr(expr).0);
            }
        }
        
        let mut active_regs = self.ctx.get_active_registers();
        for reg in &active_regs {
            self.emit("PUSH", &[reg]);
        }
        
        // Push os argumentos na ordem certa (o callee popa ao contrario)
        for arg in &arg_vals {
            self.emit("PUSH", &[arg]);
            if arg.starts_with("_tmp") {
                self.ctx.free_register(arg);
                active_regs.retain(|r| r != arg);
            }
        }
        
        self.emit("CALL", &[name]);
        
        for reg in active_regs.iter().rev() {
            self.emit("POP", &[reg]);
        }
    }
    
    pub fn visit_func_call_expr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        // Aproveita a logica do func_call (que eh um statement)
        self.visit_func_call(pair);
        
        // Retorno ta no Z, move pro tmp register
        let res_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&res_reg, "0"]);
        self.emit("ADD", &[&res_reg, "Z"]);
        
        (res_reg, Type::Int)
    }
}
