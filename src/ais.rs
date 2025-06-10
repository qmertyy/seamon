use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use url::Url;
use futures_util::{SinkExt, StreamExt};

pub struct AisStream {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AisMessage {
    #[serde(rename = "MessageType")]
    pub message_type: String,
    #[serde(rename = "MetaData")]
    pub metadata: Metadata,
    #[serde(rename = "Message")]
    pub message: MessageData,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Metadata {
    #[serde(rename = "MMSI")]
    pub mmsi: u32,
    #[serde(rename = "ShipName")]
    pub ship_name: String,
    #[serde(rename = "latitude")]
    pub latitude: f64,
    #[serde(rename = "longitude")]
    pub longitude: f64,
    #[serde(rename = "time_utc")]
    pub time_utc: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageData {
    #[serde(rename = "PositionReport")]
    pub position_report: Option<PositionReport>,
    #[serde(rename = "ShipStaticData")]
    pub ship_static_data: Option<ShipStaticData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PositionReport {
    #[serde(rename = "Cog")]
    pub cog: f64,
    #[serde(rename = "NavigationalStatus")]
    pub navigational_status: u32,
    #[serde(rename = "Sog")]
    pub sog: f64,
    #[serde(rename = "TrueHeading")]
    pub true_heading: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShipStaticData {
    #[serde(rename = "Type")]
    pub ship_type: u32,
    #[serde(rename = "Destination")]
    pub destination: String,
    #[serde(rename = "ImoNumber")]
    pub imo_number: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum AuthMessage {
    AuthError(AuthError),
    Message(AisMessage),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthError {
    pub error: String,
}

impl AisStream {
    pub async fn connect(url: Url, api_key: String) -> Result<Self> {
        let (mut socket, _) = connect_async(url).await?;

        // Send authentication
        let auth_message = serde_json::json!({
            "APIKey": api_key,
            "BoundingBoxes": [[[-180, -90], [180, 90]]], // Global coverage
            "FilterMessageTypes": ["PositionReport", "ShipStaticData"]
        });

        socket
            .send(Message::Text(auth_message.to_string()))
            .await?;

        // Wait for authentication response
        if let Some(msg) = socket.next().await {
            match msg? {
                Message::Binary(data) => {
                    match serde_json::from_slice::<AuthMessage>(&data)? {
                        AuthMessage::AuthError(error) => {
                            return Err(anyhow::anyhow!("Authentication error: {}", error.error));
                        }
                        AuthMessage::Message(_) => {
                            // Authentication successful
                        }
                    }
                }
                _ => {
                    return Err(anyhow::anyhow!("Unexpected authentication response"));
                }
            }
        }

        Ok(Self { socket })
    }

    pub async fn next_message(&mut self) -> Result<Option<AisMessage>> {
        while let Some(msg) = self.socket.next().await {
            match msg? {
                Message::Binary(data) => {
                    match serde_json::from_slice::<AisMessage>(&data) {
                        Ok(message) => return Ok(Some(message)),
                        Err(e) => {
                            tracing::warn!("Failed to parse AIS message: {}", e);
                            continue;
                        }
                    }
                }
                Message::Close(_) => {
                    return Err(anyhow::anyhow!("WebSocket connection closed"));
                }
                _ => continue,
            }
        }
        Ok(None)
    }
}