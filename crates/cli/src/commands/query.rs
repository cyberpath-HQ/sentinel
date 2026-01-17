use clap::Args;
use sentinel_dbms::futures::{pin_mut, StreamExt as _};
use serde_json::Value;
use tracing::{error, info};

/// Arguments for the query command.
#[derive(Args, Clone, Default)]
pub struct QueryArgs {
    /// Store path
    #[arg(short, long)]
    pub store_path: String,
    /// Collection name
    #[arg(short, long)]
    pub collection: String,
    /// Passphrase for decrypting the signing key
    #[arg(long)]
    pub passphrase: Option<String>,
    /// Filter documents (can be used multiple times)
    /// Syntax: field=value, field>value, field<value, field>=value, field<=value,
    /// field~substring, field^prefix, field$suffix, field in:value1,value2, field exists:true/false
    #[arg(long, value_name = "filter")]
    pub filter:     Vec<String>,
    /// Sort by field (field:asc or field:desc)
    #[arg(long, value_name = "field:order")]
    pub sort:       Option<String>,
    /// Limit number of results
    #[arg(long)]
    pub limit:      Option<usize>,
    /// Skip number of results
    #[arg(long)]
    pub offset:     Option<usize>,
    /// Project fields (comma-separated)
    #[arg(long, value_name = "field1,field2")]
    pub project:    Option<String>,
    /// Output format: json or table
    #[arg(long, default_value = "json")]
    pub format:     String,
}

/// Query documents in a Sentinel collection.
///
/// This function allows complex querying of documents with filters, sorting,
/// pagination, and field projection.
///
/// # Arguments
/// * `args` - The parsed command-line arguments for query.
///
/// # Returns
/// Returns `Ok(())` on success, or a `SentinelError` on failure.
///
/// # Examples
/// ```rust,no_run
/// use sentinel_cli::commands::query::{run, QueryArgs};
///
/// let args = QueryArgs {
///     store_path: "/tmp/my_store".to_string(),
///     collection: "users".to_string(),
///     passphrase: None,
///     filter:     vec![
///         "age>25".to_string(),
///         "city=NYC".to_string(),
///         "name~Alice".to_string(),
///         "status in:active,inactive".to_string(),
///         "email exists:true".to_string(),
///     ],
///     sort:       Some("name:asc".to_string()),
///     limit:      Some(10),
///     offset:     None,
///     project:    Some("name,email".to_string()),
///     format:     "json".to_string(),
/// };
/// run(args).await?;
/// ```
pub async fn run(args: QueryArgs) -> sentinel_dbms::Result<()> {
    let store_path = args.store_path;
    let collection = args.collection;
    info!(
        "Querying documents in collection '{}' in store {}",
        collection, store_path
    );

    let store = sentinel_dbms::Store::new(&store_path, args.passphrase.as_deref()).await?;
    let coll = store.collection(&collection).await?;

    // Build query
    let mut query_builder = sentinel_dbms::QueryBuilder::new();

    // Parse filters
    for filter_str in &args.filter {
        let filter = parse_filter(filter_str)?;
        query_builder = match filter {
            ParsedFilter::Equals(field, value) => query_builder.filter(&field, sentinel_dbms::Operator::Equals, value),
            ParsedFilter::GreaterThan(field, value) => {
                query_builder.filter(&field, sentinel_dbms::Operator::GreaterThan, value)
            },
            ParsedFilter::LessThan(field, value) => {
                query_builder.filter(&field, sentinel_dbms::Operator::LessThan, value)
            },
            ParsedFilter::GreaterOrEqual(field, value) => {
                query_builder.filter(&field, sentinel_dbms::Operator::GreaterOrEqual, value)
            },
            ParsedFilter::LessOrEqual(field, value) => {
                query_builder.filter(&field, sentinel_dbms::Operator::LessOrEqual, value)
            },
            ParsedFilter::Contains(field, substring) => {
                query_builder.filter(
                    &field,
                    sentinel_dbms::Operator::Contains,
                    Value::String(substring),
                )
            },
            ParsedFilter::StartsWith(field, prefix) => {
                query_builder.filter(
                    &field,
                    sentinel_dbms::Operator::StartsWith,
                    Value::String(prefix),
                )
            },
            ParsedFilter::EndsWith(field, suffix) => {
                query_builder.filter(
                    &field,
                    sentinel_dbms::Operator::EndsWith,
                    Value::String(suffix),
                )
            },
            ParsedFilter::In(field, values) => {
                query_builder.filter(&field, sentinel_dbms::Operator::In, Value::Array(values))
            },
            ParsedFilter::Exists(field, exists) => {
                query_builder.filter(&field, sentinel_dbms::Operator::Exists, Value::Bool(exists))
            },
        };
    }

    // Parse sort
    if let Some(sort_str) = &args.sort {
        let (field, order) = parse_sort(sort_str)?;
        let sort_order = match order.as_str() {
            "asc" => sentinel_dbms::SortOrder::Ascending,
            "desc" => sentinel_dbms::SortOrder::Descending,
            _ => {
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: format!("Invalid sort order: {}", order),
                })
            },
        };
        query_builder = query_builder.sort(&field, sort_order);
    }

    // Set limit and offset
    if let Some(limit) = args.limit {
        query_builder = query_builder.limit(limit);
    }
    if let Some(offset) = args.offset {
        query_builder = query_builder.offset(offset);
    }

    // Parse projection
    if let Some(project_str) = &args.project {
        let fields: Vec<&str> = project_str.split(',').map(|s| s.trim()).collect();
        query_builder = query_builder.projection(fields);
    }

    let query = query_builder.build();

    match coll.query(query).await {
        Ok(result) => {
            let documents_stream = result.documents;
            pin_mut!(documents_stream);

            let mut count = 0;
            let mut has_printed_header = false;

            // Process documents one by one to avoid loading all into memory
            while let Some(doc_result) = documents_stream.next().await {
                match doc_result {
                    Ok(doc) => {
                        count += 1;

                        match args.format.as_str() {
                            "json" => {
                                #[allow(clippy::print_stdout, reason = "CLI output")]
                                {
                                    println!("{}", serde_json::to_string_pretty(doc.data()).unwrap());
                                }
                            },
                            "table" => {
                                if !has_printed_header {
                                    #[allow(clippy::print_stdout, reason = "CLI output")]
                                    {
                                        println!("ID");
                                        println!("--");
                                    }
                                    has_printed_header = true;
                                }
                                #[allow(clippy::print_stdout, reason = "CLI output")]
                                {
                                    println!("{}", doc.id());
                                }
                            },
                            _ => {
                                return Err(sentinel_dbms::SentinelError::ConfigError {
                                    message: format!("Invalid format: {}", args.format),
                                });
                            },
                        }
                    },
                    Err(e) => {
                        error!("Error processing document in query results: {}", e);
                        return Err(e);
                    },
                }
            }

            info!(
                "Query returned {} documents (total: {})",
                count, result.total_count.unwrap_or(0)
            );

            // Handle empty results for table format
            if count == 0 && args.format == "table" {
                #[allow(clippy::print_stdout, reason = "CLI output")]
                {
                    println!("No documents found");
                }
            }

            Ok(())
        },
        Err(e) => {
            error!(
                "Failed to query documents in collection '{}' in store {}: {}",
                collection, store_path, e
            );
            Err(e)
        },
    }
}

/// Parsed filter from command line string.
enum ParsedFilter {
    Equals(String, Value),
    GreaterThan(String, Value),
    LessThan(String, Value),
    GreaterOrEqual(String, Value),
    LessOrEqual(String, Value),
    Contains(String, String),
    StartsWith(String, String),
    EndsWith(String, String),
    In(String, Vec<Value>),
    Exists(String, bool),
}

/// Parse a filter string like "field=value" or "field>value".
fn parse_filter(filter_str: &str) -> sentinel_dbms::Result<ParsedFilter> {
    // Check for special syntaxes first
    if let Some(exists_pos) = filter_str.find(" exists:") {
        let field = filter_str[.. exists_pos].to_string();
        let value_str = &filter_str[exists_pos + 8 ..]; // " exists:" is 8 chars
        let exists = match value_str {
            "true" => true,
            "false" => false,
            _ => {
                return Err(sentinel_dbms::SentinelError::ConfigError {
                    message: format!(
                        "Invalid exists value: {}, expected 'true' or 'false'",
                        value_str
                    ),
                })
            },
        };
        return Ok(ParsedFilter::Exists(field, exists));
    }

    if let Some(in_pos) = filter_str.find(" in:") {
        let field = filter_str[.. in_pos].to_string();
        let values_str = &filter_str[in_pos + 4 ..]; // " in:" is 4 chars
        let values: Vec<Value> = values_str
            .split(',')
            .map(|s| parse_value(s.trim()))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(ParsedFilter::In(field, values));
    }

    // Check for comparison operators (longest first)
    if let Some(ge_pos) = filter_str.find(">=") {
        let field = filter_str[.. ge_pos].to_string();
        let value_str = &filter_str[ge_pos + 2 ..];
        let value = parse_value(value_str)?;
        Ok(ParsedFilter::GreaterOrEqual(field, value))
    }
    else if let Some(le_pos) = filter_str.find("<=") {
        let field = filter_str[.. le_pos].to_string();
        let value_str = &filter_str[le_pos + 2 ..];
        let value = parse_value(value_str)?;
        Ok(ParsedFilter::LessOrEqual(field, value))
    }
    else if let Some(eq_pos) = filter_str.find('=') {
        let field = filter_str[.. eq_pos].to_string();
        let value_str = &filter_str[eq_pos + 1 ..];
        let value = parse_value(value_str)?;
        Ok(ParsedFilter::Equals(field, value))
    }
    else if let Some(gt_pos) = filter_str.find('>') {
        let field = filter_str[.. gt_pos].to_string();
        let value_str = &filter_str[gt_pos + 1 ..];
        let value = parse_value(value_str)?;
        Ok(ParsedFilter::GreaterThan(field, value))
    }
    else if let Some(lt_pos) = filter_str.find('<') {
        let field = filter_str[.. lt_pos].to_string();
        let value_str = &filter_str[lt_pos + 1 ..];
        let value = parse_value(value_str)?;
        Ok(ParsedFilter::LessThan(field, value))
    }
    else if let Some(contains_pos) = filter_str.find('~') {
        let field = filter_str[.. contains_pos].to_string();
        let substring = filter_str[contains_pos + 1 ..].to_string();
        Ok(ParsedFilter::Contains(field, substring))
    }
    else if let Some(starts_pos) = filter_str.find('^') {
        let field = filter_str[.. starts_pos].to_string();
        let prefix = filter_str[starts_pos + 1 ..].to_string();
        Ok(ParsedFilter::StartsWith(field, prefix))
    }
    else if let Some(ends_pos) = filter_str.find('$') {
        let field = filter_str[.. ends_pos].to_string();
        let suffix = filter_str[ends_pos + 1 ..].to_string();
        Ok(ParsedFilter::EndsWith(field, suffix))
    }
    else {
        Err(sentinel_dbms::SentinelError::ConfigError {
            message: format!("Invalid filter format: {}", filter_str),
        })
    }
}

/// Parse a value string into a JSON Value.
fn parse_value(value_str: &str) -> sentinel_dbms::Result<Value> {
    // Try to parse as number first
    if let Ok(num) = value_str.parse::<i64>() {
        return Ok(Value::Number(num.into()));
    }
    if let Ok(num) = value_str.parse::<f64>() {
        if let Some(number) = serde_json::Number::from_f64(num) {
            return Ok(Value::Number(number));
        }
        return Err(sentinel_dbms::SentinelError::ConfigError {
            message: format!("Invalid numeric value: {}", value_str),
        });
    }
    // Try to parse as boolean
    if value_str == "true" {
        return Ok(Value::Bool(true));
    }
    if value_str == "false" {
        return Ok(Value::Bool(false));
    }
    // Default to string
    Ok(Value::String(value_str.to_owned()))
}

/// Parse a sort string like "field:asc".
fn parse_sort(sort_str: &str) -> sentinel_dbms::Result<(String, String)> {
    let parts: Vec<&str> = sort_str.split(':').collect();
    if parts.len() != 2 {
        return Err(sentinel_dbms::SentinelError::ConfigError {
            message: format!("Invalid sort format: {}", sort_str),
        });
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;

    /// Test successful query.
    #[tokio::test]
    async fn test_query_success() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 25, "city": "NYC"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob", "age": 30, "city": "LA"}))
            .await
            .unwrap();
        collection
            .insert("doc3", json!({"name": "Charlie", "age": 35, "city": "NYC"}))
            .await
            .unwrap();

        // Test query command
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["city=NYC".to_string()],
            sort:       Some("age:asc".to_string()),
            limit:      Some(10),
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with no results.
    #[tokio::test]
    async fn test_query_no_results() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let _collection = store.collection("test_collection").await.unwrap();

        // Test query command on empty collection
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec![],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with table format.
    #[tokio::test]
    async fn test_query_table_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();

        // Test query command with table format
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec![],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "table".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with table format and no results.
    #[tokio::test]
    async fn test_query_table_format_no_results() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let _collection = store.collection("test_collection").await.unwrap();

        // Test query command with table format on empty collection
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec![],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "table".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test filter parsing.
    #[test]
    fn test_parse_filter() {
        // Test equals
        let filter = parse_filter("name=Alice").unwrap();
        match filter {
            ParsedFilter::Equals(field, value) => {
                assert_eq!(field, "name");
                assert_eq!(value, Value::String("Alice".to_string()));
            },
            _ => panic!("Expected Equals"),
        }

        // Test greater than
        let filter = parse_filter("age>25").unwrap();
        match filter {
            ParsedFilter::GreaterThan(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, Value::Number(25.into()));
            },
            _ => panic!("Expected GreaterThan"),
        }

        // Test less than
        let filter = parse_filter("age<30").unwrap();
        match filter {
            ParsedFilter::LessThan(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, Value::Number(30.into()));
            },
            _ => panic!("Expected LessThan"),
        }

        // Test greater or equal
        let filter = parse_filter("age>=25").unwrap();
        match filter {
            ParsedFilter::GreaterOrEqual(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, Value::Number(25.into()));
            },
            _ => panic!("Expected GreaterOrEqual"),
        }

        // Test less or equal
        let filter = parse_filter("age<=30").unwrap();
        match filter {
            ParsedFilter::LessOrEqual(field, value) => {
                assert_eq!(field, "age");
                assert_eq!(value, Value::Number(30.into()));
            },
            _ => panic!("Expected LessOrEqual"),
        }

        // Test contains
        let filter = parse_filter("name~Ali").unwrap();
        match filter {
            ParsedFilter::Contains(field, substring) => {
                assert_eq!(field, "name");
                assert_eq!(substring, "Ali");
            },
            _ => panic!("Expected Contains"),
        }

        // Test starts with
        let filter = parse_filter("name^Al").unwrap();
        match filter {
            ParsedFilter::StartsWith(field, prefix) => {
                assert_eq!(field, "name");
                assert_eq!(prefix, "Al");
            },
            _ => panic!("Expected StartsWith"),
        }

        // Test ends with
        let filter = parse_filter("name$ce").unwrap();
        match filter {
            ParsedFilter::EndsWith(field, suffix) => {
                assert_eq!(field, "name");
                assert_eq!(suffix, "ce");
            },
            _ => panic!("Expected EndsWith"),
        }

        // Test in
        let filter = parse_filter("city in:NYC,LA").unwrap();
        match filter {
            ParsedFilter::In(field, values) => {
                assert_eq!(field, "city");
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], Value::String("NYC".to_string()));
                assert_eq!(values[1], Value::String("LA".to_string()));
            },
            _ => panic!("Expected In"),
        }

        // Test exists true
        let filter = parse_filter("email exists:true").unwrap();
        match filter {
            ParsedFilter::Exists(field, exists) => {
                assert_eq!(field, "email");
                assert_eq!(exists, true);
            },
            _ => panic!("Expected Exists"),
        }

        // Test exists false
        let filter = parse_filter("email exists:false").unwrap();
        match filter {
            ParsedFilter::Exists(field, exists) => {
                assert_eq!(field, "email");
                assert_eq!(exists, false);
            },
            _ => panic!("Expected Exists"),
        }

        // Test invalid filter format
        assert!(parse_filter("invalid").is_err());

        // Test invalid exists value
        assert!(parse_filter("field exists:maybe").is_err());
    }

    /// Test value parsing.
    #[test]
    fn test_parse_value() {
        assert_eq!(parse_value("42").unwrap(), Value::Number(42.into()));
        assert_eq!(
            parse_value("3.14").unwrap(),
            Value::Number(serde_json::Number::from_f64(3.14).unwrap())
        );
        assert_eq!(parse_value("true").unwrap(), Value::Bool(true));
        assert_eq!(parse_value("false").unwrap(), Value::Bool(false));
        assert_eq!(
            parse_value("hello").unwrap(),
            Value::String("hello".to_string())
        );
    }

    /// Test sort parsing.
    #[test]
    fn test_parse_sort() {
        let (field, order) = parse_sort("name:asc").unwrap();
        assert_eq!(field, "name");
        assert_eq!(order, "asc");

        let (field, order) = parse_sort("age:desc").unwrap();
        assert_eq!(field, "age");
        assert_eq!(order, "desc");

        // Test invalid sort format
        assert!(parse_sort("invalid").is_err());
        assert!(parse_sort("field:order:extra").is_err());
    }

    /// Test query with invalid sort order.
    #[tokio::test]
    async fn test_query_invalid_sort_order() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test".to_string(),
            passphrase: None,
            filter:     vec![],
            sort:       Some("name:invalid".to_string()),
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        // This should fail due to invalid sort order
        assert!(run(args).await.is_err());
    }

    /// Test query with greater than filter.
    #[tokio::test]
    async fn test_query_filter_greater_than() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob", "age": 30}))
            .await
            .unwrap();

        // Test query with greater than filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["age>25".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with less than filter.
    #[tokio::test]
    async fn test_query_filter_less_than() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob", "age": 30}))
            .await
            .unwrap();

        // Test query with less than filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["age<30".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with contains filter.
    #[tokio::test]
    async fn test_query_filter_contains() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();

        // Test query with contains filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["name~Ali".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with starts with filter.
    #[tokio::test]
    async fn test_query_filter_starts_with() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();

        // Test query with starts with filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["name^Al".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with ends with filter.
    #[tokio::test]
    async fn test_query_filter_ends_with() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();

        // Test query with ends with filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["name$ce".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with in filter.
    #[tokio::test]
    async fn test_query_filter_in() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert("doc1", json!({"city": "NYC"}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"city": "LA"}))
            .await
            .unwrap();
        collection
            .insert("doc3", json!({"city": "Chicago"}))
            .await
            .unwrap();

        // Test query with in filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["city in:NYC,LA".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with exists filter.
    #[tokio::test]
    async fn test_query_filter_exists() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert test documents
        collection
            .insert(
                "doc1",
                json!({"name": "Alice", "email": "alice@example.com"}),
            )
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"name": "Bob"}))
            .await
            .unwrap();

        // Test query with exists true filter
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec!["email exists:true".to_string()],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "json".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_ok());
    }

    /// Test query with invalid format.
    #[tokio::test]
    async fn test_query_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // Setup store and collection
        let store = sentinel_dbms::Store::new(&store_path, None).await.unwrap();
        let collection = store.collection("test_collection").await.unwrap();

        // Insert a test document so the query processes something
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();

        // Test query with invalid format
        let args = QueryArgs {
            store_path: store_path.to_string_lossy().to_string(),
            collection: "test_collection".to_string(),
            passphrase: None,
            filter:     vec![],
            sort:       None,
            limit:      None,
            offset:     None,
            project:    None,
            format:     "invalid".to_string(),
        };

        let result = run(args).await;
        assert!(result.is_err());
    }
}
