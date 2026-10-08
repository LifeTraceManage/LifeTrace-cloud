//! Ephemeral multimodal report extraction. Report bytes and extracted text are never
//! persisted in Agent sessions, approvals, or the Cloud database.
//! This is a *draft-only* API: the desktop must review and commit locally.

use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use lifetrace_contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use crate::{auth::AuthenticatedPrincipal, error::ApiError, state::AppState};

const MAX_FILES: usize = 8;
const MAX_TOTAL_DECODED_BYTES: usize = 12 * 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_RESPONSE_CHARS: usize = 100_000;

const EXTRACTION_PROMPT: &str = r#"你是一个医疗检查报告的**数据转录**工具，不是临床诊断系统。
直接观察用户提供的报告图片，不使用 OCR 工具。可能有多张、跨页、混合血检和彩超等报告。
输出一个 JSON 对象，禁止 Markdown 代码围栏、前言、诊断推断。格式：
{"reports":[{"title":"检查名称","reportType":"laboratory|ultrasound|ct|mri|xray|ecg|pathology|endoscopy|other","examAt":null,"collectionAt":null,"issuedAt":null,"facility":null,"department":null,"reportNo":null,"bodySite":null,"sourceAssetIds":["对应输入中的 assetId"],"sections":[{"kind":"findings|conclusion|recommendation|other","titleRaw":"原始标题","textRaw":"完整可读原文","sourceAssetId":"文件 id","pageIndex":0}],"observations":[{"nameRaw":"完整原始项目名","kind":"numeric|qualitative|measurement|grade|text|range","valueRaw":"报告的原始值文本","valueNumber":null,"comparator":null,"unitRaw":null,"referenceRangeRaw":null,"sourceFlag":null,"bodySite":null,"laterality":null,"sourceAssetId":"文件 id","pageIndex":0}],"needsReview":false,"reviewReasons":[]}],"groupingWarnings":[]}
严格规则：
1. 每一项可读化验指标都列出，不仅异常项。数字用 valueRaw 原样转录；只有明确数值才提供 valueNumber。保留原始单位、范围与高低标记。
2. 彩超、CT、MRI、病理、胃镜报告，完整保存“检查所见”“结论/提示”等原文段落；额外结构化记录报告已有的尺寸、分级等。
3. 不得把同批上传的不同报告混为一份；不确定是否同报告时拆分并列入 groupingWarnings。
4. 没有明确给出的日期、医院、指标、单位不能猜。examAt、collectionAt、issuedAt 区分。
5. 无法读清的项目放入 reviewReasons，不能凭常识补全。不要生成治疗建议。
6. sourceAssetId 及 sourceAssetIds 必须取自下面的合法列表；pageIndex 对每张图片设为 0。
7. 报告中文保持中文原文，不要仅保留模型摘要。
"#;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractRequest {
    pub images: Vec<ImageInput>,
    #[serde(default)]
    pub instruction: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInput {
    pub asset_id: String,
    pub mime_type: String,
    pub base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractReply {
    pub schema_version: u32,
    pub reports: Vec<ReportDraft>,
    pub grouping_warnings: Vec<String>,
    pub requires_confirmation: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDraft {
    pub title: String,
    pub report_type: String,
    #[serde(default)]
    pub exam_at: Option<String>,
    #[serde(default)]
    pub collection_at: Option<String>,
    #[serde(default)]
    pub issued_at: Option<String>,
    #[serde(default)]
    pub facility: Option<String>,
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default)]
    pub report_no: Option<String>,
    #[serde(default)]
    pub body_site: Option<String>,
    pub source_asset_ids: Vec<String>,
    #[serde(default)]
    pub sections: Vec<SectionDraft>,
    #[serde(default)]
    pub observations: Vec<ObservationDraft>,
    #[serde(default)]
    pub needs_review: bool,
    #[serde(default)]
    pub review_reasons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionDraft {
    pub kind: String,
    pub title_raw: String,
    pub text_raw: String,
    pub source_asset_id: String,
    pub page_index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationDraft {
    pub name_raw: String,
    pub kind: String,
    pub value_raw: String,
    #[serde(default)]
    pub value_number: Option<f64>,
    #[serde(default)]
    pub comparator: Option<String>,
    #[serde(default)]
    pub unit_raw: Option<String>,
    #[serde(default)]
    pub reference_range_raw: Option<String>,
    #[serde(default)]
    pub source_flag: Option<String>,
    #[serde(default)]
    pub body_site: Option<String>,
    #[serde(default)]
    pub laterality: Option<String>,
    pub source_asset_id: String,
    pub page_index: u32,
}

fn invalid(message: &str) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/medical/extract", post(extract))
        .layer(DefaultBodyLimit::max(18 * 1024 * 1024))
}

/// Reject unsupported content *before* invoking the costly external vision model.
/// No arbitrary URLs or file paths are accepted from a client.
fn validate_input(request: &ExtractRequest) -> Result<Vec<String>, ApiError> {
    if request.images.is_empty() || request.images.len() > MAX_FILES {
        return Err(invalid("report images must contain 1 to 8 files"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut total = 0usize;
    let mut urls = Vec::with_capacity(request.images.len());
    for image in &request.images {
        if image.asset_id.is_empty()
            || image.asset_id.len() > 100
            || !image
                .asset_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
            || !seen.insert(image.asset_id.as_str())
        {
            return Err(invalid(
                "assetId must be unique and use only safe characters",
            ));
        }
        if !matches!(
            image.mime_type.as_str(),
            "image/jpeg" | "image/png" | "image/webp"
        ) {
            return Err(invalid("only JPEG, PNG and WebP images are supported"));
        }
        if image.base64.len() > (MAX_IMAGE_BYTES * 4 / 3) + 8 {
            return Err(invalid("image exceeds 5 MiB"));
        }
        let bytes = STANDARD
            .decode(image.base64.as_bytes())
            .map_err(|_| invalid("invalid image base64"))?;
        if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
            return Err(invalid("image exceeds 5 MiB or is empty"));
        }
        let signature_ok = match image.mime_type.as_str() {
            "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/webp" => {
                bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
            }
            _ => false,
        };
        if !signature_ok {
            return Err(invalid("image content does not match MIME type"));
        }
        total += bytes.len();
        if total > MAX_TOTAL_DECODED_BYTES {
            return Err(invalid("batch exceeds 12 MiB"));
        }
        urls.push(format!("data:{};base64,{}", image.mime_type, image.base64));
    }
    Ok(urls)
}

fn parse_draft(
    raw: &str,
    valid_ids: &std::collections::HashSet<&str>,
) -> Result<ExtractReply, ApiError> {
    if raw.len() > MAX_RESPONSE_CHARS {
        return Err(invalid("model response too long"));
    }
    let obj: Value = serde_json::from_str(raw.trim())
        .map_err(|_| invalid("vision model returned invalid JSON"))?;
    let reports = obj
        .get("reports")
        .cloned()
        .ok_or_else(|| invalid("vision model did not return reports"))?;
    let reports: Vec<ReportDraft> = serde_json::from_value(reports)
        .map_err(|_| invalid("vision model returned invalid report fields"))?;
    if reports.is_empty() || reports.len() > 16 {
        return Err(invalid(
            "vision model returned no reports or too many reports",
        ));
    }
    let mut covered_assets = std::collections::HashSet::new();
    for r in &reports {
        covered_assets.extend(r.source_asset_ids.iter().map(String::as_str));
        if !matches!(
            r.report_type.as_str(),
            "laboratory"
                | "ultrasound"
                | "ct"
                | "mri"
                | "xray"
                | "ecg"
                | "pathology"
                | "endoscopy"
                | "other"
        ) {
            return Err(invalid("vision model returned unsupported report type"));
        }
        if r.title.trim().is_empty()
            || r.title.len() > 200
            || r.sections.len() > 100
            || r.observations.len() > 500
            || r.source_asset_ids.is_empty()
            || !r
                .source_asset_ids
                .iter()
                .all(|id| valid_ids.contains(id.as_str()))
        {
            return Err(invalid("vision model returned invalid report metadata"));
        }
        for date in [&r.exam_at, &r.collection_at, &r.issued_at]
            .into_iter()
            .flatten()
        {
            let valid = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok();
            if !valid {
                return Err(invalid("vision model returned invalid date"));
            }
        }
        if r.sections.iter().any(|s| {
            !r.source_asset_ids.contains(&s.source_asset_id)
                || s.text_raw.len() > 30_000
                || s.title_raw.len() > 200
        }) || r.observations.iter().any(|v| {
            !r.source_asset_ids.contains(&v.source_asset_id)
                || v.name_raw.is_empty()
                || v.name_raw.len() > 200
                || v.value_raw.len() > 200
                || v.value_number.is_some_and(|n| !n.is_finite())
        }) {
            return Err(invalid("vision model returned invalid source evidence"));
        }
    }
    if &covered_assets != valid_ids {
        return Err(invalid(
            "vision model omitted at least one uploaded report page",
        ));
    }
    let warnings: Vec<String> =
        serde_json::from_value(obj.get("groupingWarnings").cloned().unwrap_or(json!([])))
            .map_err(|_| invalid("vision model returned invalid grouping warnings"))?;
    if warnings.len() > 50 || warnings.iter().any(|s| s.len() > 500) {
        return Err(invalid("vision model returned too many warnings"));
    }
    Ok(ExtractReply {
        schema_version: 1,
        reports,
        grouping_warnings: warnings,
        requires_confirmation: true,
    })
}

async fn extract(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<ExtractRequest>,
) -> Result<Json<ExtractReply>, ApiError> {
    principal.require_scope("files:write")?;
    let urls = validate_input(&input)?;
    if input
        .instruction
        .as_ref()
        .is_some_and(|hint| hint.chars().count() > 1000)
    {
        return Err(invalid(
            "report description must not exceed 1000 characters",
        ));
    }
    let key = state.config.model_api_key.as_deref().ok_or_else(|| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "vision model is not configured",
            StatusCode::SERVICE_UNAVAILABLE,
        )
    })?;
    let base = state.config.model_base_url.trim_end_matches('/');
    if !base.starts_with("https://") {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "vision model endpoint must use HTTPS",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    // Strictly use an admin-configured endpoint. Never accept a model URL from the user.
    let endpoint = format!("{base}/chat/completions");
    let allowed_ids = input
        .images
        .iter()
        .map(|i| i.asset_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut content = vec![json!({"type":"text","text":format!("合法 assetId: {}",
        input.images.iter().map(|i| i.asset_id.as_str()).collect::<Vec<_>>().join(", "))})];
    if let Some(instruction) = input.instruction.as_ref().filter(|s| !s.trim().is_empty()) {
        content.push(json!({"type":"text","text":format!(
            "用户提供的文件分组说明，仅作为可核验线索，仍以实际报告为准：{instruction}"
        )}));
    }
    let is_deepseek = state.config.model_provider.eq_ignore_ascii_case("deepseek")
        || state.config.model_name == "deepseek-flash";
    let image_detail = if is_deepseek { "original" } else { "high" };
    for (source, url) in input.images.iter().zip(urls) {
        content.push(
            json!({"type":"text","text":format!("以下图片的 assetId 为 {}", source.asset_id)}),
        );
        content.push(json!({"type":"image_url","image_url":{"url":url,"detail":image_detail}}));
    }
    let mut body = json!({
        "model": state.config.model_name,
        "temperature": 0.0,
        "stream": false,
        "max_tokens": 8192,
        "messages": [{"role":"system","content": EXTRACTION_PROMPT}, {"role":"user","content":content}]
    });
    if is_deepseek {
        body["response_format"] = json!({"type":"json_object"});
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|_| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "vision client unavailable",
                StatusCode::BAD_GATEWAY,
            )
        })?;
    let response = client
        .post(&endpoint)
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|_| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "vision provider request failed",
                StatusCode::BAD_GATEWAY,
            )
        })?;
    if !response.status().is_success() {
        // Provider response might echo patient data; never log or expose its body.
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "vision provider rejected request; verify model supports image input",
            StatusCode::BAD_GATEWAY,
        ));
    }
    let reply: Value = response.json().await.map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid vision provider response",
            StatusCode::BAD_GATEWAY,
        )
    })?;
    if reply
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
        == Some("length")
    {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "medical report extraction exceeded model output length; send fewer pages",
            StatusCode::BAD_GATEWAY,
        ));
    }
    let text = reply
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "vision model returned no text",
                StatusCode::BAD_GATEWAY,
            )
        })?;
    let draft = parse_draft(text, &allowed_ids).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "vision extraction needs manual review; invalid model response",
            StatusCode::BAD_GATEWAY,
        )
    })?;
    Ok(Json(draft))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_image_and_duplicate_sources() {
        let example = ImageInput {
            asset_id: "a".into(),
            mime_type: "image/png".into(),
            base64: STANDARD.encode(b"\x89PNG\r\n\x1a\nrest"),
        };
        assert!(validate_input(&ExtractRequest {
            instruction: None,
            images: vec![
                example,
                ImageInput {
                    asset_id: "a".into(),
                    mime_type: "image/png".into(),
                    base64: STANDARD.encode(b"\x89PNG\r\n\x1a\nrest")
                }
            ]
        })
        .is_err());
        assert!(validate_input(&ExtractRequest {
            instruction: None,
            images: vec![ImageInput {
                asset_id: "b".into(),
                mime_type: "text/plain".into(),
                base64: STANDARD.encode(b"test")
            }]
        })
        .is_err());
    }

    #[test]
    fn accepts_supported_image_and_preserves_data() {
        let input = ExtractRequest {
            instruction: None,
            images: vec![ImageInput {
                asset_id: "page_1".into(),
                mime_type: "image/jpeg".into(),
                base64: STANDARD.encode([0xff, 0xd8, 0xff, 0x00]),
            }],
        };
        let urls = validate_input(&input).unwrap();
        assert!(urls[0].starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn prevents_fabricated_source_references_and_invalid_dates() {
        let ids = ["one"].into_iter().collect();
        let raw = r#"{"reports":[{"title":"彩超","reportType":"ultrasound",
            "examAt":null,"sourceAssetIds":["two"],"sections":[],"observations":[]}]}"#;
        assert!(parse_draft(raw, &ids).is_err());
        let raw = raw
            .replace(r#""two""#, r#""one""#)
            .replace(r#""examAt":null"#, r#""examAt":"2026-17-72""#);
        assert!(parse_draft(&raw, &ids).is_err());
    }

    #[test]
    fn rejects_omitted_input_pages_and_wrong_report_source() {
        let ids = ["one", "two"].into_iter().collect();
        let incomplete = r#"{"reports":[{"title":"血检","reportType":"laboratory",
            "sourceAssetIds":["one"],"sections":[],"observations":[]}]}"#;
        assert!(parse_draft(incomplete, &ids).is_err());

        let wrong_source = r#"{"reports":[{"title":"彩超","reportType":"ultrasound",
            "sourceAssetIds":["one","two"],
            "sections":[{"kind":"findings","titleRaw":"所见","textRaw":"甲状腺",
                "sourceAssetId":"three","pageIndex":0}],"observations":[]}]}"#;
        assert!(parse_draft(wrong_source, &ids).is_err());

        let wrong_type = r#"{"reports":[{"title":"彩超","reportType":"hallucinated",
            "sourceAssetIds":["one","two"],"sections":[],"observations":[]}]}"#;
        assert!(parse_draft(wrong_type, &ids).is_err());
    }

    #[test]
    fn extracts_draft_without_inventing_dates() {
        let ids = ["one"].into_iter().collect();
        let raw = r#"{"reports":[{"title":"甲状腺彩超","reportType":"ultrasound",
            "examAt":null,"collectionAt":null,"issuedAt":null,
            "sourceAssetIds":["one"],"sections":[{"kind":"conclusion","titleRaw":"提示",
            "textRaw":"TI-RADS 2类","sourceAssetId":"one","pageIndex":0}],
            "observations":[]}]}"#;
        let draft = parse_draft(raw, &ids).unwrap();
        assert_eq!(draft.reports.len(), 1);
        assert_eq!(draft.reports[0].exam_at, None);
        assert_eq!(draft.reports[0].sections[0].text_raw, "TI-RADS 2类");
        assert!(draft.requires_confirmation);
    }
}
