//! # SQL Abstract Syntax Tree (AST)
//!
//! This module defines the abstract syntax tree representation for SQL queries
//! in StreamWeave. The AST is produced by the SQL parser and used by the
//! query translator to build StreamWeave pipelines.
//!
//! ## AST Structure
//!
//! The AST represents SQL queries as a tree of nodes, where each node
//! corresponds to a SQL construct (SELECT, WHERE, GROUP BY, etc.).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Root node of a SQL query AST
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SqlQuery {
  /// The SELECT clause
  pub select: SelectClause,
  /// The FROM clause (stream source)
  pub from: FromClause,
  /// Optional WHERE clause
  pub where_clause: Option<WhereClause>,
  /// Optional GROUP BY clause
  pub group_by: Option<GroupByClause>,
  /// Optional WINDOW clause
  pub window: Option<WindowClause>,
  /// Optional JOIN clause
  pub join: Option<JoinClause>,
  /// Optional ORDER BY clause
  pub order_by: Option<OrderByClause>,
  /// Optional LIMIT clause
  pub limit: Option<LimitClause>,
}

/// SELECT clause specifying projections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectClause {
  /// List of selected items (columns, expressions, aggregations)
  pub items: Vec<SelectItem>,
  /// Whether to select distinct values
  pub distinct: bool,
}

/// A single item in a SELECT clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
  /// Select all columns
  All,
  /// Select a specific column
  Column {
    /// Column name
    name: String,
    /// Optional table/stream alias
    table: Option<String>,
  },
  /// Select an expression with optional alias
  Expression {
    /// The expression
    expr: Expression,
    /// Optional alias
    alias: Option<String>,
  },
}

/// FROM clause specifying the source stream
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FromClause {
  /// Stream name or identifier
  pub stream: String,
  /// Optional alias
  pub alias: Option<String>,
}

/// WHERE clause for filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhereClause {
  /// The filter condition
  pub condition: Expression,
}

/// GROUP BY clause for aggregation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupByClause {
  /// Columns or expressions to group by
  pub keys: Vec<Expression>,
  /// Optional HAVING clause for filtering groups
  pub having: Option<Expression>,
}

/// WINDOW clause for windowing operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowClause {
  /// Window type
  pub window_type: WindowType,
  /// Window configuration
  pub config: WindowConfig,
}

/// Type of window
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowType {
  /// Tumbling window (non-overlapping)
  Tumbling,
  /// Sliding window (overlapping)
  Sliding,
  /// Session window (gap-based)
  Session,
}

/// Window configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
  /// Window size in seconds (for time-based windows)
  pub size_seconds: Option<u64>,
  /// Window size in rows (for count-based windows)
  pub size_rows: Option<u64>,
  /// Slide interval in seconds (for sliding windows)
  pub slide_seconds: Option<u64>,
  /// Slide interval in rows (for sliding windows)
  pub slide_rows: Option<u64>,
  /// Gap timeout in seconds (for session windows)
  pub gap_seconds: Option<u64>,
}

/// JOIN clause for joining streams
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinClause {
  /// Join type
  pub join_type: JoinType,
  /// Right stream to join
  pub right_stream: String,
  /// Optional alias for right stream
  pub right_alias: Option<String>,
  /// Join condition
  pub condition: JoinCondition,
}

/// Type of join
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinType {
  /// Inner join (default)
  Inner,
  /// Left outer join
  Left,
  /// Right outer join
  Right,
  /// Full outer join
  FullOuter,
}

/// Join condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinCondition {
  /// Equality join on columns
  On {
    /// Left column
    left: ColumnRef,
    /// Right column
    right: ColumnRef,
  },
  /// Custom expression
  Expression(Expression),
}

/// ORDER BY clause for sorting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByClause {
  /// Sort expressions with direction
  pub items: Vec<OrderByItem>,
}

/// A single ORDER BY item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
  /// Expression to sort by
  pub expr: Expression,
  /// Sort direction
  pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
  /// Ascending
  Asc,
  /// Descending
  Desc,
}

/// LIMIT clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimitClause {
  /// Maximum number of rows
  pub count: u64,
  /// Optional OFFSET
  pub offset: Option<u64>,
}

/// SQL expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
  /// Literal value
  Literal(Literal),
  /// Column reference
  Column(ColumnRef),
  /// Binary operation
  BinaryOp {
    /// Left operand
    left: Box<Expression>,
    /// Operator
    op: BinaryOperator,
    /// Right operand
    right: Box<Expression>,
  },
  /// Unary operation
  UnaryOp {
    /// Operator
    op: UnaryOperator,
    /// Operand
    operand: Box<Expression>,
  },
  /// Function call
  FunctionCall {
    /// Function name
    name: String,
    /// Arguments
    args: Vec<Expression>,
  },
  /// Aggregate function
  Aggregate {
    /// Aggregate function name
    name: String,
    /// Argument (column or expression)
    arg: Box<Expression>,
    /// Whether to count distinct values
    distinct: bool,
  },
  /// CASE expression
  Case {
    /// CASE conditions and results
    cases: Vec<(Expression, Expression)>,
    /// ELSE result (optional)
    else_result: Option<Box<Expression>>,
  },
  /// Subquery (for future use)
  Subquery(Box<SqlQuery>),
}

/// Binary operator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
  /// Addition
  Add,
  /// Subtraction
  Subtract,
  /// Multiplication
  Multiply,
  /// Division
  Divide,
  /// Modulo
  Modulo,
  /// Equality
  Eq,
  /// Inequality
  Ne,
  /// Less than
  Lt,
  /// Less than or equal
  Le,
  /// Greater than
  Gt,
  /// Greater than or equal
  Ge,
  /// Logical AND
  And,
  /// Logical OR
  Or,
  /// LIKE pattern matching
  Like,
  /// IN operator
  In,
  /// IS NULL
  IsNull,
  /// IS NOT NULL
  IsNotNull,
}

/// Unary operator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
  /// Negation
  Not,
  /// Unary minus
  Minus,
  /// Unary plus
  Plus,
}

/// Literal value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
  /// String literal
  String(String),
  /// Integer literal
  Integer(i64),
  /// Floating point literal
  Float(f64),
  /// Boolean literal
  Boolean(bool),
  /// NULL literal
  Null,
  /// Timestamp literal
  Timestamp(String),
  /// Date literal
  Date(String),
  /// Time literal
  Time(String),
  /// Interval literal
  Interval {
    /// Value
    value: i64,
    /// Unit (SECOND, MINUTE, HOUR, DAY, etc.)
    unit: String,
  },
}

/// Column reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnRef {
  /// Column name
  pub name: String,
  /// Optional table/stream name
  pub table: Option<String>,
}

impl fmt::Display for SqlQuery {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "SELECT ")?;
    if self.select.distinct {
      write!(f, "DISTINCT ")?;
    }
    for (i, item) in self.select.items.iter().enumerate() {
      if i > 0 {
        write!(f, ", ")?;
      }
      write!(f, "{}", item)?;
    }
    write!(f, " FROM {}", self.from.stream)?;
    if let Some(alias) = &self.from.alias {
      write!(f, " AS {}", alias)?;
    }
    if let Some(where_clause) = &self.where_clause {
      write!(f, " WHERE {}", where_clause.condition)?;
    }
    if let Some(group_by) = &self.group_by {
      write!(f, " GROUP BY ")?;
      for (i, key) in group_by.keys.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }
        write!(f, "{}", key)?;
      }
      if let Some(having) = &group_by.having {
        write!(f, " HAVING {}", having)?;
      }
    }
    if let Some(window) = &self.window {
      write!(f, " WINDOW {}", window)?;
    }
    if let Some(join) = &self.join {
      write!(f, " {}", join)?;
    }
    if let Some(order_by) = &self.order_by {
      write!(f, " ORDER BY ")?;
      for (i, item) in order_by.items.iter().enumerate() {
        if i > 0 {
          write!(f, ", ")?;
        }
        write!(f, "{}", item)?;
      }
    }
    if let Some(limit) = &self.limit {
      write!(f, " LIMIT {}", limit.count)?;
      if let Some(offset) = limit.offset {
        write!(f, " OFFSET {}", offset)?;
      }
    }
    Ok(())
  }
}

impl fmt::Display for SelectItem {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SelectItem::All => write!(f, "*"),
      SelectItem::Column { name, table } => {
        if let Some(t) = table {
          write!(f, "{}.{}", t, name)
        } else {
          write!(f, "{}", name)
        }
      }
      SelectItem::Expression { expr, alias } => {
        write!(f, "{}", expr)?;
        if let Some(a) = alias {
          write!(f, " AS {}", a)?;
        }
        Ok(())
      }
    }
  }
}

impl fmt::Display for Expression {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Expression::Literal(lit) => write!(f, "{}", lit),
      Expression::Column(col) => write!(f, "{}", col),
      Expression::BinaryOp { left, op, right } => {
        write!(f, "({} {} {})", left, op, right)
      }
      Expression::UnaryOp { op, operand } => {
        write!(f, "{}{}", op, operand)
      }
      Expression::FunctionCall { name, args } => {
        write!(f, "{}({})", name, args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", "))
      }
      Expression::Aggregate { name, arg, distinct } => {
        if *distinct {
          write!(f, "{}(DISTINCT {})", name, arg)
        } else {
          write!(f, "{}({})", name, arg)
        }
      }
      Expression::Case { cases, else_result } => {
        write!(f, "CASE")?;
        for (cond, result) in cases {
          write!(f, " WHEN {} THEN {}", cond, result)?;
        }
        if let Some(else_expr) = else_result {
          write!(f, " ELSE {}", else_expr)?;
        }
        write!(f, " END")
      }
      Expression::Subquery(query) => {
        write!(f, "({})", query)
      }
    }
  }
}

impl fmt::Display for BinaryOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      BinaryOperator::Add => write!(f, "+"),
      BinaryOperator::Subtract => write!(f, "-"),
      BinaryOperator::Multiply => write!(f, "*"),
      BinaryOperator::Divide => write!(f, "/"),
      BinaryOperator::Modulo => write!(f, "%"),
      BinaryOperator::Eq => write!(f, "="),
      BinaryOperator::Ne => write!(f, "!="),
      BinaryOperator::Lt => write!(f, "<"),
      BinaryOperator::Le => write!(f, "<="),
      BinaryOperator::Gt => write!(f, ">"),
      BinaryOperator::Ge => write!(f, ">="),
      BinaryOperator::And => write!(f, "AND"),
      BinaryOperator::Or => write!(f, "OR"),
      BinaryOperator::Like => write!(f, "LIKE"),
      BinaryOperator::In => write!(f, "IN"),
      BinaryOperator::IsNull => write!(f, "IS NULL"),
      BinaryOperator::IsNotNull => write!(f, "IS NOT NULL"),
    }
  }
}

impl fmt::Display for UnaryOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      UnaryOperator::Not => write!(f, "NOT"),
      UnaryOperator::Minus => write!(f, "-"),
      UnaryOperator::Plus => write!(f, "+"),
    }
  }
}

impl fmt::Display for Literal {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Literal::String(s) => write!(f, "'{}'", s),
      Literal::Integer(i) => write!(f, "{}", i),
      Literal::Float(fl) => write!(f, "{}", fl),
      Literal::Boolean(b) => write!(f, "{}", b),
      Literal::Null => write!(f, "NULL"),
      Literal::Timestamp(ts) => write!(f, "TIMESTAMP '{}'", ts),
      Literal::Date(d) => write!(f, "DATE '{}'", d),
      Literal::Time(t) => write!(f, "TIME '{}'", t),
      Literal::Interval { value, unit } => write!(f, "INTERVAL '{}' {}", value, unit),
    }
  }
}

impl fmt::Display for ColumnRef {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(table) = &self.table {
      write!(f, "{}.{}", table, self.name)
    } else {
      write!(f, "{}", self.name)
    }
  }
}

impl fmt::Display for WindowClause {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.window_type {
      WindowType::Tumbling => write!(f, "TUMBLING")?,
      WindowType::Sliding => write!(f, "SLIDING")?,
      WindowType::Session => write!(f, "SESSION")?,
    }
    write!(f, " (")?;
    let mut parts = Vec::new();
    if let Some(secs) = self.config.size_seconds {
      parts.push(format!("SIZE {} SECONDS", secs));
    }
    if let Some(rows) = self.config.size_rows {
      parts.push(format!("SIZE {} ROWS", rows));
    }
    if let Some(slide_secs) = self.config.slide_seconds {
      parts.push(format!("SLIDE {} SECONDS", slide_secs));
    }
    if let Some(slide_rows) = self.config.slide_rows {
      parts.push(format!("SLIDE {} ROWS", slide_rows));
    }
    if let Some(gap) = self.config.gap_seconds {
      parts.push(format!("GAP {} SECONDS", gap));
    }
    write!(f, "{})", parts.join(", "))
  }
}

impl fmt::Display for JoinClause {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.join_type {
      JoinType::Inner => write!(f, "INNER JOIN")?,
      JoinType::Left => write!(f, "LEFT JOIN")?,
      JoinType::Right => write!(f, "RIGHT JOIN")?,
      JoinType::FullOuter => write!(f, "FULL OUTER JOIN")?,
    }
    write!(f, " {}", self.right_stream)?;
    if let Some(alias) = &self.right_alias {
      write!(f, " AS {}", alias)?;
    }
    write!(f, " ON {}", self.condition)
  }
}

impl fmt::Display for JoinCondition {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      JoinCondition::On { left, right } => write!(f, "{} = {}", left, right),
      JoinCondition::Expression(expr) => write!(f, "{}", expr),
    }
  }
}

impl fmt::Display for OrderByItem {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.expr)?;
    match self.direction {
      SortDirection::Asc => write!(f, " ASC"),
      SortDirection::Desc => write!(f, " DESC"),
    }
  }
}

/// Alias for the AST type
pub type SqlAst = SqlQuery;

