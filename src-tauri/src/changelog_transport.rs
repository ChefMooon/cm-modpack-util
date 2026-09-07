use crate::domain::CommandError;
use reqwest::blocking::Client;
use std::time::Duration;

pub const MAX_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModrinthVersionQuery {
    pub loader: Option<String>,
    pub game_version: Option<String>,
}

pub trait ModrinthTransport {
    fn get_versions(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
    ) -> Result<String, CommandError>;
}

pub struct HttpModrinthTransport {
    client: Client,
}

impl HttpModrinthTransport {
    pub fn new() -> Result<Self, CommandError> {
        Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map(|client| Self { client })
            .map_err(|error| {
                CommandError::new("provider_unavailable", "Modrinth client could not start")
                    .with_details(error.to_string())
            })
    }
}

impl ModrinthTransport for HttpModrinthTransport {
    fn get_versions(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
    ) -> Result<String, CommandError> {
        let endpoint = format!("https://api.modrinth.com/v2/project/{project}/version");
        eprintln!(
            "[changelog][modrinth] request project={project} endpoint={endpoint} loader={:?} game_version={:?}",
            query.loader,
            query.game_version
        );
        let mut request = self
            .client
            .get(&endpoint)
            .query(&[("include_changelog", "true")]);
        if let Some(loader) = &query.loader {
            request = request.query(&[("loaders", format!("[\"{loader}\"]"))]);
        }
        if let Some(game_version) = &query.game_version {
            request = request.query(&[("game_versions", format!("[\"{game_version}\"]"))]);
        }
        let response = request
            .header("User-Agent", "CM-Modpack-Util/0.0.7")
            .send()
            .map_err(|error| {
                CommandError::new("provider_request_failed", "Modrinth request failed")
                    .with_details(error.to_string())
            })?;
        let status = response.status();
        eprintln!("[changelog][modrinth] response project={project} status={status}");
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(CommandError::new(
                "provider_version_missing",
                "Modrinth does not have the requested version",
            ));
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(CommandError::new(
                "provider_rate_limited",
                "Modrinth rate limited the request; try again later",
            ));
        }
        if !status.is_success() {
            return Err(CommandError::new(
                "provider_request_failed",
                "Modrinth rejected the request",
            )
            .with_details(status.to_string()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(CommandError::new(
                "provider_response_too_large",
                "Modrinth response exceeded the permitted size",
            ));
        }
        let body = response.bytes().map_err(|error| {
            CommandError::new(
                "provider_response_invalid",
                "Modrinth response could not be read",
            )
            .with_details(error.to_string())
        })?;
        eprintln!(
            "[changelog][modrinth] response project={project} bytes={}",
            body.len()
        );
        if body.len() > MAX_RESPONSE_BYTES {
            return Err(CommandError::new(
                "provider_response_too_large",
                "Modrinth response exceeded the permitted size",
            ));
        }
        String::from_utf8(body.to_vec()).map_err(|error| {
            CommandError::new(
                "provider_response_invalid",
                "Modrinth response was not UTF-8",
            )
            .with_details(error.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureTransport;

    impl ModrinthTransport for FixtureTransport {
        fn get_versions(
            &self,
            project: &str,
            _query: &ModrinthVersionQuery,
        ) -> Result<String, CommandError> {
            Ok(format!(
                r#"[{{"project_id":"{project}","id":"version-id","version_number":"1.2.3","changelog":"Fixed"}}]"#
            ))
        }
    }

    #[test]
    fn fixture_transport_can_supply_exact_version_without_network() {
        let response = FixtureTransport
            .get_versions("project-id", &ModrinthVersionQuery::default())
            .expect("fixture response should be available");
        assert!(response.contains("\"version_number\":\"1.2.3\""));
    }

    #[test]
    fn response_limit_is_one_megabyte() {
        assert_eq!(MAX_RESPONSE_BYTES, 1_048_576);
    }
}
