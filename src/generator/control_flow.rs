use super::CodeGenerator;
use pest::iterators::Pair;
use crate::Rule;

impl CodeGenerator {
    pub fn visit_block(&mut self, pair: Pair<Rule>) {
        for stmt in pair.into_inner() {
            self.visit_statement(stmt);
        }
    }

    pub fn visit_if(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let cond_expr = inner.next().unwrap();
        let block_true = inner.next().unwrap();
        let block_else = inner.next();

        let (left, _) = self.visit_expr(cond_expr);

        let label_true = self.ctx.new_label("if_true");
        let label_false = self.ctx.new_label("if_false");
        let label_end = self.ctx.new_label("if_end");

        self.emit("JNE", &[&left, "0", &label_true]);
        self.emit("JMP", &[&label_false]);

        if left.starts_with("_tmp") {
            self.ctx.free_register(&left);
        }

        self.emit_label(&label_true);
        self.visit_block(block_true);
        self.emit("JMP", &[&label_end]);

        self.emit_label(&label_false);
        if let Some(el) = block_else {
            self.visit_block(el);
        }
        
        self.emit_label(&label_end);
    }

    pub fn visit_while(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        let cond_expr = inner.next().unwrap();
        let block = inner.next().unwrap();

        let label_start = self.ctx.new_label("while_start");
        let label_body = self.ctx.new_label("while_body");
        let label_end = self.ctx.new_label("while_end");

        self.emit_label(&label_start);
        
        let (left, _) = self.visit_expr(cond_expr);
        self.emit("JNE", &[&left, "0", &label_body]);
        self.emit("JMP", &[&label_end]);

        if left.starts_with("_tmp") {
            self.ctx.free_register(&left);
        }

        self.emit_label(&label_body);
        self.ctx.push_loop(label_start.clone(), label_end.clone());
        self.visit_block(block);
        self.ctx.pop_loop();
        
        self.emit("JMP", &[&label_start]);
        self.emit_label(&label_end);
    }

    pub fn visit_for(&mut self, pair: Pair<Rule>) {
        let mut inner = pair.into_inner();
        
        let mut next_pair = inner.next().unwrap();
        
        if next_pair.as_rule() == Rule::var_decl {
            self.visit_var_decl(next_pair);
            next_pair = inner.next().unwrap();
        } else if next_pair.as_rule() == Rule::assign {
            self.visit_assign(next_pair);
            next_pair = inner.next().unwrap();
        }
        
        let cond_expr = next_pair;
        
        next_pair = inner.next().unwrap();
        
        let mut step_pair = None;
        let block_pair;
        
        if next_pair.as_rule() == Rule::block {
            block_pair = next_pair;
        } else {
            step_pair = Some(next_pair);
            block_pair = inner.next().unwrap();
        }
        
        let label_start = self.ctx.new_label("for_start");
        let label_body = self.ctx.new_label("for_body");
        let label_end = self.ctx.new_label("for_end");

        self.emit_label(&label_start);
        
        let (left, _) = self.visit_expr(cond_expr.clone());
        self.emit("JNE", &[&left, "0", &label_body]);
        self.emit("JMP", &[&label_end]);

        if left.starts_with("_tmp") {
            self.ctx.free_register(&left);
        }

        self.emit_label(&label_body);
        self.ctx.push_loop(label_start.clone(), label_end.clone());
        self.visit_block(block_pair);
        self.ctx.pop_loop();
        
        if let Some(step) = step_pair {
            if step.as_rule() == Rule::assign_expr {
                self.visit_assign_expr(step);
            } else if step.as_rule() == Rule::increment {
                self.visit_increment(step);
            }
        }
        
        self.emit("JMP", &[&label_start]);
        self.emit_label(&label_end);
    }
}
