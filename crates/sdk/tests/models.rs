use ndc_sdk::{json_response::JsonResponse, models};
use serde_json::json;

type TestError = Box<dyn std::error::Error + Send + Sync>;
type TestResult = Result<(), TestError>;

#[tokio::test]
async fn capabilities_report_ndc_spec_0_2_15() -> TestResult {
    let response =
        ndc_sdk_core::schema::get_capabilities::<ndc_sdk::connector::example::Example>().await;
    let capabilities = response.into_value::<TestError>()?;

    assert_eq!(capabilities.version, "0.2.15");
    assert_eq!(capabilities.version, models::VERSION);
    Ok(())
}

#[test]
fn relational_queries_preserve_updated_aggregates_and_window_expressions() -> TestResult {
    let query = json!({
        "root_relation": {
            "type": "project",
            "input": {"type": "from", "collection": "test_collection", "columns": ["value"]},
            "exprs": [
                {"type": "first_value", "expr": {"type": "column", "index": 0},
                 "order_by": [{"expr": {"type": "column", "index": 0},
                               "direction": "asc", "nulls_sort": "nulls_last"}]},
                {"type": "last_value", "expr": {"type": "column", "index": 0}},
                {"type": "lag", "expr": {"type": "column", "index": 0}, "offset": 1,
                 "order_by": [], "partition_by": []},
                {"type": "lead", "expr": {"type": "column", "index": 0}, "offset": 2,
                 "default": {"type": "column", "index": 0}, "order_by": [], "partition_by": []}
            ]
        }
    });
    let response =
        JsonResponse::<models::RelationalQuery>::Serialized(serde_json::to_vec(&query)?.into());
    let parsed = response.into_value::<TestError>()?;
    let models::Relation::Project { ref exprs, .. } = parsed.root_relation else {
        panic!("expected projection");
    };
    assert!(matches!(
        &exprs[0],
        models::RelationalExpression::FirstValue { order_by: Some(order), .. } if order.len() == 1
    ));
    assert!(matches!(
        &exprs[1],
        models::RelationalExpression::LastValue { order_by: None, .. }
    ));
    assert!(matches!(
        &exprs[2],
        models::RelationalExpression::Lag {
            offset: 1,
            default: None,
            ..
        }
    ));
    assert!(matches!(
        &exprs[3],
        models::RelationalExpression::Lead {
            offset: 2,
            default: Some(_),
            ..
        }
    ));
    assert_eq!(serde_json::to_value(parsed)?, query);
    Ok(())
}

#[test]
fn updated_relational_capabilities_round_trip() -> TestResult {
    let aggregate = json!({"first_value": {"order_by": {}}, "last_value": {}});
    let parsed: models::RelationalAggregateExpressionCapabilities =
        serde_json::from_value(aggregate.clone())?;
    assert!(parsed.first_value.as_ref().unwrap().order_by.is_some());
    assert!(parsed.last_value.as_ref().unwrap().order_by.is_none());
    assert_eq!(serde_json::to_value(parsed)?, aggregate);

    let window = json!({"lag": {}, "lead": {}});
    let parsed: models::RelationalWindowExpressionCapabilities =
        serde_json::from_value(window.clone())?;
    assert!(parsed.lag.is_some());
    assert!(parsed.lead.is_some());
    assert_eq!(serde_json::to_value(parsed)?, window);
    Ok(())
}
