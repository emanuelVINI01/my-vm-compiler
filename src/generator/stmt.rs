use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;
use crate::context::Type;

impl CodeGenerator {
    pub fn visit_array_decl(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let _type_decl = inner.next().unwrap();
        let ident = inner.next().unwrap().as_str();
        let size_str = inner.next().unwrap().as_str();
        let _size: u32 = size_str.parse().unwrap();
        
        let target_reg = self.ctx.allocate_register(ident, Type::Array(Box::new(Type::Int)));
        
        self.emit("GETLASTADDR", &[&target_reg]);
        let tmp = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        self.emit("SET", &[&tmp, "1"]);
        self.emit("ADD", &[&target_reg, &tmp]);
        
        let loop_start = self.ctx.new_label("arr_init");
        let loop_end = self.ctx.new_label("arr_end");
        let counter = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        let addr = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        
        self.emit("SET", &[&counter, "0"]);
        self.emit_label(&loop_start);
        self.emit("JEQ", &[&counter, &size_str, &loop_end]);
        self.emit("SET", &[&addr, &target_reg]);
        self.emit("ADD", &[&addr, &counter]);
        self.emit("STORE", &[&addr, &counter]);
        
        let zero = self.ctx.allocate_register(&format!("_tmp_{}", self.ctx.label_counter), Type::Int);
        self.ctx.label_counter += 1;
        self.emit("SET", &[&zero, "0"]);
        self.emit("STORE", &[&addr, &zero]);
        
        self.emit("ADD", &[&counter, &tmp]);
        self.emit("JMP", &[&loop_start]);
        self.emit_label(&loop_end);
        
        self.ctx.free_register(&tmp);
        self.ctx.free_register(&counter);
        self.ctx.free_register(&addr);
        self.ctx.free_register(&zero);
    }

    pub fn visit_var_decl(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner().next().unwrap().into_inner();
        let type_pair = inner.next().unwrap(); 
        let parsed_type = self.parse_type_decl(type_pair);
        let ident = inner.next().unwrap().as_str();
        let expr = inner.next().unwrap();

        let target_reg = self.ctx.allocate_register(ident, parsed_type);
        let (val_reg, _) = self.visit_expr(expr);
        
        self.emit("SET", &[&target_reg, &val_reg]);
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
                if val.starts_with("_tmp") || val.len() == 1 {
                    self.emit("PRINT", &[val]);
                } else {
                    self.emit("WRITESTR", &[&format!("\"{}\"", val)]);
                }
            }
            "println" => {
                let val = &arg_vals[0];
                if val.starts_with("_tmp") || val.len() == 1 {
                    self.emit("PRINTLN", &[val]);
                } else {
                    self.emit("WRITESTR", &[&format!("\"{}\"", val)]);
                    self.emit("PRINTLN", &["\"\""]);
                }
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
