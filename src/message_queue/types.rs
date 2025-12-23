use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaEnvelope<T> {
    pub id: uuid::Uuid,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequestInner {
    pub download_url: String,
}

pub type AnalysisRequest = KafkaEnvelope<AnalysisRequestInner>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionMetrics {
    arousal: f32,
    dominance: f32,
    valence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub metrics: Vec<MetricCollection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetrics {
    pub idx: i32,
    pub segments: Vec<Segment>,
    pub metrics: Vec<MetricCollection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMsg {
    pub percent_done: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetrics {
    pub metrics: Vec<MetricCollection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMsg {
    pub error: String,
    pub trace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
pub enum KafkaAnalysisResponseInner {
    RecordingMetrics(RecordingMetrics),
    ChannelMetrics(ChannelMetrics),
    ProgressMsg(ProgressMsg),
    ErrorMsg(ErrorMsg),
}

pub type KafkaAnalysisResponse = KafkaEnvelope<KafkaAnalysisResponseInner>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricCollection {
    pub provider: String,
    pub metrics: Vec<Metric>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Metric {
    Int {
        name: String,
        value: i64,
        description: Option<String>,
        unit: Option<String>,
    },
    Float {
        name: String,
        value: f32,
        description: Option<String>,
        unit: Option<String>,
    },
    String {
        name: String,
        value: String,
        description: Option<String>,
        unit: Option<String>,
    },
    Bool {
        name: String,
        value: bool,
        description: Option<String>,
        unit: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_type() {
        let data = r#"
    {
        "type": "int",
        "name": "test",
        "value": 1,
        "description": "test"
    }"#;

        let metric: Metric = serde_json::from_str(data).unwrap();
        assert_eq!(
            metric,
            Metric::Int {
                name: "test".to_string(),
                value: 1,
                description: Some("test".to_string()),
                unit: None
            }
        );
    }
}
