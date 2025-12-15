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
pub struct AgeGenderMetrics {
    pub age: f32,
    pub male: f32,
    pub female: f32,
    pub child: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionMetrics {
    arousal: f32,
    dominance: f32,
    valence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordMetrics {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperMetrics {
    pub id: i64,
    pub seek: i64,
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub tokens: Vec<i64>,
    pub temperature: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
    pub no_speech_prob: f32,
    pub words: Vec<WordMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetrics {
    pub emotion: EmotionMetrics,
    pub whisper: WhisperMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetrics {
    pub idx: i32, // TODO: add this to Python side
    pub age_gender: AgeGenderMetrics,
    pub talk_percent: f32,
    pub segments: Vec<SegmentMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMsg {
    pub percent_done: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetrics {
    pub duration_seconds: f32,
    pub sample_rate: u32,
    pub channels: u32,
    pub bit_depth: u16,
    pub max_dbfs: f32,
    pub rms: i64,
    pub raw_data_length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetrics {
    pub audio: AudioMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMsg {
    pub error: String,
    pub trace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KafkaAnalysisResponseInner {
    RecordingMetrics(RecordingMetrics),
    ChannelMetrics(ChannelMetrics),
    ProgressMsg(ProgressMsg),
    ErrorMsg(ErrorMsg),
}

pub type KafkaAnalysisResponse = KafkaEnvelope<KafkaAnalysisResponseInner>;
