use std::collections::BTreeMap;

use async_trait::async_trait;
use tracing::info_span;
use tracing::Instrument;

use super::*;

#[derive(Clone, Default)]
pub struct Example {}

#[async_trait]
impl ConnectorSetup for Example {
    type Connector = Self;

    async fn parse_configuration(
        &self,
        _configuration_dir: impl AsRef<Path> + Send,
    ) -> Result<<Self as Connector>::Configuration, ParseError> {
        Ok(())
    }

    async fn try_init_state(
        &self,
        _configuration: &<Self as Connector>::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> Result<<Self as Connector>::State, InitializationError> {
        Ok(())
    }
}

#[async_trait]
impl Connector for Example {
    type Configuration = ();
    type State = ();

    fn fetch_metrics(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<(), FetchMetricsError> {
        Ok(())
    }

    async fn health_check(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<(), HealthError> {
        Ok(())
    }

    async fn get_capabilities() -> models::Capabilities {
        models::Capabilities {
            relationships: None,
            query: models::QueryCapabilities {
                variables: None,
                aggregates: None,
                exists: models::ExistsCapabilities {
                    nested_collections: None,
                },
                explain: None,
                nested_fields: models::NestedFieldCapabilities {
                    filter_by: None,
                    order_by: None,
                    aggregates: None,
                },
            },
            mutation: models::MutationCapabilities {
                transactional: None,
                explain: None,
            },
        }
    }

    async fn get_schema(
        _configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>, SchemaError> {
        async {
            info_span!("inside tracing example");
        }
        .instrument(info_span!("tracing example"))
        .await;

        Ok(models::SchemaResponse {
            collections: vec![],
            functions: vec![],
            procedures: vec![],
            object_types: BTreeMap::new(),
            scalar_types: BTreeMap::new(),
        }
        .into())
    }

    async fn query_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        todo!()
    }

    async fn mutation_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError> {
        todo!()
    }

    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>, MutationError> {
        todo!()
    }

    async fn query(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>, QueryError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use axum_test_helper::TestClient;
    use http::StatusCode;

    use super::*;

    #[tokio::test]
    async fn capabilities_match_ndc_spec_version() -> Result<(), Box<dyn Error + Send + Sync>> {
        let state =
            crate::default_main::init_server_state(Example::default(), PathBuf::new()).await?;
        let app = crate::default_main::create_router::<Example>(state, None);

        let client = TestClient::new(app);
        let response = client.get("/capabilities").send().await;

        assert_eq!(response.status(), StatusCode::OK);

        let body: ndc_models::CapabilitiesResponse = response.json().await;
        assert_eq!(body.version, ndc_models::VERSION);
        Ok(())
    }
}
