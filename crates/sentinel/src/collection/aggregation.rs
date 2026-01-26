use futures::TryStreamExt as _;
use serde_json::{json, Value};
use tracing::{debug, trace};

use crate::{filtering::matches_filters, Document, Result};
use super::coll::Collection;

impl Collection {
    /// Performs aggregation operations on documents matching the given filters.
    ///
    /// Supported aggregations:
    /// - `Count`: Count of matching documents
    /// - `Sum(field)`: Sum of numeric values in the specified field
    /// - `Avg(field)`: Average of numeric values in the specified field
    /// - `Min(field)`: Minimum value in the specified field
    /// - `Max(field)`: Maximum value in the specified field
    ///
    /// # Arguments
    ///
    /// * `filters` - Filters to apply before aggregation
    /// * `aggregation` - The aggregation operation to perform
    ///
    /// # Returns
    ///
    /// Returns the aggregated result as a JSON `Value`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sentinel_dbms::{Store, Collection, Filter, Aggregation};
    /// use serde_json::json;
    ///
    /// # async fn example() -> sentinel_dbms::Result<()> {
    /// let store = Store::new("/path/to/data", None).await?;
    /// let collection = store.collection("products").await?;
    ///
    /// // Insert some test data
    /// collection.insert("prod-1", json!({"name": "Widget", "price": 10.0})).await?;
    /// collection.insert("prod-2", json!({"name": "Gadget", "price": 20.0})).await?;
    ///
    /// // Count all products
    /// let count = collection.aggregate(vec![], Aggregation::Count).await?;
    /// assert_eq!(count, json!(2));
    ///
    /// // Sum of all prices
    /// let total = collection.aggregate(vec![], Aggregation::Sum("price".to_string())).await?;
    /// assert_eq!(total, json!(30.0));
    /// # Ok(())
    /// # }
    /// ```
    pub async fn aggregate(&self, filters: Vec<crate::Filter>, aggregation: crate::Aggregation) -> Result<Value> {
        trace!("Performing aggregation: {:?}", aggregation);

        // Get all documents (we'll filter them)
        let mut stream = self.all();

        let mut count = 0usize;
        let mut sum = 0.0f64;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut numeric_count = 0usize;

        while let Some(doc) = stream.try_next().await? {
            // Apply filters
            if !filters.is_empty() {
                let filter_refs: Vec<&crate::Filter> = filters.iter().collect();
                if !matches_filters(&doc, &filter_refs) {
                    continue;
                }
            }

            count = count.saturating_add(1);

            // Extract value for field-based aggregations
            if let crate::Aggregation::Sum(ref field) |
            crate::Aggregation::Avg(ref field) |
            crate::Aggregation::Min(ref field) |
            crate::Aggregation::Max(ref field) = aggregation &&
                let Some(value) = Self::extract_numeric_value(&doc, field)
            {
                sum += value;
                min = min.min(value);
                max = max.max(value);
                numeric_count += 1;
            }
        }

        let result = match aggregation {
            crate::Aggregation::Count => json!(count),
            crate::Aggregation::Sum(_) => json!(sum),
            crate::Aggregation::Avg(_) => {
                if numeric_count == 0 {
                    json!(null)
                }
                else {
                    json!(sum / numeric_count as f64)
                }
            },
            crate::Aggregation::Min(_) => {
                if min == f64::INFINITY {
                    json!(null)
                }
                else {
                    json!(min)
                }
            },
            crate::Aggregation::Max(_) => {
                if max == f64::NEG_INFINITY {
                    json!(null)
                }
                else {
                    json!(max)
                }
            },
        };

        debug!("Aggregation result: {}", result);
        Ok(result)
    }

    /// Extracts a numeric value from a document field for aggregation operations.
    pub fn extract_numeric_value(doc: &Document, field: &str) -> Option<f64> {
        doc.data().get(field).and_then(|v| {
            match *v {
                Value::Number(ref n) => n.as_f64(),
                Value::Null | Value::Bool(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => None,
            }
        })
    }
}
