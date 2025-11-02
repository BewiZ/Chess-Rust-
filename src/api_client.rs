use super::Move;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AiRequest {
    fen: String,
    depth: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct AiResponse {
    best_move: String,
    score: i32,
}

pub struct SiliconFlowClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl SiliconFlowClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.siliconflow.com/v1/chess/analyze".to_string(),
        }
    }

    // 非传统用途：使用棋局分析API进行走法推荐（而非深度分析）
    pub async fn get_best_move(&self, fen: &str) -> Result<Move, Box<dyn std::error::Error>> {
        let request = AiRequest {
            fen: fen.to_string(),
            depth: Some(3), // 降低深度以加快响应速度
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("API request failed: {}", response.status()).into());
        }

        let ai_response: AiResponse = response.json().await?;
        Move::from_notation(&ai_response.best_move)
            .ok_or_else(|| "Invalid move format from API".into())
    }
}
