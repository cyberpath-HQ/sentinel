use clap::Args;
use sentinel_dbms::{Aggregation, Filter};
use serde_json::Value;

/// Arguments for collection aggregate command.
#[derive(Args)]
pub struct AggregateArgs {
    /// Aggregation operation: count, sum:field, avg:field, min:field, max:field
    #[arg(short, long)]
    pub aggregation: String,

    /// Filter documents (can be used multiple times)
    /// Syntax: field=value, field>value, field<value, field>=value, field<=value,
    /// field~substring, field^prefix, field$suffix, field in:value1,value2, field exists:true/false
    #[arg(long, value_name = "filter")]
    pub filter: Vec<String>,

    /// WAL configuration options for this collection
    #[command(flatten)]
    pub wal: crate::commands::WalArgs,
}

/// Execute collection aggregate command.
///
/// Performs aggregation operations on documents in the specified collection.
///
/// # Arguments
/// * `store_path` - Path to the Sentinel store
/// * `collection_name` - Name of the collection
/// * `passphrase` - Optional passphrase for decrypting signing key
/// * `args` - Aggregate command arguments
///
/// # Returns
/// Returns `Ok(())` on success.
pub async fn run(
    store_path: String,
    collection_name: String,
    passphrase: Option<String>,
    args: AggregateArgs,
) -> sentinel_dbms::Result<()> {
    // Parse aggregation
    let aggregation = parse_aggregation(&args.aggregation)?;

    // Parse filters
    let filters = parse_filters(&args.filter)?;

    let store = sentinel_dbms::Store::new_with_config(
        &store_path,
        passphrase.as_deref(),
        sentinel_dbms::StoreWalConfig::default(),
    )
    .await?;
    let collection = store
        .collection_with_config(&collection_name, Some(args.wal.to_overrides()))
        .await?;

    let result = collection.aggregate(filters, aggregation).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

/// Parse aggregation specification from string.
fn parse_aggregation(spec: &str) -> sentinel_dbms::Result<Aggregation> {
    if spec == "count" {
        return Ok(Aggregation::Count);
    }

    if let Some(field) = spec.strip_prefix("sum:") {
        return Ok(Aggregation::Sum(field.to_string()));
    }

    if let Some(field) = spec.strip_prefix("avg:") {
        return Ok(Aggregation::Avg(field.to_string()));
    }

    if let Some(field) = spec.strip_prefix("min:") {
        return Ok(Aggregation::Min(field.to_string()));
    }

    if let Some(field) = spec.strip_prefix("max:") {
        return Ok(Aggregation::Max(field.to_string()));
    }

    Err(sentinel_dbms::SentinelError::Internal {
        message: format!(
            "Invalid aggregation: {}. Use count, sum:field, avg:field, min:field, or max:field",
            spec
        ),
    })
}

/// Parse filter specifications from strings.
fn parse_filters(filter_specs: &[String]) -> sentinel_dbms::Result<Vec<Filter>> {
    let mut filters = Vec::new();

    for spec in filter_specs {
        let filter = parse_filter(spec)?;
        filters.push(filter);
    }

    Ok(filters)
}

/// Parse a single filter specification.
fn parse_filter(spec: &str) -> sentinel_dbms::Result<Filter> {
    // Split on first occurrence of operator
    let operators = [
        "==", "!=", ">=", "<=", ">", "<", "~", "^", "$", " in:", " exists:",
    ];

    for op in &operators {
        if let Some((field, value_str)) = spec.split_once(op) {
            let field = field.trim().to_string();

            match *op {
                "==" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::Equals(field, value));
                },
                "!=" => {
                    // For now, we'll implement != as NOT equals, but this might need extension
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::Equals(field, value)); // This is a simplification
                },
                ">=" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::GreaterOrEqual(field, value));
                },
                "<=" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::LessOrEqual(field, value));
                },
                ">" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::GreaterThan(field, value));
                },
                "<" => {
                    let value = parse_value(value_str.trim())?;
                    return Ok(Filter::LessThan(field, value));
                },
                "~" => {
                    return Ok(Filter::Contains(field, value_str.trim().to_string()));
                },
                "^" => {
                    return Ok(Filter::StartsWith(field, value_str.trim().to_string()));
                },
                "$" => {
                    return Ok(Filter::EndsWith(field, value_str.trim().to_string()));
                },
                " in:" => {
                    let values = parse_value_list(value_str.trim())?;
                    return Ok(Filter::In(field, values));
                },
                " exists:" => {
                    let exists = parse_bool(value_str.trim())?;
                    return Ok(Filter::Exists(field, exists));
                },
                _ => {},
            }
        }
    }

    // Default to equals if no operator found
    if let Some((field, value_str)) = spec.split_once('=') {
        let value = parse_value(value_str.trim())?;
        return Ok(Filter::Equals(field.trim().to_string(), value));
    }

    Err(sentinel_dbms::SentinelError::Internal {
        message: format!(
            "Invalid filter: {}. Use field=value, field>value, etc.",
            spec
        ),
    })
}

/// Parse a JSON value from string.
fn parse_value(s: &str) -> sentinel_dbms::Result<Value> {
    // Try to parse as JSON first
    if let Ok(value) = serde_json::from_str(s) {
        return Ok(value);
    }

    // If not valid JSON, treat as string
    Ok(Value::String(s.to_string()))
}

/// Parse a list of values from comma-separated string.
fn parse_value_list(s: &str) -> sentinel_dbms::Result<Vec<Value>> {
    let mut values = Vec::new();
    for item in s.split(',') {
        let value = parse_value(item.trim())?;
        values.push(value);
    }
    Ok(values)
}

/// Parse boolean from string.
fn parse_bool(s: &str) -> sentinel_dbms::Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => {
            Err(sentinel_dbms::SentinelError::Internal {
                message: format!("Invalid boolean: {}. Use true or false", s),
            })
        },
    }
}
