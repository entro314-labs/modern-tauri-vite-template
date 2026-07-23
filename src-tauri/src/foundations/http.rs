//! HTTP client builder with user-controlled TLS validation.
//! See `docs/LUDUSAVI_PATTERNS.md` section 9.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Security {
  /// Validate certificates and hostnames (default, recommended).
  #[default]
  Safe,
  /// Skip certificate and hostname validation. Useful for users behind a
  /// corporate proxy with broken cert chains. Expose this only via config.
  Unsafe,
}

pub fn reqwest_client(security: Security) -> reqwest::Client {
  let builder =
    reqwest::ClientBuilder::new().user_agent(crate::foundations::app::USER_AGENT.as_str());

  match security {
    Security::Safe => builder.build().unwrap_or_else(|_| reqwest::Client::new()),
    Security::Unsafe => builder
      .danger_accept_invalid_certs(true)
      .danger_accept_invalid_hostnames(true)
      .build()
      .unwrap_or_else(|_| reqwest::Client::new()),
  }
}

pub fn reqwest_blocking_client(security: Security) -> reqwest::blocking::Client {
  let builder = reqwest::blocking::ClientBuilder::new()
    .user_agent(crate::foundations::app::USER_AGENT.as_str());

  match security {
    Security::Safe => builder
      .build()
      .unwrap_or_else(|_| reqwest::blocking::Client::new()),
    Security::Unsafe => builder
      .danger_accept_invalid_certs(true)
      .danger_accept_invalid_hostnames(true)
      .build()
      .unwrap_or_else(|_| reqwest::blocking::Client::new()),
  }
}
