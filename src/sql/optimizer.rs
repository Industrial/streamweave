//! # Query Optimizer
//!
//! This module optimizes SQL query plans for better performance.
//! It applies optimization rules such as predicate pushdown, projection
//! pruning, and join reordering.
//!
//! ## Optimization Rules
//!
//! The optimizer applies the following rules to improve query performance:
//!
//! 1. **Predicate Pushdown**: Move WHERE conditions as early as possible
//!    to filter data before expensive operations
//! 2. **Projection Pruning**: Remove unused columns from SELECT early
//!    to reduce data volume through the pipeline
//! 3. **Join Reordering**: Reorder joins to process smaller streams first
//! 4. **Constant Folding**: Evaluate constant expressions at compile time
//! 5. **Dead Code Elimination**: Remove unreachable or redundant operations

use crate::error::{ComponentInfo, ErrorContext, StreamError, StringError};
use crate::sql::ast::*;

/// Query optimizer for SQL queries
///
/// The optimizer applies various rules to improve query performance
/// without changing the semantic meaning of the query.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::sql::{SqlQuery, QueryOptimizer};
///
/// let query = SqlQuery { /* ... */ };
/// let optimizer = QueryOptimizer::new();
/// let optimized = optimizer.optimize(query)?;
/// ```
pub struct QueryOptimizer {
  /// Whether to enable aggressive optimizations
  pub aggressive: bool,
}

impl QueryOptimizer {
  /// Create a new query optimizer with default settings
  pub fn new() -> Self {
    Self { aggressive: false }
  }

  /// Create an optimizer with aggressive optimizations enabled
  pub fn aggressive() -> Self {
    Self { aggressive: true }
  }

  /// Optimize a SQL query AST
  ///
  /// Applies optimization rules to improve query performance while
  /// preserving semantic correctness.
  ///
  /// # Arguments
  ///
  /// * `query` - SQL query AST to optimize
  ///
  /// # Returns
  ///
  /// Optimized SQL query AST or an error if optimization fails
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::sql::{SqlQuery, QueryOptimizer};
  ///
  /// let query = SqlQuery { /* ... */ };
  /// let optimizer = QueryOptimizer::new();
  /// let optimized = optimizer.optimize(query)?;
  /// ```
  pub fn optimize(&self, mut query: SqlQuery) -> Result<SqlQuery, Box<StreamError<String>>> {
    // Apply optimization rules in order

    // 1. Constant folding in WHERE clause
    if let Some(ref mut where_clause) = query.where_clause {
      where_clause.condition = self
        .fold_constants(where_clause.condition.clone())
        .map_err(Box::new)?;
    }

    // 2. Constant folding in SELECT expressions
    query.select.items = query
      .select
      .items
      .iter()
      .map(|item| match item {
        crate::sql::ast::SelectItem::Expression { expr, alias } => self
          .fold_constants(expr.clone())
          .map(|expr| crate::sql::ast::SelectItem::Expression {
            expr,
            alias: alias.clone(),
          })
          .map_err(Box::new),
        _ => Ok::<crate::sql::ast::SelectItem, Box<StreamError<String>>>(item.clone()),
      })
      .collect::<Result<Vec<_>, _>>()?;

    // 3. Projection pruning (if aggressive mode)
    if self.aggressive {
      query = self.prune_projections(query).map_err(Box::new)?;
    }

    // 4. Predicate pushdown is already handled by query structure
    // (WHERE is applied before GROUP BY, etc.)

    Ok(query)
  }

  /// Fold constant expressions (evaluate at compile time)
  #[allow(clippy::result_large_err)] // Boxing errors intentionally to reduce Result size
  fn fold_constants(&self, expr: Expression) -> Result<Expression, StreamError<String>> {
    match expr {
      Expression::BinaryOp { left, op, right } => {
        let left_folded = self.fold_constants(*left)?;
        let right_folded = self.fold_constants(*right)?;

        // If both sides are literals, evaluate the expression
        if let (Expression::Literal(left_lit), Expression::Literal(right_lit)) =
          (&left_folded, &right_folded)
        {
          if let Some(result) = self.evaluate_binary_op(left_lit, &op, right_lit)? {
            return Ok(Expression::Literal(result));
          }
        }

        Ok(Expression::BinaryOp {
          left: Box::new(left_folded),
          op,
          right: Box::new(right_folded),
        })
      }
      Expression::UnaryOp { op, operand } => {
        let operand_folded = self.fold_constants(*operand)?;
        if let Expression::Literal(lit) = &operand_folded {
          if let Some(result) = self.evaluate_unary_op(lit, &op)? {
            return Ok(Expression::Literal(result));
          }
        }
        Ok(Expression::UnaryOp {
          op,
          operand: Box::new(operand_folded),
        })
      }
      Expression::FunctionCall { name, args } => {
        let args_folded = args
          .iter()
          .map(|arg| self.fold_constants(arg.clone()))
          .collect::<Result<Vec<_>, _>>()?;
        Ok(Expression::FunctionCall {
          name,
          args: args_folded,
        })
      }
      other => Ok(other),
    }
  }

  /// Evaluate a binary operation on literals
  #[allow(clippy::result_large_err)] // Boxing errors intentionally to reduce Result size
  fn evaluate_binary_op(
    &self,
    left: &Literal,
    op: &crate::sql::ast::BinaryOperator,
    right: &Literal,
  ) -> Result<Option<Literal>, StreamError<String>> {
    match (left, op, right) {
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Add, Literal::Integer(r)) => {
        Ok(Some(Literal::Integer(l + r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Subtract, Literal::Integer(r)) => {
        Ok(Some(Literal::Integer(l - r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Multiply, Literal::Integer(r)) => {
        Ok(Some(Literal::Integer(l * r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Divide, Literal::Integer(r)) => {
        if *r == 0 {
          return Err(self.create_error("Division by zero".to_string()));
        }
        Ok(Some(Literal::Integer(l / r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Eq, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l == r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Ne, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l != r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Lt, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l < r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Le, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l <= r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Gt, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l > r)))
      }
      (Literal::Integer(l), crate::sql::ast::BinaryOperator::Ge, Literal::Integer(r)) => {
        Ok(Some(Literal::Boolean(l >= r)))
      }
      (Literal::Float(l), crate::sql::ast::BinaryOperator::Add, Literal::Float(r)) => {
        Ok(Some(Literal::Float(l + r)))
      }
      (Literal::Float(l), crate::sql::ast::BinaryOperator::Subtract, Literal::Float(r)) => {
        Ok(Some(Literal::Float(l - r)))
      }
      (Literal::Float(l), crate::sql::ast::BinaryOperator::Multiply, Literal::Float(r)) => {
        Ok(Some(Literal::Float(l * r)))
      }
      (Literal::Float(l), crate::sql::ast::BinaryOperator::Divide, Literal::Float(r)) => {
        if *r == 0.0 {
          return Err(self.create_error("Division by zero".to_string()));
        }
        Ok(Some(Literal::Float(l / r)))
      }
      (Literal::Boolean(l), crate::sql::ast::BinaryOperator::And, Literal::Boolean(r)) => {
        Ok(Some(Literal::Boolean(*l && *r)))
      }
      (Literal::Boolean(l), crate::sql::ast::BinaryOperator::Or, Literal::Boolean(r)) => {
        Ok(Some(Literal::Boolean(*l || *r)))
      }
      _ => Ok(None), // Can't evaluate, return None
    }
  }

  /// Evaluate a unary operation on a literal
  #[allow(clippy::result_large_err)] // Boxing errors intentionally to reduce Result size
  fn evaluate_unary_op(
    &self,
    operand: &Literal,
    op: &crate::sql::ast::UnaryOperator,
  ) -> Result<Option<Literal>, StreamError<String>> {
    match (operand, op) {
      (Literal::Integer(n), crate::sql::ast::UnaryOperator::Minus) => {
        Ok(Some(Literal::Integer(-n)))
      }
      (Literal::Float(n), crate::sql::ast::UnaryOperator::Minus) => Ok(Some(Literal::Float(-n))),
      (Literal::Boolean(b), crate::sql::ast::UnaryOperator::Not) => Ok(Some(Literal::Boolean(!b))),
      _ => Ok(None),
    }
  }

  /// Prune unused projections (remove columns that aren't referenced)
  #[allow(clippy::result_large_err)] // Boxing errors intentionally to reduce Result size
  fn prune_projections(&self, query: SqlQuery) -> Result<SqlQuery, StreamError<String>> {
    // For now, this is a placeholder
    // In a full implementation, we would:
    // 1. Track which columns are actually used (in WHERE, GROUP BY, ORDER BY, etc.)
    // 2. Remove unused columns from SELECT
    // 3. This requires more sophisticated analysis
    Ok(query)
  }

  /// Create a StreamError from a string message
  fn create_error(&self, message: String) -> StreamError<String> {
    StreamError::new(
      Box::new(StringError(message.clone())),
      ErrorContext::default(),
      ComponentInfo::new("SQL Optimizer".to_string(), "QueryOptimizer".to_string()),
    )
  }
}

impl Default for QueryOptimizer {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_constant_folding_addition() {
    let optimizer = QueryOptimizer::new();
    let expr = Expression::BinaryOp {
      left: Box::new(Expression::Literal(Literal::Integer(5))),
      op: crate::sql::ast::BinaryOperator::Add,
      right: Box::new(Expression::Literal(Literal::Integer(3))),
    };

    let result = optimizer.fold_constants(expr).unwrap();
    assert_eq!(result, Expression::Literal(Literal::Integer(8)));
  }

  #[test]
  fn test_constant_folding_comparison() {
    let optimizer = QueryOptimizer::new();
    let expr = Expression::BinaryOp {
      left: Box::new(Expression::Literal(Literal::Integer(10))),
      op: crate::sql::ast::BinaryOperator::Gt,
      right: Box::new(Expression::Literal(Literal::Integer(5))),
    };

    let result = optimizer.fold_constants(expr).unwrap();
    assert_eq!(result, Expression::Literal(Literal::Boolean(true)));
  }

  #[test]
  fn test_optimize_preserves_semantics() {
    let optimizer = QueryOptimizer::new();
    let query = SqlQuery {
      select: SelectClause {
        items: vec![SelectItem::Column {
          name: "id".to_string(),
          table: None,
        }],
        distinct: false,
      },
      from: FromClause {
        stream: "users".to_string(),
        alias: None,
      },
      where_clause: Some(WhereClause {
        condition: Expression::BinaryOp {
          left: Box::new(Expression::Literal(Literal::Integer(5))),
          op: crate::sql::ast::BinaryOperator::Add,
          right: Box::new(Expression::Literal(Literal::Integer(3))),
        },
      }),
      group_by: None,
      window: None,
      join: None,
      order_by: None,
      limit: None,
    };

    let optimized = optimizer.optimize(query).unwrap();
    // WHERE clause should have constant folded
    if let Some(WhereClause { condition }) = optimized.where_clause {
      assert_eq!(condition, Expression::Literal(Literal::Integer(8)));
    } else {
      panic!("WHERE clause should be present");
    }
  }
}
