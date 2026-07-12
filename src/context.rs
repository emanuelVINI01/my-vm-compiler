use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Void,
    Array(Box<Type>),
    Pointer(Box<Type>),
    Struct(String),
}

#[derive(Clone)]
pub struct VariableInfo {
    pub reg: String,
    pub var_type: Type,
}

pub struct CompilationContext {
    pub variables: HashMap<String, VariableInfo>,
    pub global_variables: HashMap<String, VariableInfo>,
    pub label_counter: usize,
    pub loop_stack: Vec<(String, String)>,
    pub next_register: usize,
    pub next_global_register: usize,
    pub registers: Vec<String>,
    pub free_registers: Vec<usize>,
    pub structs: HashMap<String, StructInfo>,
    pub heap_ptr: u32,
}

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub fields: Vec<(String, Type)>,
    pub offsets: HashMap<String, usize>,
}

impl CompilationContext {
    pub fn new() -> Self {
        let mut registers = Vec::new();
        for c in b'A'..=b'Z' {
            registers.push((c as char).to_string());
        }

        CompilationContext {
            variables: HashMap::new(),
            global_variables: HashMap::new(),
            label_counter: 0,
            loop_stack: Vec::new(),
            next_register: 0,
            next_global_register: 0,
            registers,
            free_registers: Vec::new(),
            structs: HashMap::new(),
            heap_ptr: 1024,
        }
    }

    pub fn allocate_register(&mut self, var_name: &str, var_type: Type) -> String {
        if let Some(info) = self.variables.get(var_name) {
            return info.reg.clone();
        }
        
        let reg = self.next_virtual_register();
        self.variables.insert(var_name.to_string(), VariableInfo {
            reg: reg.clone(),
            var_type,
        });
        reg
    }
    
    pub fn allocate_global_register(&mut self, var_name: &str, var_type: Type) -> String {
        let reg = format!("g{}", self.next_global_register);
        self.next_global_register += 1;
        self.global_variables.insert(var_name.to_string(), VariableInfo {
            reg: reg.clone(),
            var_type,
        });
        reg
    }

    pub fn free_register(&mut self, _var_name: &str) {
        // No IR, a gente não reutiliza registradores virtuais tão cedo, 
        // e o alocador futuro cuidará disso, então `free_register` vira um no-op.
    }

    pub fn get_register(&self, var_name: &str) -> String {
        if let Some(info) = self.variables.get(var_name) {
            return info.reg.clone();
        }
        if let Some(info) = self.global_variables.get(var_name) {
            return info.reg.clone();
        }
        panic!("Variável '{}' não foi declarada!", var_name);
    }
    
    pub fn get_type(&self, var_name: &str) -> Type {
        if var_name.starts_with("v_") {
            return Type::Int; // tmp regs
        }
        if let Some(info) = self.variables.get(var_name) {
            return info.var_type.clone();
        }
        if let Some(info) = self.global_variables.get(var_name) {
            return info.var_type.clone();
        }
        panic!("Variável '{}' não foi declarada!", var_name);
    }
    
    pub fn get_active_registers(&self) -> Vec<String> {
        let mut all = Vec::new();
        for (_, info) in &self.global_variables {
            all.push(info.reg.clone());
        }
        for (_, info) in &self.variables {
            all.push(info.reg.clone());
        }
        all.sort();
        all.dedup();
        all
    }
    
    pub fn reset_for_function(&mut self) {
        self.variables.clear();
        // Não resetamos next_register pq os Virtuais devem ser unicos globais (opcional)
    }

    fn next_virtual_register(&mut self) -> String {
        let reg = format!("v{}", self.next_register);
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
