use std::{io::Write, path::Path};

use crate::{
    connector::{Connector, ConnectorSetup, Result},
    json_response::JsonResponse,
    state::init_server_state,
};

pub async fn get_capabilities<C: Connector>() -> JsonResponse<ndc_models::CapabilitiesResponse> {
    let capabilities = C::get_capabilities().await;
    ndc_models::CapabilitiesResponse {
        version: ndc_models::VERSION.into(),
        capabilities,
    }
    .into()
}

/// Prints a JSON object to the writer containing the ndc schema and capabilities of the connector
pub async fn print_schema_and_capabilities<Setup, W: Write>(
    setup: Setup,
    config_directory: &Path,
    writer: W,
) -> Result<()>
where
    Setup: ConnectorSetup,
    Setup::Connector: Connector + 'static,
    <Setup::Connector as Connector>::Configuration: Clone,
    <Setup::Connector as Connector>::State: Clone,
{
    let server_state = init_server_state(setup, config_directory).await?;

    let schema = Setup::Connector::get_schema(server_state.configuration()).await?;
    let capabilities = get_capabilities::<Setup::Connector>().await;

    print_json_schema_and_capabilities(writer, schema, capabilities)?;

    Ok(())
}

/// This foulness manually writes out a JSON object with schema and capabilities properties.
/// We do it like this to avoid having to deserialize and reserialize any
/// JsonResponse::Serialized values.
fn print_json_schema_and_capabilities<W: Write>(
    mut writer: W,
    schema: JsonResponse<ndc_models::SchemaResponse>,
    capabilities: JsonResponse<ndc_models::CapabilitiesResponse>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    write!(writer, r#"{{"schema":"#)?;
    write_json_response(&mut writer, schema)?;
    write!(writer, r#","capabilities":"#)?;
    write_json_response(&mut writer, capabilities)?;
    writeln!(writer, r#"}}"#)?;

    Ok(())
}

fn write_json_response<W: Write, A: serde::Serialize>(
    writer: &mut W,
    json: JsonResponse<A>,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match json {
        JsonResponse::Value(value) => Ok(serde_json::to_writer(writer, &value)?),
        JsonResponse::Serialized(bytes) => Ok(writer.write_all(&bytes)?),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::connector::{example::Example, Connector};

    use super::{get_capabilities, print_json_schema_and_capabilities};

    #[derive(Debug, serde::Deserialize)]
    #[allow(dead_code)]
    struct SchemaAndCapabilities {
        pub schema: ndc_models::SchemaResponse,
        pub capabilities: ndc_models::CapabilitiesResponse,
    }

    #[test]
    fn test_print_json_schema_and_capabilities_is_valid_json() {
        tokio_test::block_on(async {
            let mut bytes = Cursor::new(vec![]);
            let schema = Example::get_schema(&()).await.unwrap();
            let capabilities = get_capabilities::<Example>().await;
            print_json_schema_and_capabilities(&mut bytes, schema, capabilities).unwrap();

            let bytes = bytes.into_inner();
            serde_json::from_slice::<SchemaAndCapabilities>(&bytes).unwrap();
        });
    }
}
