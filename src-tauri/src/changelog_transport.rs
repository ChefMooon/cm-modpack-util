use crate::domain::CommandError;
use reqwest::blocking::Client;
use std::thread;
use std::time::Duration;

pub const MAX_RESPONSE_BYTES: usize = 1_048_576;
pub const MAX_PAGE_SIZE: u32 = 100;
pub const MAX_RETRIES: u32 = 3;

pub trait RequestControl {
    fn before_request(&self) -> Result<(), CommandError>;
    fn is_cancelled(&self) -> bool;
    fn wait(&self, duration: Duration);
}

struct NoopRequestControl;

impl RequestControl for NoopRequestControl {
    fn before_request(&self) -> Result<(), CommandError> {
        Ok(())
    }

    fn is_cancelled(&self) -> bool {
        false
    }

    fn wait(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModrinthVersionQuery {
    pub loader: Option<String>,
    pub game_version: Option<String>,
    pub limit: u32,
    pub offset: u32,
}

impl Default for ModrinthVersionQuery {
    fn default() -> Self {
        Self {
            loader: None,
            game_version: None,
            limit: MAX_PAGE_SIZE,
            offset: 0,
        }
    }
}

pub trait ModrinthTransport {
    fn get_versions(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
    ) -> Result<String, CommandError>;

    fn get_versions_with_control(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
        control: &dyn RequestControl,
    ) -> Result<String, CommandError> {
        control.before_request()?;
        self.get_versions(project, query)
    }
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
        self.get_versions_with_control_impl(project, query, &NoopRequestControl)
    }

    fn get_versions_with_control(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
        control: &dyn RequestControl,
    ) -> Result<String, CommandError> {
        self.get_versions_with_control_impl(project, query, control)
    }
}

impl HttpModrinthTransport {
    fn get_versions_with_control_impl(
        &self,
        project: &str,
        query: &ModrinthVersionQuery,
        control: &dyn RequestControl,
    ) -> Result<String, CommandError> {
        control.before_request()?;
        let endpoint = format!("https://api.modrinth.com/v2/project/{project}/version");
        eprintln!(
            "[changelog][modrinth] request project={project} endpoint={endpoint} loader={:?} game_version={:?}",
            query.loader,
            query.game_version
        );
        let mut request = self.client.get(&endpoint).query(&[
            ("include_changelog", "true"),
            ("limit", &query.limit.min(MAX_PAGE_SIZE).to_string()),
            ("offset", &query.offset.to_string()),
        ]);
        if let Some(loader) = &query.loader {
            request = request.query(&[("loaders", format!("[\"{loader}\"]"))]);
        }
        if let Some(game_version) = &query.game_version {
            request = request.query(&[("game_versions", format!("[\"{game_version}\"]"))]);
        }
        let response = (0..=MAX_RETRIES)
            .find_map(|attempt| {
                if control.is_cancelled() {
                    return Some(Err(CommandError::new(
                        "changelog_cancelled",
                        "Changelog generation was cancelled",
                    )));
                }
                let response = request
                    .try_clone()?
                    .header("User-Agent", "CM-Modpack-Util/0.1.0")
                    .send();
                match response {
                    Ok(response)
                        if (response.status().is_server_error()
                            || response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS)
                            && attempt < MAX_RETRIES =>
                    {
                        let retry_after = response
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|value| value.to_str().ok())
                            .and_then(|value| value.parse::<u64>().ok())
                            .map(Duration::from_secs)
                            .unwrap_or_else(|| Duration::from_millis(250 * 2_u64.pow(attempt)));
                        control.wait(retry_after.min(Duration::from_secs(8)));
                        None
                    }
                    Ok(response) => Some(Ok(response)),
                    Err(_error) if attempt < MAX_RETRIES => {
                        control.wait(Duration::from_millis(250 * 2_u64.pow(attempt)));
                        None
                    }
                    Err(error) => Some(Err(CommandError::new(
                        "provider_request_failed",
                        "Modrinth request failed",
                    )
                    .with_details(error.to_string()))),
                }
            })
            .ok_or_else(|| {
                CommandError::new(
                    "provider_rate_limit_exhausted",
                    "Modrinth retries were exhausted",
                )
            })??;
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

    #[test]
    fn page_size_is_bounded() {
        assert_eq!(MAX_PAGE_SIZE, 100);
        assert!(ModrinthVersionQuery::default().limit <= MAX_PAGE_SIZE);
    }
}
