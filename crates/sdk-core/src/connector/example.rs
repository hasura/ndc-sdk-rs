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
        _configuration_dir: &Path,
    ) -> Result<<Self as Connector>::Configuration> {
        Ok(())
    }

    async fn try_init_state(
        &self,
        _configuration: &<Self as Connector>::Configuration,
        _metrics: &mut prometheus::Registry,
    ) -> Result<<Self as Connector>::State> {
        Ok(())
    }
}

#[async_trait]
impl Connector for Example {
    type Configuration = ();
    type State = ();

    fn connector_name() -> String {
        "example".into()
    }

    fn connector_version() -> String {
        "1.0.0".into()
    }

    fn fetch_metrics(_configuration: &Self::Configuration, _state: &Self::State) -> Result<()> {
        Ok(())
    }

    async fn get_capabilities() -> models::Capabilities {
        models::Capabilities {
            relationships: None,
            query: models::QueryCapabilities {
                variables: None,
                aggregates: None,
                explain: None,
                nested_fields: models::NestedFieldCapabilities {
                    filter_by: None,
                    order_by: None,
                    aggregates: None,
                    nested_collections: None,
                },
                exists: models::ExistsCapabilities {
                    nested_collections: None,
                    unrelated: None,
                    named_scopes: None,
                    nested_scalar_collections: None,
                },
            },
            mutation: models::MutationCapabilities {
                transactional: None,
                explain: None,
            },
            relational_mutation: None,
            relational_query: None,
        }
    }

    async fn get_schema(
        _configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>> {
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
            capabilities: None,
            request_arguments: None,
        }
        .into())
    }

    async fn query_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>> {
        todo!()
    }

    async fn mutation_explain(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>> {
        todo!()
    }

    async fn mutation(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>> {
        todo!()
    }

    async fn query(
        _configuration: &Self::Configuration,
        _state: &Self::State,
        _request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>> {
        todo!()
    }
}
