use crate::settings::OnePasswordSettings;

use super::item::Item;

pub async fn get_item(
    onepassword_settings: &OnePasswordSettings,
    vault_id: &str,
    item_id: &str,
) -> anyhow::Result<Item> {
    // Now we have everthing to do the actual lookup
    let url = format!(
        "{}/v1/vaults/{}/items/{}",
        onepassword_settings.server, vault_id, item_id
    );

    let item: Item = reqwest::Client::new()
        .get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", onepassword_settings.jwt_token),
        )
        .send()
        .await?
        .json()
        .await?;

    Ok(item)
}

#[cfg(test)]
mod test {
    use super::*;

    async fn request_item(vault_id: &str, item_id: &str) -> anyhow::Result<Item> {
        let settings = OnePasswordSettings {
            jwt_token: std::env::var("SCOTTY_OP_JWT_TEST_TOKEN")
                .expect("SCOTTY_OP_JWT_TEST_TOKEN not set"),
            server: "https://vault.factorial.io".to_string(),
        };

        get_item(&settings, vault_id, item_id).await
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_item() {
        let result = request_item("n33i6edy47edsntxuj3a7lgiz4", "br47sla67ebp7e57lvpizumq4q").await;
        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result.id, "br47sla67ebp7e57lvpizumq4q");
        assert_eq!(result.vault.id, "n33i6edy47edsntxuj3a7lgiz4");
        assert!(result.has_field("username"));
        assert_eq!(
            result.get_field_value("username", None),
            Some("marvin@factorial.io")
        );
        assert!(result.get_password().is_some());
    }

    #[tokio::test]
    #[ignore] // This indicates the test won't run by default.
    async fn test_get_non_existent_item() {
        let result = request_item("n33i6edy47edsntxuj3a7lgiz4", "r47sla67ebp7e57lvpizumq4q").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_item_2() {
        let result = request_item("n33i6edy47edsntxuj3a7lgiz4", "7j7uxs6rx6w5ggku4zrhimwuye").await;
        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result.id, "7j7uxs6rx6w5ggku4zrhimwuye");
        assert_eq!(result.vault.id, "n33i6edy47edsntxuj3a7lgiz4");
        let password = result.get_password().unwrap();
        assert!(!password.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_item_from_section() {
        let result = request_item("n33i6edy47edsntxuj3a7lgiz4", "ida4izoksx4mwdpvt7wbbq6d7y").await;
        assert!(result.is_ok());
        let result = result.unwrap();

        assert_eq!(result.id, "ida4izoksx4mwdpvt7wbbq6d7y");
        assert_eq!(result.vault.id, "n33i6edy47edsntxuj3a7lgiz4");
        let password = result.get_field_value("server", Some("Section A")).unwrap();
        assert!(!password.is_empty());
    }
}
