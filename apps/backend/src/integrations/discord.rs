use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    pub fields: Vec<DiscordField>,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

#[derive(Debug, Serialize)]
struct DiscordWebhookPayload {
    username: String,
    avatar_url: String,
    content: String,
    embeds: Vec<DiscordEmbedPayload>,
}

#[derive(Debug, Serialize)]
struct DiscordEmbedPayload {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordFieldPayload>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct DiscordFieldPayload {
    name: String,
    value: String,
    inline: bool,
}

pub struct DiscordWebhook;

impl DiscordWebhook {
    pub async fn send_message(
        webhook_url: &str,
        username: &str,
        avatar_url: &str,
        content: &str,
        embeds: &[DiscordEmbed],
    ) -> Result<(), String> {
        if webhook_url.is_empty() {
            return Ok(());
        }

        let embed_payloads: Vec<DiscordEmbedPayload> = embeds
            .iter()
            .map(|e| DiscordEmbedPayload {
                title: e.title.clone(),
                description: e.description.clone(),
                color: e.color,
                fields: e
                    .fields
                    .iter()
                    .map(|f| DiscordFieldPayload {
                        name: f.name.clone(),
                        value: f.value.clone(),
                        inline: f.inline,
                    })
                    .collect(),
                timestamp: e.timestamp.clone(),
            })
            .collect();

        let payload = DiscordWebhookPayload {
            username: username.into(),
            avatar_url: avatar_url.into(),
            content: content.into(),
            embeds: embed_payloads,
        };

        let client = reqwest::Client::new();
        let resp = client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send Discord webhook: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Discord webhook returned status: {}",
                resp.status()
            ));
        }

        Ok(())
    }

    pub async fn send_alert(
        webhook_url: &str,
        username: &str,
        avatar_url: &str,
        gpu_name: &str,
        metric: &str,
        value: f64,
        threshold: f64,
        severity: &str,
    ) -> Result<(), String> {
        let color = match severity {
            "critical" => 0xe74c3c,
            "warning" => 0xf39c12,
            "info" => 0x3498db,
            _ => 0x95a5a6,
        };

        let embed = DiscordEmbed {
            title: format!("GPU Alert: {}", metric),
            description: format!("{} has exceeded the threshold", metric),
            color,
            fields: vec![
                DiscordField {
                    name: "GPU".into(),
                    value: gpu_name.into(),
                    inline: true,
                },
                DiscordField {
                    name: "Value".into(),
                    value: format!("{:.1}", value),
                    inline: true,
                },
                DiscordField {
                    name: "Threshold".into(),
                    value: format!("{:.1}", threshold),
                    inline: true,
                },
                DiscordField {
                    name: "Severity".into(),
                    value: severity.to_uppercase(),
                    inline: true,
                },
            ],
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        Self::send_message(webhook_url, username, avatar_url, "", &[embed]).await
    }

    pub async fn send_report(
        webhook_url: &str,
        username: &str,
        avatar_url: &str,
        gpu_name: &str,
        metrics: &[(String, f64, String)],
    ) -> Result<(), String> {
        let fields: Vec<DiscordField> = metrics
            .iter()
            .map(|(name, value, unit)| DiscordField {
                name: name.clone(),
                value: format!("{:.1} {}", value, unit),
                inline: true,
            })
            .collect();

        let embed = DiscordEmbed {
            title: format!("GPU Status Report — {}", gpu_name),
            description: "Current GPU metrics snapshot".into(),
            color: 0x2ecc71,
            fields,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        Self::send_message(webhook_url, username, avatar_url, "", &[embed]).await
    }

    pub async fn test_connection(webhook_url: &str) -> Result<String, String> {
        let embed = DiscordEmbed {
            title: "OverDrive — Connection Test".into(),
            description: "Your Discord webhook is configured correctly.".into(),
            color: 0x2ecc71,
            fields: vec![DiscordField {
                name: "Timestamp".into(),
                value: chrono::Utc::now().to_rfc3339(),
                inline: false,
            }],
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        Self::send_message(webhook_url, "OverDrive", "", "", &[embed]).await?;
        Ok("Discord webhook test successful".into())
    }
}
