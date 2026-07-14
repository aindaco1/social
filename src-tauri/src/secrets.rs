use crate::domain::{ServiceCredentialFieldStatus, ServiceCredentialStatus, ServiceSummary};
use keyring::Entry;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
struct ServiceCredentialDefinition {
    service: &'static str,
    label: &'static str,
    group: &'static str,
    fields: &'static [ServiceCredentialFieldDefinition],
}

#[derive(Debug, Clone)]
struct ServiceCredentialFieldDefinition {
    field: &'static str,
    label: &'static str,
    env_vars: &'static [&'static str],
}

#[derive(Debug)]
pub enum SecretError {
    Keychain(String),
    UnknownField {
        service: String,
        field: String,
    },
    MissingCredential {
        label: &'static str,
        field: &'static str,
        env_vars: &'static [&'static str],
    },
    MissingStoredCredential {
        service: String,
        field: String,
    },
}

impl Display for SecretError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keychain(error) => write!(formatter, "keychain error: {error}"),
            Self::UnknownField { service, field } => {
                write!(formatter, "unknown credential field {service}.{field}")
            }
            Self::MissingCredential {
                label,
                field,
                env_vars,
                ..
            } => write!(
                formatter,
                "{label} {field} is required; set {}",
                env_vars.join(" or ")
            ),
            Self::MissingStoredCredential { service, field } => {
                write!(formatter, "missing stored credential {service}.{field}")
            }
        }
    }
}

impl Error for SecretError {}

const CLIENT_ID: &str = "client_id";
const CLIENT_SECRET: &str = "client_secret";
const KEYCHAIN_SERVICE: &str = "com.dustwave.social";

const SERVICE_CREDENTIALS: &[ServiceCredentialDefinition] = &[
    ServiceCredentialDefinition {
        service: "twitter",
        label: "X/Twitter",
        group: "social",
        fields: &[
            ServiceCredentialFieldDefinition {
                field: CLIENT_ID,
                label: "API Key",
                env_vars: &["DUSTWAVE_TWITTER_CLIENT_ID", "TWITTER_CLIENT_ID"],
            },
            ServiceCredentialFieldDefinition {
                field: CLIENT_SECRET,
                label: "API Secret",
                env_vars: &["DUSTWAVE_TWITTER_CLIENT_SECRET", "TWITTER_CLIENT_SECRET"],
            },
        ],
    },
    ServiceCredentialDefinition {
        service: "facebook",
        label: "Facebook/Meta",
        group: "social",
        fields: &[
            ServiceCredentialFieldDefinition {
                field: CLIENT_ID,
                label: "App ID",
                env_vars: &["DUSTWAVE_FACEBOOK_CLIENT_ID", "FACEBOOK_CLIENT_ID"],
            },
            ServiceCredentialFieldDefinition {
                field: CLIENT_SECRET,
                label: "App Secret",
                env_vars: &["DUSTWAVE_FACEBOOK_CLIENT_SECRET", "FACEBOOK_CLIENT_SECRET"],
            },
        ],
    },
    ServiceCredentialDefinition {
        service: "unsplash",
        label: "Unsplash",
        group: "media",
        fields: &[ServiceCredentialFieldDefinition {
            field: CLIENT_ID,
            label: "API Key",
            env_vars: &["DUSTWAVE_UNSPLASH_CLIENT_ID", "UNSPLASH_CLIENT_ID"],
        }],
    },
    ServiceCredentialDefinition {
        service: "klipy",
        label: "Klipy",
        group: "media",
        fields: &[ServiceCredentialFieldDefinition {
            field: CLIENT_ID,
            label: "API Key",
            env_vars: &[
                "DUSTWAVE_KLIPY_CLIENT_ID",
                "DUSTWAVE_KLIPY_API_KEY",
                "KLIPY_CLIENT_ID",
                "KLIPY_API_KEY",
            ],
        }],
    },
];

pub fn service_credential_statuses(services: &[ServiceSummary]) -> Vec<ServiceCredentialStatus> {
    service_credential_statuses_with(services, |service, field| {
        resolve_stored_credential(service, field).ok().flatten()
    })
}

pub fn resolve_service_credential(service: &str, field: &str) -> Result<String, SecretError> {
    let definition = SERVICE_CREDENTIALS
        .iter()
        .find(|definition| definition.service == service);
    let Some(definition) = definition else {
        return Err(SecretError::UnknownField {
            service: service.to_string(),
            field: field.to_string(),
        });
    };
    let field_definition = definition
        .fields
        .iter()
        .find(|definition| definition.field == field);
    let Some(field_definition) = field_definition else {
        return Err(SecretError::UnknownField {
            service: service.to_string(),
            field: field.to_string(),
        });
    };

    match resolve_keychain(definition.service, field_definition.field) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => resolve_env(field_definition.env_vars).ok_or(SecretError::MissingCredential {
            label: definition.label,
            field: field_definition.field,
            env_vars: field_definition.env_vars,
        }),
        Err(error) => resolve_env(field_definition.env_vars).ok_or(error),
    }
}

pub fn save_service_credential(
    service: &str,
    field: &str,
    value: &str,
) -> Result<String, SecretError> {
    let definition = SERVICE_CREDENTIALS
        .iter()
        .find(|definition| definition.service == service);
    let Some(definition) = definition else {
        return Err(SecretError::UnknownField {
            service: service.to_string(),
            field: field.to_string(),
        });
    };
    let field_definition = definition
        .fields
        .iter()
        .find(|definition| definition.field == field);
    let Some(field_definition) = field_definition else {
        return Err(SecretError::UnknownField {
            service: service.to_string(),
            field: field.to_string(),
        });
    };

    save_secret_value(definition.service, field_definition.field, value)
}

pub fn save_secret_value(service: &str, field: &str, value: &str) -> Result<String, SecretError> {
    let entry = keychain_entry(service, field)?;

    entry
        .set_password(value)
        .map_err(|error| SecretError::Keychain(error.to_string()))?;

    Ok(secret_ref(service, field))
}

pub fn resolve_secret_value(service: &str, field: &str) -> Result<String, SecretError> {
    resolve_keychain(service, field)?.ok_or_else(|| SecretError::MissingStoredCredential {
        service: service.to_string(),
        field: field.to_string(),
    })
}

pub fn save_account_secret(
    provider: &str,
    provider_id: &str,
    field: &str,
    value: &str,
) -> Result<String, SecretError> {
    let subject = format!("accounts/{provider}/{provider_id}");
    let entry = keychain_entry(&subject, field)?;

    entry
        .set_password(value)
        .map_err(|error| SecretError::Keychain(error.to_string()))?;

    Ok(format!(
        "secret://accounts/{provider}/{provider_id}/{field}"
    ))
}

pub fn resolve_account_secret(
    provider: &str,
    provider_id: &str,
    field: &str,
) -> Result<String, SecretError> {
    let subject = format!("accounts/{provider}/{provider_id}");

    resolve_keychain(&subject, field)?.ok_or_else(|| SecretError::MissingStoredCredential {
        service: subject,
        field: field.to_string(),
    })
}

fn service_credential_statuses_with<F>(
    services: &[ServiceSummary],
    resolver: F,
) -> Vec<ServiceCredentialStatus>
where
    F: Fn(&'static str, &'static ServiceCredentialFieldDefinition) -> Option<String>,
{
    SERVICE_CREDENTIALS
        .iter()
        .map(|definition| {
            let fields = definition
                .fields
                .iter()
                .map(|field| {
                    let configured = resolver(definition.service, field).is_some();

                    ServiceCredentialFieldStatus {
                        field: field.field.to_string(),
                        label: field.label.to_string(),
                        configured,
                        env_vars: field
                            .env_vars
                            .iter()
                            .map(|value| value.to_string())
                            .collect(),
                    }
                })
                .collect::<Vec<_>>();
            let configured = fields.iter().all(|field| field.configured);

            ServiceCredentialStatus {
                service: definition.service.to_string(),
                label: definition.label.to_string(),
                group: definition.group.to_string(),
                active: services
                    .iter()
                    .find(|service| service.name == definition.service)
                    .map(|service| service.active)
                    .unwrap_or(false),
                configured,
                fields,
            }
        })
        .collect()
}

fn resolve_stored_credential(
    service: &str,
    field: &ServiceCredentialFieldDefinition,
) -> Result<Option<String>, SecretError> {
    resolve_keychain(service, field.field)
        .map(|value| value.or_else(|| resolve_env(field.env_vars)))
}

fn resolve_keychain(service: &str, field: &str) -> Result<Option<String>, SecretError> {
    let entry = keychain_entry(service, field)?;

    match entry.get_password() {
        Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(SecretError::Keychain(error.to_string())),
    }
}

fn keychain_entry(service: &str, field: &str) -> Result<Entry, SecretError> {
    Entry::new(KEYCHAIN_SERVICE, &keychain_user(service, field))
        .map_err(|error| SecretError::Keychain(error.to_string()))
}

fn keychain_user(service: &str, field: &str) -> String {
    format!("services/{service}/{field}")
}

fn secret_ref(service: &str, field: &str) -> String {
    format!("secret://services/{service}/{field}")
}

fn resolve_env(names: &[&'static str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| env::var(name).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_service_credential_status_without_exposing_values() {
        let services = vec![ServiceSummary {
            id: 1,
            name: "unsplash".to_string(),
            configuration_secret_ref: "secret://services/unsplash".to_string(),
            configuration: serde_json::json!({}),
            active: true,
        }];
        let statuses = service_credential_statuses_with(&services, |service, field| {
            if service == "unsplash" && field.field == "client_id" {
                Some("secret-value".to_string())
            } else {
                None
            }
        });

        let unsplash = statuses
            .iter()
            .find(|status| status.service == "unsplash")
            .expect("unsplash status should exist");
        assert!(unsplash.active);
        assert!(unsplash.configured);
        assert_eq!(unsplash.fields[0].field, "client_id");
        assert_eq!(
            unsplash.fields[0].env_vars,
            vec!["DUSTWAVE_UNSPLASH_CLIENT_ID", "UNSPLASH_CLIENT_ID"]
        );

        let klipy = statuses
            .iter()
            .find(|status| status.service == "klipy")
            .expect("klipy status should exist");
        assert!(!klipy.active);
        assert!(!klipy.configured);
    }
}
