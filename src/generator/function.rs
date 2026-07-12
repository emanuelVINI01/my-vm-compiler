use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;
use crate::context::Type;

impl CodeGenerator {
    pub fn visit_function(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let first = inner.next().unwrap();

        let is_interrupt;
        let type_decl_pair;
        if first.as_rule() == Rule::interrupt_mod {
            is_interrupt = true;
            type_decl_pair = inner.next().unwrap();
        } else {
            is_interrupt = false;
            type_decl_pair = first;
        }

        let _type_decl = self.parse_type_decl(type_decl_pair);
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
            next_pair = inner.next().unwrap();
        }
        
        let just_names: Vec<String> = param_names.iter().map(|(n, _)| n.clone()).collect();
        self.functions_info.insert(name.to_string(), just_names);

        let prev_function = self.current_function_name.clone();
        self.current_function_name = name.to_string();
        let prev_registers = self.ctx.variables.clone();
        
        let prev_instructions = std::mem::take(&mut self.current_instructions);

        self.ctx.reset_for_function();
        
        if name != "main" && !param_names.is_empty() {
            self.emit("POP", &["Z"]);
            
            for (p_name, p_type) in param_names.iter().rev() {
                let reg = self.ctx.allocate_register(p_name, p_type.clone());
                self.emit("POP", &[&reg]);
            }
            
            self.emit("PUSH", &["Z"]);
        }
        
        self.visit_block(next_pair);
        
        if name == "main" {
            self.emit("HALT", &[]);
        } else {
            self.emit("RET", &[]);
        }
        
        let func_instructions = std::mem::take(&mut self.current_instructions);
        self.program.functions.push(crate::ir::IRFunction {
            name: name.to_string(),
            instructions: func_instructions,
        });

        self.ctx.variables = prev_registers;
        self.current_function_name = prev_function;
        self.current_instructions = prev_instructions;
    }

    pub fn visit_func_call(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let func_call_expr = inner.next().unwrap();
        let mut cie_inner = func_call_expr.into_inner();
        let name = cie_inner.next().unwrap().as_str();
        let args_pair = cie_inner.next();
        
        let mut arg_vals = Vec::new();
        if let Some(expr_list) = args_pair {
            for expr in expr_list.into_inner() {
                arg_vals.push(self.visit_expr(expr).0);
            }
        }
        
        // Push os argumentos na ordem certa (o callee popa ao contrario)
        for arg in &arg_vals {
            self.emit("PUSH", &[arg]);
            if arg.starts_with("_tmp") {
                self.ctx.free_register(arg);
            }
        }
        
        self.emit("CALL", &[name]);
    }
    
    pub fn visit_func_call_expr(&mut self, pair: Pair<Rule>) -> (String, Type) {
        let mut inner = pair.into_inner();
        let name = inner.next().unwrap().as_str();
        let args_pair = inner.next();

        let mut arg_vals = Vec::new();
        if let Some(expr_list) = args_pair {
            for expr in expr_list.into_inner() {
                arg_vals.push(self.visit_expr(expr).0);
            }
        }
        
        for arg in &arg_vals {
            self.emit("PUSH", &[arg]);
            if arg.starts_with("_tmp") {
                self.ctx.free_register(arg);
            }
        }
        
        self.emit("CALL", &[name]);

        let res_reg = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&res_reg, "0"]);
        self.emit("ADD", &[&res_reg, "Z"]);
        
        (res_reg, Type::Int)
    }
}
