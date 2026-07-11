use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Array(Box<Type>),
}

#[derive(Clone)]
pub struct VariableInfo {
    pub reg: String,
    pub var_type: Type,
}

pub struct CompilationContext {
    pub variables: HashMap<String, VariableInfo>,
    pub label_counter: usize,
    pub loop_stack: Vec<(String, String)>,
    pub next_register: usize,
    pub registers: Vec<String>,
}

impl CompilationContext {
    pub fn new() -> Self {
        let mut registers = Vec::new();
        for c in b'A'..=b'Z' {
            registers.push((c as char).to_string());
        }

        CompilationContext {
            variables: HashMap::new(),
            label_counter: 0,
            loop_stack: Vec::new(),
            next_register: 0, // 'A'
            registers,
        }
    }

    pub fn allocate_register(&mut self, var_name: &str, var_type: Type) -> String {
        if let Some(info) = self.variables.get(var_name) {
            return info.reg.clone();
        }
        
        let reg = self.next_free_register();
        self.variables.insert(var_name.to_string(), VariableInfo {
            reg: reg.clone(),
            var_type,
        });
        reg
    }

    pub fn free_register(&mut self, var_name: &str) {
        if let Some(info) = self.variables.remove(var_name) {
            // Find index of this reg
            if let Some(pos) = self.registers.iter().position(|x| x == &info.reg) {
                // If this was the last allocated register, we can walk back next_register
                // (Very simplistic register allocation, but works for our linear case)
                if pos + 1 == self.next_register {
                    self.next_register -= 1;
                }
            }
        }
    }

    pub fn get_register(&self, var_name: &str) -> String {
        self.variables.get(var_name)
            .expect(&format!("Variável '{}' não foi declarada!", var_name))
            .reg.clone()
    }
    
    pub fn get_type(&self, var_name: &str) -> Type {
        if var_name.starts_with("_tmp") {
            return Type::Int; // simplificação
        }
        self.variables.get(var_name)
            .expect(&format!("Variável '{}' não foi declarada!", var_name))
            .var_type.clone()
    }
    
    pub fn get_active_registers(&self) -> Vec<String> {
        self.registers[0..self.next_register].to_vec()
    }
    
    pub fn reset_for_function(&mut self) {
        self.variables.clear();
        self.next_register = 0;
    }

    fn next_free_register(&mut self) -> String {
        // Reserve 'Y' for scratch, 'Z' for return
        if self.next_register >= 24 {
            panic!("Erro: Faltam registradores livres (Max 24)");
        }
        let reg = self.registers[self.next_register].clone();
        self.next_register += 1;
        reg
    }

    pub fn new_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("{}_{}", prefix, self.label_counter)
    }

    pub fn push_loop(&mut self, continue_label: String, break_label: String) {
        self.loop_stack.push((continue_label, break_label));
    }

    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }
}
