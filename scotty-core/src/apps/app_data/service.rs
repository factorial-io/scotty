use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Serialize, Clone, ToSchema, ToResponse)]
pub struct ServicePortMapping {
    pub service: String,
    pub port: u32,
    pub domains: Vec<String>,
}

impl ServicePortMapping {
    /// Returns the effective domains for this service.
    ///
    /// If custom domains are configured, returns those.
    /// Otherwise, returns the auto-generated domain: `{service}.{app_domain}`.
    pub fn get_domains(&self, app_domain: &str) -> Vec<String> {
        if !self.domains.is_empty() {
            self.domains.clone()
        } else if !app_domain.is_empty() {
            vec![format!("{}.{}", self.service, app_domain)]
        } else {
            vec![]
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DomainField {
    Single { domain: String },
    Multiple { domains: Vec<String> },
}

impl<'de> Deserialize<'de> for ServicePortMapping {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the incoming JSON into a temporary map
        #[derive(Deserialize)]
        struct Temp {
            service: String,
            port: u32,
            #[serde(flatten)]
            domain_field: Option<DomainField>,
        }

        // Use the Temp struct to parse and transform into ServicePortMapping
        let Temp {
            service,
            port,
            domain_field,
        } = Temp::deserialize(deserializer)?;

        // Map the domain field to the `domains` field in ServicePortMapping
        let domains = match domain_field {
            None => vec![],
            Some(DomainField::Single { domain }) => vec![domain],
            Some(DomainField::Multiple { domains }) => domains,
        };

        Ok(ServicePortMapping {
            service,
            port,
            domains,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_service_port_mapping_deserialization() {
        // Test no domain
        let json = json!({
            "service": "web",
            "port": 8080,
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "web");
        assert_eq!(mapping.port, 8080);
        assert_eq!(mapping.domains.len(), 0);

        // Test single domain
        let json = json!({
            "service": "web",
            "port": 8080,
            "domain": "example.com"
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "web");
        assert_eq!(mapping.port, 8080);
        assert_eq!(mapping.domains, vec!["example.com"]);

        // Test multiple domains
        let json = json!({
            "service": "api",
            "port": 3000,
            "domains": ["api1.com", "api2.com"]
        });
        let mapping: ServicePortMapping = serde_json::from_value(json).unwrap();
        assert_eq!(mapping.service, "api");
        assert_eq!(mapping.port, 3000);
        assert_eq!(mapping.domains, vec!["api1.com", "api2.com"]);
    }

    #[test]
    fn test_get_domains_with_custom_domains() {
        let mapping = ServicePortMapping {
            service: "web".to_string(),
            port: 8080,
            domains: vec!["custom.example.com".to_string()],
        };
        assert_eq!(
            mapping.get_domains("myapp.apps.example.com"),
            vec!["custom.example.com"]
        );
    }

    #[test]
    fn test_get_domains_auto_generated() {
        let mapping = ServicePortMapping {
            service: "web".to_string(),
            port: 8080,
            domains: vec![],
        };
        assert_eq!(
            mapping.get_domains("myapp.apps.example.com"),
            vec!["web.myapp.apps.example.com"]
        );
    }

    #[test]
    fn test_get_domains_empty_app_domain() {
        let mapping = ServicePortMapping {
            service: "web".to_string(),
            port: 8080,
            domains: vec![],
        };
        let result: Vec<String> = vec![];
        assert_eq!(mapping.get_domains(""), result);
    }
}
