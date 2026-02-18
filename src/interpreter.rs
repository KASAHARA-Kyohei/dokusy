use std::collections::HashMap;
use std::fmt;

use crate::ast::{
    BinaryOp, Expr, ExprKind, Function, Program, Span, Stmt, StmtKind, TypeName, UnaryOp,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
    Unit,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::String(v) => write!(f, "{}", v),
            Value::Unit => write!(f, "()"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl RuntimeError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            line: span.line,
            column: span.column,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[runtime] {} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub exit_code: i64,
    pub output: String,
}

#[derive(Debug, Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

enum ExecFlow {
    Continue,
    Return(Value),
}

pub struct Interpreter {
    functions: HashMap<String, Function>,
    scopes: Vec<HashMap<String, Binding>>,
    output: String,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            scopes: Vec::new(),
            output: String::new(),
        }
    }

    pub fn run_program(&mut self, program: Program) -> Result<ExecutionResult, RuntimeError> {
        self.functions.clear();
        self.scopes.clear();
        self.output.clear();

        for func in program.functions {
            if self.functions.contains_key(&func.name) {
                return Err(RuntimeError::new(
                    format!("duplicate function `{}`", func.name),
                    func.span,
                ));
            }
            self.functions.insert(func.name.clone(), func);
        }

        let main_span = if let Some(main_func) = self.functions.get("main") {
            main_func.span
        } else {
            return Err(RuntimeError::new("function `main` is not defined", Span::new(1, 1)));
        };

        let main_value = self.call_function("main", Vec::new(), main_span)?;
        let exit_code = match main_value {
            Value::Int(v) => v,
            _ => {
                return Err(RuntimeError::new(
                    "`main` must return i64",
                    main_span,
                ))
            }
        };

        Ok(ExecutionResult {
            exit_code,
            output: self.output.clone(),
        })
    }

    fn call_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        call_span: Span,
    ) -> Result<Value, RuntimeError> {
        if name == "print" {
            return self.call_builtin_print(args, call_span);
        }

        let func = self
            .functions
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("undefined function `{}`", name), call_span))?;

        if args.len() != func.params.len() {
            return Err(RuntimeError::new(
                format!(
                    "function `{}` expects {} args but got {}",
                    name,
                    func.params.len(),
                    args.len()
                ),
                call_span,
            ));
        }

        self.push_scope();
        for (param, arg) in func.params.iter().zip(args.into_iter()) {
            self.declare(&param.name, arg, false, param.span)?;
        }

        let flow = self.execute_stmt(&func.body)?;
        self.pop_scope();

        match flow {
            ExecFlow::Return(value) => {
                self.check_return_type(name, &func.return_type, &value, call_span)?;
                Ok(value)
            }
            ExecFlow::Continue => Err(RuntimeError::new(
                format!("function `{}` ended without `return`", name),
                func.span,
            )),
        }
    }

    fn check_return_type(
        &self,
        func_name: &str,
        ty: &TypeName,
        value: &Value,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let ok = matches!(
            (ty, value),
            (TypeName::I64, Value::Int(_))
                | (TypeName::Bool, Value::Bool(_))
                | (TypeName::String, Value::String(_))
                | (TypeName::Unit, Value::Unit)
        );

        if ok {
            Ok(())
        } else {
            Err(RuntimeError::new(
                format!("function `{}` returned wrong type", func_name),
                span,
            ))
        }
    }

    fn call_builtin_print(&mut self, args: Vec<Value>, call_span: Span) -> Result<Value, RuntimeError> {
        if args.len() != 1 {
            return Err(RuntimeError::new(
                "`print` expects exactly 1 argument",
                call_span,
            ));
        }
        let value = &args[0];
        self.output.push_str(&format!("{}\n", value));
        Ok(Value::Unit)
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<ExecFlow, RuntimeError> {
        match &stmt.kind {
            StmtKind::Let {
                name,
                mutable,
                value,
            } => {
                let evaluated = self.eval_expr(value)?;
                self.declare(name, evaluated, *mutable, stmt.span)?;
                Ok(ExecFlow::Continue)
            }
            StmtKind::Assign { name, value } => {
                let evaluated = self.eval_expr(value)?;
                self.assign(name, evaluated, stmt.span)?;
                Ok(ExecFlow::Continue)
            }
            StmtKind::ExprStmt(expr) => {
                self.eval_expr(expr)?;
                Ok(ExecFlow::Continue)
            }
            StmtKind::Block(stmts) => {
                self.push_scope();
                for inner in stmts {
                    let flow = self.execute_stmt(inner)?;
                    if let ExecFlow::Return(value) = flow {
                        self.pop_scope();
                        return Ok(ExecFlow::Return(value));
                    }
                }
                self.pop_scope();
                Ok(ExecFlow::Continue)
            }
            StmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_value = self.eval_expr(cond)?;
                let cond_bool = self.expect_bool(cond_value, cond.span, "if condition must be bool")?;

                if cond_bool {
                    self.execute_stmt(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.execute_stmt(else_stmt)
                } else {
                    Ok(ExecFlow::Continue)
                }
            }
            StmtKind::While { cond, body } => {
                loop {
                    let cond_value = self.eval_expr(cond)?;
                    let cond_bool =
                        self.expect_bool(cond_value, cond.span, "while condition must be bool")?;
                    if !cond_bool {
                        break;
                    }

                    let flow = self.execute_stmt(body)?;
                    if let ExecFlow::Return(value) = flow {
                        return Ok(ExecFlow::Return(value));
                    }
                }
                Ok(ExecFlow::Continue)
            }
            StmtKind::Return(expr) => {
                let value = self.eval_expr(expr)?;
                Ok(ExecFlow::Return(value))
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match &expr.kind {
            ExprKind::Int(v) => Ok(Value::Int(*v)),
            ExprKind::Bool(v) => Ok(Value::Bool(*v)),
            ExprKind::String(v) => Ok(Value::String(v.clone())),
            ExprKind::Var(name) => self.lookup(name, expr.span),
            ExprKind::Unary { op, expr: inner } => {
                let value = self.eval_expr(inner)?;
                match op {
                    UnaryOp::Not => {
                        let v = self.expect_bool(value, expr.span, "`!` expects bool")?;
                        Ok(Value::Bool(!v))
                    }
                    UnaryOp::Neg => {
                        let v = self.expect_int(value, expr.span, "unary `-` expects i64")?;
                        Ok(Value::Int(-v))
                    }
                }
            }
            ExprKind::Binary { left, op, right } => self.eval_binary(left, *op, right, expr.span),
            ExprKind::Call { name, args } => {
                let mut evaluated = Vec::with_capacity(args.len());
                for arg in args {
                    evaluated.push(self.eval_expr(arg)?);
                }
                self.call_function(name, evaluated, expr.span)
            }
        }
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        match op {
            BinaryOp::And => {
                let lv = self.eval_expr(left)?;
                let lb = self.expect_bool(lv, span, "`&&` expects bool operands")?;
                if !lb {
                    return Ok(Value::Bool(false));
                }
                let rv = self.eval_expr(right)?;
                let rb = self.expect_bool(rv, span, "`&&` expects bool operands")?;
                Ok(Value::Bool(rb))
            }
            BinaryOp::Or => {
                let lv = self.eval_expr(left)?;
                let lb = self.expect_bool(lv, span, "`||` expects bool operands")?;
                if lb {
                    return Ok(Value::Bool(true));
                }
                let rv = self.eval_expr(right)?;
                let rb = self.expect_bool(rv, span, "`||` expects bool operands")?;
                Ok(Value::Bool(rb))
            }
            _ => {
                let left_value = self.eval_expr(left)?;
                let right_value = self.eval_expr(right)?;
                match op {
                    BinaryOp::Add => match (left_value, right_value) {
                        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
                        (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                        _ => Err(RuntimeError::new(
                            "`+` expects (i64, i64) or (string, string)",
                            span,
                        )),
                    },
                    BinaryOp::Sub => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`-` expects i64")?;
                        Ok(Value::Int(l - r))
                    }
                    BinaryOp::Mul => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`*` expects i64")?;
                        Ok(Value::Int(l * r))
                    }
                    BinaryOp::Div => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`/` expects i64")?;
                        if r == 0 {
                            return Err(RuntimeError::new("division by zero", span));
                        }
                        Ok(Value::Int(l / r))
                    }
                    BinaryOp::Eq => Ok(Value::Bool(values_equal(&left_value, &right_value))),
                    BinaryOp::Ne => Ok(Value::Bool(!values_equal(&left_value, &right_value))),
                    BinaryOp::Lt => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`<` expects i64")?;
                        Ok(Value::Bool(l < r))
                    }
                    BinaryOp::Le => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`<=` expects i64")?;
                        Ok(Value::Bool(l <= r))
                    }
                    BinaryOp::Gt => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`>` expects i64")?;
                        Ok(Value::Bool(l > r))
                    }
                    BinaryOp::Ge => {
                        let (l, r) = self.expect_int_pair(left_value, right_value, span, "`>=` expects i64")?;
                        Ok(Value::Bool(l >= r))
                    }
                    BinaryOp::And | BinaryOp::Or => unreachable!(),
                }
            }
        }
    }

    fn expect_bool(
        &self,
        value: Value,
        span: Span,
        message: &str,
    ) -> Result<bool, RuntimeError> {
        if let Value::Bool(v) = value {
            Ok(v)
        } else {
            Err(RuntimeError::new(message, span))
        }
    }

    fn expect_int(&self, value: Value, span: Span, message: &str) -> Result<i64, RuntimeError> {
        if let Value::Int(v) = value {
            Ok(v)
        } else {
            Err(RuntimeError::new(message, span))
        }
    }

    fn expect_int_pair(
        &self,
        left: Value,
        right: Value,
        span: Span,
        message: &str,
    ) -> Result<(i64, i64), RuntimeError> {
        match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok((l, r)),
            _ => Err(RuntimeError::new(message, span)),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(
        &mut self,
        name: &str,
        value: Value,
        mutable: bool,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or_else(|| RuntimeError::new("internal error: no scope", span))?;

        if scope.contains_key(name) {
            return Err(RuntimeError::new(
                format!("variable `{}` already declared in this scope", name),
                span,
            ));
        }

        scope.insert(name.to_string(), Binding { value, mutable });
        Ok(())
    }

    fn assign(&mut self, name: &str, value: Value, span: Span) -> Result<(), RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(binding) = scope.get_mut(name) {
                if !binding.mutable {
                    return Err(RuntimeError::new(
                        format!("cannot assign to immutable variable `{}`", name),
                        span,
                    ));
                }
                binding.value = value;
                return Ok(());
            }
        }

        Err(RuntimeError::new(
            format!("undefined variable `{}`", name),
            span,
        ))
    }

    fn lookup(&self, name: &str, span: Span) -> Result<Value, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Ok(binding.value.clone());
            }
        }

        Err(RuntimeError::new(
            format!("undefined variable `{}`", name),
            span,
        ))
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(l), Value::Int(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Unit, Value::Unit) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    use super::Interpreter;

    fn run(src: &str) -> Result<i64, String> {
        let tokens = Lexer::new(src)
            .tokenize()
            .map_err(|e| format!("{}", e))?;
        let program = Parser::new(tokens)
            .parse_program()
            .map_err(|e| format!("{}", e))?;
        let mut interpreter = Interpreter::new();
        let result = interpreter
            .run_program(program)
            .map_err(|e| format!("{}", e))?;
        Ok(result.exit_code)
    }

    #[test]
    fn evaluates_arithmetic_precedence() {
        let src = "fn main() -> i64 { return 1 + 2 * 3; }";
        let code = run(src).expect("must run");
        assert_eq!(code, 7);
    }

    #[test]
    fn evaluates_if_and_while() {
        let src = r#"
            fn main() -> i64 {
                let mut sum = 0;
                let mut i = 0;
                while i < 5 {
                    sum = sum + i;
                    i = i + 1;
                }

                if sum == 10 {
                    return 1;
                } else {
                    return 0;
                }
            }
        "#;

        let code = run(src).expect("must run");
        assert_eq!(code, 1);
    }

    #[test]
    fn calls_function() {
        let src = r#"
            fn add(a: i64, b: i64) -> i64 {
                return a + b;
            }

            fn main() -> i64 {
                return add(2, 3);
            }
        "#;
        let code = run(src).expect("must run");
        assert_eq!(code, 5);
    }

    #[test]
    fn errors_on_assign_to_immutable() {
        let src = r#"
            fn main() -> i64 {
                let x = 1;
                x = 2;
                return x;
            }
        "#;

        let err = run(src).expect_err("must fail");
        assert!(err.contains("immutable variable"));
    }
}
