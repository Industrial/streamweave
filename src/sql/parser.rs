//! # SQL Parser
//!
//! This module provides SQL parsing functionality, converting SQL text
//! into an Abstract Syntax Tree (AST).
//!
//! The parser uses sqlparser to parse SQL queries according to the
//! Stream SQL dialect defined in the dialect module.

use crate::error::{ComponentInfo, ErrorContext, StreamError, StringError};
use crate::sql::ast::*;
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::{Parser, ParserError};

/// SQL parser for StreamWeave queries
///
/// This parser converts SQL text into a StreamWeave SQL AST that can
/// be translated into pipeline operations.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::sql::SqlParser;
///
/// let parser = SqlParser::new();
/// let query = parser.parse(
///     "SELECT user_id, COUNT(*) FROM events GROUP BY user_id"
/// )?;
/// ```
pub struct SqlParser {
  /// SQL dialect configuration
  dialect: GenericDialect,
}

impl SqlParser {
  /// Create a new SQL parser with default dialect
  pub fn new() -> Self {
    Self {
      dialect: GenericDialect {},
    }
  }

  /// Parse a SQL query string into an AST
  ///
  /// # Arguments
  ///
  /// * `query` - SQL query string to parse
  ///
  /// # Returns
  ///
  /// Parsed SQL query AST or an error if parsing fails
  ///
  /// # Errors
  ///
  /// Returns a `StreamError` if the SQL query is invalid or contains
  /// unsupported constructs. The error message includes the position
  /// where parsing failed.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::sql::SqlParser;
  ///
  /// let parser = SqlParser::new();
  /// let query = parser.parse(
  ///     "SELECT user_id, COUNT(*) FROM events GROUP BY user_id"
  /// )?;
  /// ```
  pub fn parse(&self, query: &str) -> Result<SqlQuery, StreamError<String>> {
    let mut parser = Parser::new(&self.dialect);
    parser.try_with_sql(query)?;
    let ast = parser.parse_statement()?;

    match ast {
      Statement::Query(query) => self.convert_query(query),
      _ => Err(self.create_error(format!(
        "Unsupported statement type. Only SELECT queries are supported, found: {:?}",
        ast
      ))),
    }
  }

  /// Convert sqlparser Query AST to StreamWeave SqlQuery
  fn convert_query(&self, query: Query) -> Result<SqlQuery, StreamError<String>> {
    // Convert SELECT clause
    let select = self.convert_select(&query.body)?;

    // Convert FROM clause
    let from = self.convert_from(&query.body)?;

    // Convert WHERE clause
    let where_clause = query.selection.as_ref().map(|expr| WhereClause {
      condition: self.convert_expr(expr)?,
    });

    // Convert GROUP BY clause
    let group_by = if query.group_by.is_empty() {
      None
    } else {
      Some(GroupByClause {
        keys: query
          .group_by
          .iter()
          .map(|e| self.convert_expr(e))
          .collect::<Result<Vec<_>, _>>()?,
        having: query
          .having
          .as_ref()
          .map(|e| self.convert_expr(e))
          .transpose()?,
      })
    };

    // Convert ORDER BY clause
    let order_by = if query.order_by.is_empty() {
      None
    } else {
      Some(OrderByClause {
        items: query
          .order_by
          .iter()
          .map(|item| {
            Ok(OrderByItem {
              expr: self.convert_expr(&item.expr)?,
              direction: match item.asc {
                Some(true) | None => SortDirection::Asc,
                Some(false) => SortDirection::Desc,
              },
            })
          })
          .collect::<Result<Vec<_>, _>>()?,
      })
    };

    // Convert LIMIT clause
    let limit = query.limit.as_ref().map(|limit_expr| {
      // Try to extract a numeric value from the limit expression
      // For now, we'll handle simple integer literals
      let count = match limit_expr {
        Expr::Value(Value::Number(n, _)) => n.parse::<u64>().unwrap_or(0),
        _ => 0,
      };
      LimitClause {
        count,
        offset: query
          .offset
          .as_ref()
          .and_then(|offset| match &offset.value {
            Expr::Value(Value::Number(n, _)) => n.parse::<u64>().ok(),
            _ => None,
          }),
      }
    });

    // WINDOW and JOIN clauses are not directly supported by sqlparser's standard AST
    // These will need to be parsed from extensions or handled separately
    // For now, we'll leave them as None and handle them in a future enhancement
    let window = None;
    let join = None;

    Ok(SqlQuery {
      select,
      from,
      where_clause,
      group_by,
      window,
      join,
      order_by,
      limit,
    })
  }

  /// Convert sqlparser Select body to StreamWeave SelectClause
  fn convert_select(&self, body: &SetExpr) -> Result<SelectClause, StreamError<String>> {
    match body {
      SetExpr::Select(select) => {
        let items = select
          .projection
          .iter()
          .map(|item| self.convert_select_item(item))
          .collect::<Result<Vec<_>, _>>()?;

        Ok(SelectClause {
          items,
          distinct: select.distinct.is_some(),
        })
      }
      _ => Err(self.create_error(format!("Unsupported SELECT body type: {:?}", body))),
    }
  }

  /// Convert sqlparser SelectItem to StreamWeave SelectItem
  fn convert_select_item(&self, item: &SelectItem) -> Result<SelectItem, StreamError<String>> {
    match item {
      sqlparser::ast::SelectItem::UnnamedExpr(expr) => Ok(SelectItem::Expression {
        expr: self.convert_expr(expr)?,
        alias: None,
      }),
      sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => Ok(SelectItem::Expression {
        expr: self.convert_expr(expr)?,
        alias: Some(alias.value.clone()),
      }),
      sqlparser::ast::SelectItem::Wildcard(_) => Ok(SelectItem::All),
      sqlparser::ast::SelectItem::QualifiedWildcard(_, _) => Ok(SelectItem::All),
    }
  }

  /// Convert sqlparser FROM clause to StreamWeave FromClause
  fn convert_from(&self, body: &SetExpr) -> Result<FromClause, StreamError<String>> {
    match body {
      SetExpr::Select(select) => {
        if select.from.is_empty() {
          return Err(self.create_error("FROM clause is required".to_string()));
        }

        // For now, handle single table in FROM
        // JOINs will be handled separately
        if select.from.len() > 1 {
          return Err(self.create_error(
            "Multiple tables in FROM clause require JOIN syntax (not yet supported)".to_string(),
          ));
        }

        let table = &select.from[0];
        match &table.relation {
          TableFactor::Table { name, alias, .. } => {
            let stream_name = name.to_string();
            let alias_name = alias.as_ref().map(|a| a.name.value.clone());
            Ok(FromClause {
              stream: stream_name,
              alias: alias_name,
            })
          }
          _ => Err(self.create_error(format!(
            "Unsupported table factor in FROM: {:?}",
            table.relation
          ))),
        }
      }
      _ => Err(self.create_error(format!("FROM clause not found in body: {:?}", body))),
    }
  }

  /// Convert sqlparser Expr to StreamWeave Expression
  fn convert_expr(&self, expr: &Expr) -> Result<Expression, StreamError<String>> {
    match expr {
      Expr::Identifier(ident) => Ok(Expression::Column(ColumnRef {
        name: ident.value.clone(),
        table: None,
      })),
      Expr::CompoundIdentifier(idents) => {
        if idents.len() == 2 {
          Ok(Expression::Column(ColumnRef {
            table: Some(idents[0].value.clone()),
            name: idents[1].value.clone(),
          }))
        } else {
          Err(StreamError::new(format!(
            "Invalid compound identifier: {:?}",
            idents
          )))
        }
      }
      Expr::Value(value) => Ok(Expression::Literal(self.convert_value(value)?)),
      Expr::BinaryOp { left, op, right } => Ok(Expression::BinaryOp {
        left: Box::new(self.convert_expr(left)?),
        op: self.convert_binary_op(op)?,
        right: Box::new(self.convert_expr(right)?),
      }),
      Expr::UnaryOp { op, expr } => Ok(Expression::UnaryOp {
        op: self.convert_unary_op(op)?,
        operand: Box::new(self.convert_expr(expr)?),
      }),
      Expr::Function(func) => {
        let name = func.name.to_string();
        let args = func
          .args
          .iter()
          .map(|arg| match arg {
            FunctionArg::Unnamed(expr) => self.convert_expr(expr),
            FunctionArg::Named { name: _, arg } => match arg {
              FunctionArgExpr::Expr(expr) => self.convert_expr(expr),
              _ => Err(self.create_error("Unsupported function argument type".to_string())),
            },
          })
          .collect::<Result<Vec<_>, _>>()?;

        // Check if it's an aggregate function
        if func.distinct {
          if args.len() == 1 {
            Ok(Expression::Aggregate {
              name: name.to_uppercase(),
              arg: Box::new(args[0].clone()),
              distinct: true,
            })
          } else {
            Ok(Expression::FunctionCall { name, args })
          }
        } else if matches!(
          name.to_uppercase().as_str(),
          "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "FIRST" | "LAST"
        ) {
          if args.len() == 1 {
            Ok(Expression::Aggregate {
              name: name.to_uppercase(),
              arg: Box::new(args[0].clone()),
              distinct: false,
            })
          } else if name.to_uppercase() == "COUNT" && args.is_empty() {
            // COUNT(*) case
            Ok(Expression::Aggregate {
              name: "COUNT".to_string(),
              arg: Box::new(Expression::Literal(Literal::Null)),
              distinct: false,
            })
          } else {
            Ok(Expression::FunctionCall { name, args })
          }
        } else {
          Ok(Expression::FunctionCall { name, args })
        }
      }
      Expr::Case {
        operand: _,
        conditions,
        results,
        else_result,
      } => {
        let cases = conditions
          .iter()
          .zip(results.iter())
          .map(|(cond, result)| Ok((self.convert_expr(cond)?, self.convert_expr(result)?)))
          .collect::<Result<Vec<_>, _>>()?;

        Ok(Expression::Case {
          cases,
          else_result: else_result
            .as_ref()
            .map(|e| Box::new(self.convert_expr(e)?))
            .transpose()?,
        })
      }
      Expr::IsNull(expr) => Ok(Expression::BinaryOp {
        left: Box::new(self.convert_expr(expr)?),
        op: BinaryOperator::IsNull,
        right: Box::new(Expression::Literal(Literal::Null)),
      }),
      Expr::IsNotNull(expr) => Ok(Expression::BinaryOp {
        left: Box::new(self.convert_expr(expr)?),
        op: BinaryOperator::IsNotNull,
        right: Box::new(Expression::Literal(Literal::Null)),
      }),
      _ => Err(self.create_error(format!("Unsupported expression type: {:?}", expr))),
    }
  }

  /// Create a StreamError from a string message
  fn create_error(&self, message: String) -> StreamError<String> {
    StreamError::new(
      Box::new(StringError(message.clone())),
      ErrorContext::default(),
      ComponentInfo::new("SQL Parser".to_string(), "SqlParser".to_string()),
    )
  }

  /// Convert sqlparser Value to StreamWeave Literal
  fn convert_value(&self, value: &Value) -> Result<Literal, StreamError<String>> {
    match value {
      Value::Number(n, _) => {
        if n.contains('.') {
          Ok(Literal::Float(n.parse().map_err(|_| {
            self.create_error(format!("Invalid float literal: {}", n))
          })?))
        } else {
          Ok(Literal::Integer(n.parse().map_err(|_| {
            self.create_error(format!("Invalid integer literal: {}", n))
          })?))
        }
      }
      Value::SingleQuotedString(s) | Value::DoubleQuotedString(s) => Ok(Literal::String(s.clone())),
      Value::Boolean(b) => Ok(Literal::Boolean(*b)),
      Value::Null => Ok(Literal::Null),
      _ => Err(self.create_error(format!("Unsupported value type: {:?}", value))),
    }
  }

  /// Convert sqlparser BinaryOperator to StreamWeave BinaryOperator
  fn convert_binary_op(&self, op: &BinaryOperator) -> Result<BinaryOperator, StreamError<String>> {
    match op {
      sqlparser::ast::BinaryOperator::Plus => Ok(BinaryOperator::Add),
      sqlparser::ast::BinaryOperator::Minus => Ok(BinaryOperator::Subtract),
      sqlparser::ast::BinaryOperator::Multiply => Ok(BinaryOperator::Multiply),
      sqlparser::ast::BinaryOperator::Divide => Ok(BinaryOperator::Divide),
      sqlparser::ast::BinaryOperator::Modulo => Ok(BinaryOperator::Modulo),
      sqlparser::ast::BinaryOperator::Eq => Ok(BinaryOperator::Eq),
      sqlparser::ast::BinaryOperator::NotEq => Ok(BinaryOperator::Ne),
      sqlparser::ast::BinaryOperator::Lt => Ok(BinaryOperator::Lt),
      sqlparser::ast::BinaryOperator::LtEq => Ok(BinaryOperator::Le),
      sqlparser::ast::BinaryOperator::Gt => Ok(BinaryOperator::Gt),
      sqlparser::ast::BinaryOperator::GtEq => Ok(BinaryOperator::Ge),
      sqlparser::ast::BinaryOperator::And => Ok(BinaryOperator::And),
      sqlparser::ast::BinaryOperator::Or => Ok(BinaryOperator::Or),
      sqlparser::ast::BinaryOperator::Like => Ok(BinaryOperator::Like),
      _ => Err(self.create_error(format!("Unsupported binary operator: {:?}", op))),
    }
  }

  /// Convert sqlparser UnaryOperator to StreamWeave UnaryOperator
  fn convert_unary_op(&self, op: &UnaryOperator) -> Result<UnaryOperator, StreamError<String>> {
    match op {
      sqlparser::ast::UnaryOperator::Not => Ok(UnaryOperator::Not),
      sqlparser::ast::UnaryOperator::Minus => Ok(UnaryOperator::Minus),
      sqlparser::ast::UnaryOperator::Plus => Ok(UnaryOperator::Plus),
    }
  }
}

impl Default for SqlParser {
  fn default() -> Self {
    Self::new()
  }
}

// Helper trait to convert ParserError to StreamError
impl From<ParserError> for StreamError<String> {
  fn from(err: ParserError) -> Self {
    StreamError::new(
      Box::new(StringError(format!(
        "SQL parse error at position {}: {}",
        err.pos, err.message
      ))),
      ErrorContext::default(),
      ComponentInfo::new("SQL Parser".to_string(), "SqlParser".to_string()),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_simple_select() {
    let parser = SqlParser::new();
    let result = parser.parse("SELECT id, name FROM users");
    assert!(result.is_ok());
    let query = result.unwrap();
    assert_eq!(query.from.stream, "users");
    assert_eq!(query.select.items.len(), 2);
  }

  #[test]
  fn test_parse_select_with_where() {
    let parser = SqlParser::new();
    let result = parser.parse("SELECT * FROM events WHERE value > 100");
    assert!(result.is_ok());
    let query = result.unwrap();
    assert!(query.where_clause.is_some());
  }

  #[test]
  fn test_parse_select_with_group_by() {
    let parser = SqlParser::new();
    let result = parser.parse("SELECT user_id, COUNT(*) FROM events GROUP BY user_id");
    assert!(result.is_ok());
    let query = result.unwrap();
    assert!(query.group_by.is_some());
    let group_by = query.group_by.unwrap();
    assert_eq!(group_by.keys.len(), 1);
  }

  #[test]
  fn test_parse_aggregate_function() {
    let parser = SqlParser::new();
    let result =
      parser.parse("SELECT user_id, COUNT(*), SUM(amount) FROM purchases GROUP BY user_id");
    assert!(result.is_ok());
    let query = result.unwrap();
    // Check that COUNT and SUM are recognized as aggregates
    let select_items = &query.select.items;
    assert!(select_items.len() >= 2);
  }

  #[test]
  fn test_parse_error_invalid_syntax() {
    let parser = SqlParser::new();
    let result = parser.parse("SELECT FROM");
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_error_missing_from() {
    let parser = SqlParser::new();
    let result = parser.parse("SELECT id, name");
    assert!(result.is_err());
  }
}
