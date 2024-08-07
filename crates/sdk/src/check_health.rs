#[derive(Debug, thiserror::Error)]
pub enum HealthCheckError {
    #[error("URL parse error: {0}")]
    ParseError(url::ParseError),
    #[error("request error: {0}")]
    RequestError(reqwest::Error),
    #[error("unsuccessful response with status code: {status}\nbody:\n{body}")]
    UnsuccessfulResponse {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl From<HealthCheckError> for crate::connector::error::ErrorResponse {
    fn from(value: HealthCheckError) -> Self {
        Self::from_error(value)
    }
}

pub async fn check_health(host: Option<String>, port: u16) -> Result<(), HealthCheckError> {
    let url = (|| -> Result<url::Url, url::ParseError> {
        let mut url = reqwest::Url::parse("http://localhost/").unwrap(); // cannot fail
        if let Some(host) = host {
            url.set_host(Some(&host))?;
        }
        url.set_port(Some(port)).unwrap(); // canont fail for HTTP URLs
        url.set_path("/health");
        Ok(url)
    })()
    .map_err(HealthCheckError::ParseError)?;
    let response = reqwest::get(url)
        .await
        .map_err(HealthCheckError::RequestError)?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(HealthCheckError::RequestError)?;
    if status.is_success() {
        Ok(())
    } else {
        Err(HealthCheckError::UnsuccessfulResponse { status, body })
    }
}
