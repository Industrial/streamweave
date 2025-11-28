//! # Query Translator
//!
//! This module translates SQL AST nodes into StreamWeave pipeline operations.
//! It converts SQL constructs (SELECT, WHERE, GROUP BY, etc.) into appropriate
//! transformers and pipeline configurations.
//!
//! ## Translation Strategy
//!
//! SQL queries are translated into a sequence of pipeline operations:
//!
//! 1. **FROM** → Producer (must be provided separately, as stream name doesn't specify producer type)
//! 2. **WHERE** → FilterTransformer
//! 3. **SELECT** → MapTransformer (for projections) or identity
//! 4. **GROUP BY** → GroupByTransformer + aggregation transformers
//! 5. **WINDOW** → WindowTransformer
//! 6. **ORDER BY** → SortTransformer
//! 7. **LIMIT** → LimitTransformer
//!
//! The translator creates a `QueryPlan` that describes these operations.
//! The plan can then be used to build an actual pipeline when a producer is provided.

use crate::error::StreamError;
use crate::sql::ast::*;
use serde::{Deserialize, Serialize};

/// Plan describing how to translate a SQL query into pipeline operations
///
/// This plan captures the sequence of transformations needed to execute
/// a SQL query. The actual pipeline is built from this plan when a
/// producer is provided.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPlan {
  /// Source stream name (from FROM clause)
  pub source_stream: String,
  /// Sequence of transformation operations
  pub operations: Vec<QueryOperation>,
}

/// A single operation in the query plan
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOperation {
  /// Filter operation (WHERE clause)
  Filter {
    /// Filter condition expression
    condition: Expression,
  },
  /// Projection operation (SELECT clause)
  Project {
    /// Selected items/expressions
    items: Vec<SelectItem>,
  },
  /// Group by operation
  GroupBy {
    /// Grouping keys
    keys: Vec<Expression>,
    /// Aggregations to compute
    aggregations: Vec<Aggregation>,
    /// HAVING clause filter (optional)
    having: Option<Expression>,
  },
  /// Window operation
  Window {
    /// Window type
    window_type: WindowType,
    /// Window configuration
    config: WindowConfig,
  },
  /// Sort operation (ORDER BY)
  Sort {
    /// Sort items with directions
    items: Vec<OrderByItem>,
  },
  /// Limit operation
  Limit {
    /// Maximum number of rows
    count: u64,
    /// Optional offset
    offset: Option<u64>,
  },
}

/// Aggregation operation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aggregation {
  /// Aggregation function name (COUNT, SUM, AVG, etc.)
  pub function: String,
  /// Expression to aggregate
  pub expression: Expression,
  /// Whether to count distinct values
  pub distinct: bool,
  /// Optional alias for the result
  pub alias: Option<String>,
}

/// Translates SQL queries to StreamWeave pipeline plans
///
/// The translator converts SQL AST nodes into a sequence of pipeline
/// operations. The resulting plan can be used to build an actual pipeline
/// when combined with a producer.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::sql::{SqlQuery, QueryTranslator};
///
/// let query = SqlQuery { /* ... */ };
/// let translator = QueryTranslator::new();
/// let plan = translator.translate(&query)?;
/// ```
pub struct QueryTranslator;

impl QueryTranslator {
  /// Create a new query translator
  pub fn new() -> Self {
    Self
  }

  /// Translate a SQL query AST into a query plan
  ///
  /// # Arguments
  ///
  /// * `query` - SQL query AST to translate
  ///
  /// # Returns
  ///
  /// A query plan describing the pipeline operations or an error if translation fails
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::sql::{SqlQuery, QueryTranslator};
  ///
  /// let query = SqlQuery { /* ... */ };
  /// let translator = QueryTranslator::new();
  /// let plan = translator.translate(&query)?;
  /// ```
  pub fn translate(&self, query: &SqlQuery) -> Result<QueryPlan, Box<StreamError<String>>> {
    let mut operations = Vec::new();

    // WHERE clause → Filter operation
    if let Some(where_clause) = &query.where_clause {
      operations.push(QueryOperation::Filter {
        condition: where_clause.condition.clone(),
      });
    }

    // GROUP BY + aggregations → GroupBy operation
    if let Some(group_by) = &query.group_by {
      // Extract aggregations from SELECT items
      let aggregations = self
        .extract_aggregations(&query.select.items)
        .map_err(Box::new)?;

      operations.push(QueryOperation::GroupBy {
        keys: group_by.keys.clone(),
        aggregations,
        having: group_by.having.clone(),
      });
    }

    // WINDOW clause → Window operation
    if let Some(window) = &query.window {
      operations.push(QueryOperation::Window {
        window_type: window.window_type.clone(),
        config: window.config.clone(),
      });
    }

    // SELECT clause → Project operation (if not already handled by GROUP BY)
    if query.group_by.is_none() {
      operations.push(QueryOperation::Project {
        items: query.select.items.clone(),
      });
    }

    // ORDER BY clause → Sort operation
    if let Some(order_by) = &query.order_by {
      operations.push(QueryOperation::Sort {
        items: order_by.items.clone(),
      });
    }

    // LIMIT clause → Limit operation
    if let Some(limit) = &query.limit {
      operations.push(QueryOperation::Limit {
        count: limit.count,
        offset: limit.offset,
      });
    }

    Ok(QueryPlan {
      source_stream: query.from.stream.clone(),
      operations,
    })
  }

  /// Extract aggregation functions from SELECT items
  #[allow(clippy::result_large_err)] // Boxing errors intentionally to reduce Result size
  fn extract_aggregations(
    &self,
    items: &[SelectItem],
  ) -> Result<Vec<Aggregation>, StreamError<String>> {
    let mut aggregations = Vec::new();

    for item in items {
      #[allow(clippy::collapsible_if)]
      if let SelectItem::Expression { expr, alias } = item {
        if let Expression::Aggregate {
          name,
          arg,
          distinct,
        } = expr
        {
          aggregations.push(Aggregation {
            function: name.clone(),
            expression: *arg.clone(),
            distinct: *distinct,
            alias: alias.clone(),
          });
        }
      }
    }

    Ok(aggregations)
  }
}

impl Default for QueryTranslator {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_translate_simple_select() {
    let translator = QueryTranslator::new();
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
      where_clause: None,
      group_by: None,
      window: None,
      join: None,
      order_by: None,
      limit: None,
    };

    let plan = translator.translate(&query).unwrap();
    assert_eq!(plan.source_stream, "users");
    assert_eq!(plan.operations.len(), 1); // Just Project
  }

  #[test]
  fn test_translate_with_where() {
    let translator = QueryTranslator::new();
    let query = SqlQuery {
      select: SelectClause {
        items: vec![SelectItem::All],
        distinct: false,
      },
      from: FromClause {
        stream: "events".to_string(),
        alias: None,
      },
      where_clause: Some(WhereClause {
        condition: Expression::BinaryOp {
          left: Box::new(Expression::Column(ColumnRef {
            name: "value".to_string(),
            table: None,
          })),
          op: BinaryOperator::Gt,
          right: Box::new(Expression::Literal(Literal::Integer(100))),
        },
      }),
      group_by: None,
      window: None,
      join: None,
      order_by: None,
      limit: None,
    };

    let plan = translator.translate(&query).unwrap();
    assert_eq!(plan.operations.len(), 2); // Filter + Project
    match &plan.operations[0] {
      QueryOperation::Filter { .. } => {}
      _ => panic!("First operation should be Filter"),
    }
  }

  #[test]
  fn test_translate_with_group_by() {
    let translator = QueryTranslator::new();
    let query = SqlQuery {
      select: SelectClause {
        items: vec![
          SelectItem::Column {
            name: "user_id".to_string(),
            table: None,
          },
          SelectItem::Expression {
            expr: Expression::Aggregate {
              name: "COUNT".to_string(),
              arg: Box::new(Expression::Literal(Literal::Null)),
              distinct: false,
            },
            alias: Some("count".to_string()),
          },
        ],
        distinct: false,
      },
      from: FromClause {
        stream: "events".to_string(),
        alias: None,
      },
      where_clause: None,
      group_by: Some(GroupByClause {
        keys: vec![Expression::Column(ColumnRef {
          name: "user_id".to_string(),
          table: None,
        })],
        having: None,
      }),
      window: None,
      join: None,
      order_by: None,
      limit: None,
    };

    let plan = translator.translate(&query).unwrap();
    assert!(
      plan
        .operations
        .iter()
        .any(|op| matches!(op, QueryOperation::GroupBy { .. }))
    );
  }
}
