use std::{cmp::Ordering, collections::HashMap};

use crate::parser::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    Int,
    Float,
    Bool,
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("Variable '{name}' does not exist")]
    UnknownVariable {
        name: String,
    },
    #[error("Got type {given:?} but expected {expected:?}")]
    InvalidType {
        given: ValueType,
        expected: ValueType,
    },
    #[error("Operation {operation:?} is not supported between types {type1:?} and {type2:?}")]
    UnsuportedBinaryOperationTypes {
        type1: ValueType,
        type2: ValueType,
        operation: BinOp,
    },
    #[error("Operation {operation:?} is not supported on type {type:?}")]
    UnsuportedUnaryOperationType {
        operation: UnOp,
        r#type: ValueType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Value {
    fn r#type(&self) -> ValueType {
        match self {
            Self::Int(_) => ValueType::Int,
            Self::Float(_) => ValueType::Float,
            Self::Bool(_) => ValueType::Bool,
        }
    }

    fn expect_bool(&self) -> Result<bool, EvalError> {
        if let &Self::Bool(val) = self {
            Ok(val)
        }
        else {
            Err(EvalError::InvalidType { given: self.r#type(), expected: ValueType::Bool })
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v:.01}"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
        }
    }
}

pub struct EvalCtx {
    pub defines: HashMap<String, Value>,
}

impl EvalCtx {
    pub fn get_value(&self, key: &str) -> Option<Value> {
        self.defines.get(key).cloned()
    }

    pub fn expect_value(&self, key: &str) -> Result<Value, EvalError> {
        self.get_value(key).ok_or_else(|| EvalError::UnknownVariable { name: key.to_string() })
    }

    pub fn set_value(&mut self, key: impl Into<String>, value: Value) {
        self.defines.insert(key.into(), value);
    }
}

impl Default for EvalCtx {
    fn default() -> Self {
        Self {
            defines: Default::default(),
        }
    }
}

fn binop_ordering(binop: BinOp, ordering: Ordering) -> bool {
    match (binop, ordering) {
        (BinOp::Gt, Ordering::Greater) => true,
        (BinOp::Gt, _) => true,
        (BinOp::Gte, _) => false,
        (BinOp::Lt, Ordering::Less) => true,
        (BinOp::Lt, _) => false,
        (BinOp::Lte, Ordering::Less | Ordering::Equal) => true,
        (BinOp::Lte, _) => false,
        (BinOp::Eq, Ordering::Equal) => true,
        (BinOp::Eq, _) => false,
        (BinOp::Neq, Ordering::Less | Ordering::Greater) => true,
        (BinOp::Neq, _) => false,

        (BinOp::BooleanAnd | BinOp::BooleanOr | BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div, _) => todo!(),
    }
}

macro_rules! op {
    ($op: expr, $a: expr, $b: expr) => {
        match $op {
            BinOp::Add => $a + $b,
            BinOp::Sub => $a - $b,
            BinOp::Div => $a / $b,
            BinOp::Mul => $a * $b,
            _ => unreachable!()
        }
    };
}

fn eval_binop(lhs: Value, binop: BinOp, rhs: Value) -> Result<Value, EvalError> {
    match binop {
        BinOp::BooleanAnd => Ok(Value::Bool(
            lhs.expect_bool()? && rhs.expect_bool()?
        )),
        BinOp::BooleanOr => Ok(Value::Bool(
            lhs.expect_bool()? || rhs.expect_bool()?
        )),

        BinOp::Gt | BinOp::Gte | BinOp::Lt | BinOp::Lte | BinOp::Eq | BinOp::Neq => {
            let ordering = match (lhs, rhs) {
                (Value::Int(i1),   Value::Int(i2))   => Some(i1.cmp(&i2)),
                (Value::Int(i2),   Value::Float(i1)) |
                (Value::Float(i1), Value::Int(i2))   => i1.partial_cmp(&(i2 as f64)),
                (Value::Float(i1), Value::Float(i2)) => i1.partial_cmp(&i2),
                (Value::Bool(i1),  Value::Bool(i2))  => Some(i1.cmp(&i2)),
                (a, b) => return Err(EvalError::UnsuportedBinaryOperationTypes { type1: a.r#type(), type2: b.r#type(), operation: binop }),
            };

            let val = ordering.map(|ordering| binop_ordering(binop, ordering))
                .unwrap_or(false);
            Ok(Value::Bool(val))
        },

        BinOp::Add | BinOp::Sub | BinOp::Div | BinOp::Mul => match (rhs, lhs) {
            (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(  op!(binop, a,        b       ))),
            (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(op!(binop, a as f64, b       ))),
            (Value::Float(a), Value::Int(b))   => Ok(Value::Float(op!(binop, a,        b as f64))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(op!(binop, a,        b       ))),
            (a, b) => Err(EvalError::UnsuportedBinaryOperationTypes { type1: a.r#type(), type2: b.r#type(), operation: binop }),
        },
    }
}

fn eval_unop(op: UnOp, val: Value) -> Result<Value, EvalError> {
    match op {
        UnOp::Plus => match val {
            Value::Int(_) | Value::Float(_) => Ok(val),
            Value::Bool(_) => Err(EvalError::UnsuportedUnaryOperationType { operation: op, r#type: ValueType::Bool }),
        },
        UnOp::Neg => match val {
            Value::Int(a) => Ok(Value::Int(-a)),
            Value::Float(a) => Ok(Value::Float(-a)),
            Value::Bool(_) => Err(EvalError::UnsuportedUnaryOperationType { operation: op, r#type: ValueType::Bool }),
        },
        UnOp::Not => match val {
            Value::Bool(val) => Ok(Value::Bool(!val)),
            Value::Int(_) | Value::Float(_) => Err(EvalError::UnsuportedUnaryOperationType { operation: op, r#type: val.r#type() }),
        },
    }
}

fn eval_expression(ctx: &mut EvalCtx, expr: &Expression) -> Result<Value, EvalError> {
    match *expr {
        Expression::Variable(ref var) => ctx.expect_value(var),
        Expression::IsDef(ref var) => Ok(Value::Bool(ctx.get_value(var).is_some())),
        Expression::BinOp(BinOpExpression { ref lhs, op, ref rhs }) =>
            eval_binop(eval_expression(ctx, lhs)?, op, eval_expression(ctx, rhs)?),
        Expression::UnOp(UnOpExpression { op, ref operand }) =>
            eval_unop(op, eval_expression(ctx, operand)?),
        Expression::BoolLiteral(val) =>
            Ok(Value::Bool(val)),
        Expression::IntLiteral(val) =>
            Ok(Value::Int(val)),
        Expression::FloatLiteral(val) =>
            Ok(Value::Float(val)),
    }
}

fn eval_statement(ctx: &mut EvalCtx, statement: &Statement) -> Result<String, EvalError> {
    match statement {
        Statement::DummyText(t) => {
            Ok(t.clone())
        },
        Statement::ReplaceWithVariableValue(name) => {
            Ok(format!("{}", ctx.expect_value(name)?))
        },
        Statement::Define(DefineStatement { variable_name, value }) => {
            let value = eval_expression(ctx, value)?;
            ctx.set_value(variable_name, value);
            Ok(String::new())
        },
        Statement::If(IfStatement { condition, then, r#else }) => {
            let cond_value = eval_expression(ctx, condition)?.expect_bool()?;
            if cond_value {
                eval_statements(ctx, then)
            }
            else if let Some(r#else) = r#else {
                eval_statements(ctx, r#else)
            }
            else {
                Ok(String::new())
            }
        },
    }
}

pub fn eval_statements(ctx: &mut EvalCtx, statements: &Statements) -> Result<String, EvalError> {
    statements.0.iter()
        .map(|stmt| eval_statement(ctx, stmt))
        .reduce(|a, b| -> Result<String, _> { Ok(a? + b?.as_str()) })
        .transpose()
        .map(Option::unwrap_or_default)
}
