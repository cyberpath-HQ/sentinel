use serde_json::Value;
use tokio_stream::Stream;

/// Represents a query for filtering documents in a collection.
///
/// A query consists of filters, sorting, limits, offsets, and field projections.
/// Queries are executed in-memory for basic filtering operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    /// List of filters to apply
    pub filters:    Vec<Filter>,
    /// Optional sorting (field, order)
    pub sort:       Option<(String, SortOrder)>,
    /// Maximum number of results
    pub limit:      Option<usize>,
    /// Number of results to skip
    pub offset:     Option<usize>,
    /// Fields to include in results (projection)
    pub projection: Option<Vec<String>>,
}

/// The result of executing a query.
pub struct QueryResult {
    /// The matching documents as a stream
    pub documents:      std::pin::Pin<Box<dyn Stream<Item = crate::Result<crate::Document>> + Send>>,
    /// Total number of documents that matched (before limit/offset), None if not known
    pub total_count:    Option<usize>,
    /// Time taken to execute the query
    pub execution_time: std::time::Duration,
}

/// Sort order for query results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    /// Ascending order
    Ascending,
    /// Descending order
    Descending,
}

/// A filter condition for querying documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    /// Equality filter: field == value
    Equals(String, Value),
    /// Greater than filter: field > value
    GreaterThan(String, Value),
    /// Less than filter: field < value
    LessThan(String, Value),
    /// Greater or equal filter: field >= value
    GreaterOrEqual(String, Value),
    /// Less or equal filter: field <= value
    LessOrEqual(String, Value),
    /// Contains filter: field contains substring (for strings)
    Contains(String, String),
    /// Starts with filter: field starts with prefix (for strings)
    StartsWith(String, String),
    /// Ends with filter: field ends with suffix (for strings)
    EndsWith(String, String),
    /// In filter: field value is in the provided list
    In(String, Vec<Value>),
    /// Exists filter: field exists (or doesn't exist if false)
    Exists(String, bool),
    /// Logical AND of two filters
    And(Box<Self>, Box<Self>),
    /// Logical OR of two filters
    Or(Box<Self>, Box<Self>),
}

/// Operator for building filters in the query builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    /// Equality
    Equals,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Greater or equal
    GreaterOrEqual,
    /// Less or equal
    LessOrEqual,
    /// Contains substring
    Contains,
    /// Starts with prefix
    StartsWith,
    /// Ends with suffix
    EndsWith,
    /// Value in list
    In,
    /// Field exists
    Exists,
}

/// Builder pattern for constructing queries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBuilder {
    filters:    Vec<Filter>,
    sort:       Option<(String, SortOrder)>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    projection: Option<Vec<String>>,
}

impl Default for QueryBuilder {
    fn default() -> Self { Self::new() }
}

impl QueryBuilder {
    /// Creates a new empty query builder.
    pub const fn new() -> Self {
        Self {
            filters:    Vec::new(),
            sort:       None,
            limit:      None,
            offset:     None,
            projection: None,
        }
    }

    /// Adds a filter condition to the query.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name to filter on
    /// * `op` - The operator to use
    /// * `value` - The value to compare against
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{Operator, QueryBuilder};
    /// use serde_json::json;
    ///
    /// let query = QueryBuilder::new()
    ///     .filter("age", Operator::GreaterThan, json!(18))
    ///     .filter("status", Operator::Equals, json!("active"));
    /// ```
    pub fn filter(mut self, field: &str, op: Operator, value: Value) -> Self {
        let filter = match op {
            Operator::Equals => Filter::Equals(field.to_owned(), value),
            Operator::GreaterThan => Filter::GreaterThan(field.to_owned(), value),
            Operator::LessThan => Filter::LessThan(field.to_owned(), value),
            Operator::GreaterOrEqual => Filter::GreaterOrEqual(field.to_owned(), value),
            Operator::LessOrEqual => Filter::LessOrEqual(field.to_owned(), value),
            Operator::Contains => {
                if let Value::String(s) = value {
                    Filter::Contains(field.to_owned(), s)
                }
                else {
                    // Invalid type for contains, ignore or handle error
                    return self;
                }
            },
            Operator::StartsWith => {
                if let Value::String(s) = value {
                    Filter::StartsWith(field.to_owned(), s)
                }
                else {
                    return self;
                }
            },
            Operator::EndsWith => {
                if let Value::String(s) = value {
                    Filter::EndsWith(field.to_owned(), s)
                }
                else {
                    return self;
                }
            },
            Operator::In => {
                if let Value::Array(arr) = value {
                    Filter::In(field.to_owned(), arr)
                }
                else {
                    return self;
                }
            },
            Operator::Exists => {
                let exists = match value {
                    Value::Bool(b) => b,
                    Value::Number(n) if n.as_i64() == Some(1) => true,
                    Value::Number(n) if n.as_i64() == Some(0) => false,
                    Value::Null | Value::Number(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => true, /* Default to exists */
                };
                Filter::Exists(field.to_owned(), exists)
            },
        };
        self.filters.push(filter);
        self
    }

    /// Adds a logical AND filter combining the current filters.
    ///
    /// # Arguments
    ///
    /// * `other` - Another filter to AND with the current query
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    pub fn and(mut self, other: Filter) -> Self {
        if let Some(last) = self.filters.pop() {
            let combined = Filter::And(Box::new(last), Box::new(other));
            self.filters.push(combined);
        }
        else {
            self.filters.push(other);
        }
        self
    }

    /// Adds a logical OR filter combining the current filters.
    ///
    /// # Arguments
    ///
    /// * `other` - Another filter to OR with the current query
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    pub fn or(mut self, other: Filter) -> Self {
        if let Some(last) = self.filters.pop() {
            let combined = Filter::Or(Box::new(last), Box::new(other));
            self.filters.push(combined);
        }
        else {
            self.filters.push(other);
        }
        self
    }

    /// Sets the sort order for the query results.
    ///
    /// # Arguments
    ///
    /// * `field` - The field to sort by
    /// * `order` - The sort order (ascending or descending)
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::{QueryBuilder, SortOrder};
    ///
    /// let query = QueryBuilder::new().sort("age", SortOrder::Descending);
    /// ```
    pub fn sort(mut self, field: &str, order: SortOrder) -> Self {
        self.sort = Some((field.to_owned(), order));
        self
    }

    /// Sets the maximum number of results to return.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of documents to return
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    pub const fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the number of results to skip.
    ///
    /// # Arguments
    ///
    /// * `offset` - Number of documents to skip
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    pub const fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the fields to include in the results (projection).
    ///
    /// If projection is set, only the specified fields will be included
    /// in the returned documents. If not set, all fields are included.
    ///
    /// # Arguments
    ///
    /// * `fields` - List of field names to include
    ///
    /// # Returns
    ///
    /// Returns the query builder for chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sentinel_dbms::QueryBuilder;
    ///
    /// let query = QueryBuilder::new().projection(vec!["name", "email"]);
    /// ```
    pub fn projection(mut self, fields: Vec<&str>) -> Self {
        self.projection = Some(fields.into_iter().map(|s| s.to_owned()).collect());
        self
    }

    /// Builds the query from the current builder state.
    ///
    /// # Returns
    ///
    /// Returns a `Query` that can be executed against a collection.
    pub fn build(self) -> Query {
        Query {
            filters:    self.filters,
            sort:       self.sort,
            limit:      self.limit,
            offset:     self.offset,
            projection: self.projection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_builder_new() {
        let qb = QueryBuilder::new();
        assert!(qb.filters.is_empty());
        assert!(qb.sort.is_none());
        assert!(qb.limit.is_none());
        assert!(qb.offset.is_none());
        assert!(qb.projection.is_none());
    }

    #[test]
    fn test_query_builder_filter_equals() {
        let qb = QueryBuilder::new().filter("name", Operator::Equals, json!("Alice"));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Equals(field, value) => {
                assert_eq!(field, "name");
                assert_eq!(value, &json!("Alice"));
            },
            _ => panic!("Expected Equals filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_greater_than() {
        let qb = QueryBuilder::new().filter("age", Operator::GreaterThan, json!(18));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::GreaterThan(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, &json!(18));
            },
            _ => panic!("Expected GreaterThan filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_less_than() {
        let qb = QueryBuilder::new().filter("age", Operator::LessThan, json!(65));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::LessThan(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, &json!(65));
            },
            _ => panic!("Expected LessThan filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_greater_or_equal() {
        let qb = QueryBuilder::new().filter("age", Operator::GreaterOrEqual, json!(18));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::GreaterOrEqual(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, &json!(18));
            },
            _ => panic!("Expected GreaterOrEqual filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_less_or_equal() {
        let qb = QueryBuilder::new().filter("age", Operator::LessOrEqual, json!(65));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::LessOrEqual(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, &json!(65));
            },
            _ => panic!("Expected LessOrEqual filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_contains_valid() {
        let qb = QueryBuilder::new().filter("name", Operator::Contains, json!("Ali"));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Contains(field, value) => {
                assert_eq!(field, "name");
                assert_eq!(value, "Ali");
            },
            _ => panic!("Expected Contains filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_contains_invalid() {
        let qb = QueryBuilder::new().filter("name", Operator::Contains, json!(123));
        assert!(qb.filters.is_empty());
    }

    #[test]
    fn test_query_builder_filter_starts_with_valid() {
        let qb = QueryBuilder::new().filter("name", Operator::StartsWith, json!("Ali"));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::StartsWith(field, value) => {
                assert_eq!(field, "name");
                assert_eq!(value, "Ali");
            },
            _ => panic!("Expected StartsWith filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_starts_with_invalid() {
        let qb = QueryBuilder::new().filter("name", Operator::StartsWith, json!(123));
        assert!(qb.filters.is_empty());
    }

    #[test]
    fn test_query_builder_filter_ends_with_valid() {
        let qb = QueryBuilder::new().filter("name", Operator::EndsWith, json!("ice"));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::EndsWith(field, value) => {
                assert_eq!(field, "name");
                assert_eq!(value, "ice");
            },
            _ => panic!("Expected EndsWith filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_ends_with_invalid() {
        let qb = QueryBuilder::new().filter("name", Operator::EndsWith, json!(123));
        assert!(qb.filters.is_empty());
    }

    #[test]
    fn test_query_builder_filter_in_valid() {
        let qb = QueryBuilder::new().filter("status", Operator::In, json!(["active", "inactive"]));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::In(field, values) => {
                assert_eq!(field, "status");
                assert_eq!(values, &vec![json!("active"), json!("inactive")]);
            },
            _ => panic!("Expected In filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_in_invalid() {
        let qb = QueryBuilder::new().filter("status", Operator::In, json!("active"));
        assert!(qb.filters.is_empty());
    }

    #[test]
    fn test_query_builder_filter_exists_bool() {
        let qb = QueryBuilder::new().filter("name", Operator::Exists, json!(true));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Exists(field, exists) => {
                assert_eq!(field, "name");
                assert!(*exists);
            },
            _ => panic!("Expected Exists filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_exists_number() {
        let qb = QueryBuilder::new().filter("name", Operator::Exists, json!(1));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Exists(field, exists) => {
                assert_eq!(field, "name");
                assert!(*exists);
            },
            _ => panic!("Expected Exists filter"),
        }
    }

    #[test]
    fn test_query_builder_filter_exists_false() {
        let qb = QueryBuilder::new().filter("name", Operator::Exists, json!(false));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Exists(field, exists) => {
                assert_eq!(field, "name");
                assert!(!*exists);
            },
            _ => panic!("Expected Exists filter"),
        }
    }

    #[test]
    fn test_query_builder_sort() {
        let qb = QueryBuilder::new().sort("age", SortOrder::Descending);
        assert_eq!(qb.sort, Some(("age".to_string(), SortOrder::Descending)));
    }

    #[test]
    fn test_query_builder_limit() {
        let qb = QueryBuilder::new().limit(10);
        assert_eq!(qb.limit, Some(10));
    }

    #[test]
    fn test_query_builder_offset() {
        let qb = QueryBuilder::new().offset(5);
        assert_eq!(qb.offset, Some(5));
    }

    #[test]
    fn test_query_builder_projection() {
        let qb = QueryBuilder::new().projection(vec!["name", "age"]);
        assert_eq!(qb.projection, Some(vec!["name".to_string(), "age".to_string()]));
    }

    #[test]
    fn test_query_builder_and() {
        let qb = QueryBuilder::new()
            .filter("age", Operator::GreaterThan, json!(18))
            .and(Filter::Equals("status".to_string(), json!("active")));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::And(left, right) => {
                match **left {
                    Filter::GreaterThan(ref field, _) => assert_eq!(field, "age"),
                    _ => panic!("Expected GreaterThan in left"),
                }
                match **right {
                    Filter::Equals(ref field, _) => assert_eq!(field, "status"),
                    _ => panic!("Expected Equals in right"),
                }
            },
            _ => panic!("Expected And filter"),
        }
    }

    #[test]
    fn test_query_builder_or() {
        let qb = QueryBuilder::new()
            .filter("age", Operator::GreaterThan, json!(18))
            .or(Filter::Equals("status".to_string(), json!("active")));
        assert_eq!(qb.filters.len(), 1);
        match &qb.filters[0] {
            Filter::Or(left, right) => {
                match **left {
                    Filter::GreaterThan(ref field, _) => assert_eq!(field, "age"),
                    _ => panic!("Expected GreaterThan in left"),
                }
                match **right {
                    Filter::Equals(ref field, _) => assert_eq!(field, "status"),
                    _ => panic!("Expected Equals in right"),
                }
            },
            _ => panic!("Expected Or filter"),
        }
    }

    #[test]
    fn test_query_builder_build() {
        let query = QueryBuilder::new()
            .filter("age", Operator::GreaterThan, json!(18))
            .sort("name", SortOrder::Ascending)
            .limit(10)
            .offset(5)
            .projection(vec!["name", "age"])
            .build();

        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.sort, Some(("name".to_string(), SortOrder::Ascending)));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(5));
        assert_eq!(query.projection, Some(vec!["name".to_string(), "age".to_string()]));
    }
}
