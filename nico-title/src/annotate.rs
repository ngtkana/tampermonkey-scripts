use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Serialize, Deserialize)]
struct AnnotationRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<RequestMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnnotationResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnnotationResult {
    pub title: String,
    pub extracted_title: String,
}

pub fn annotate_titles(count: usize) {
    // API キーを環境変数から取得
    let api_key = match env::var("ANTHROPIC_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Error: ANTHROPIC_API_KEY environment variable not set");
            eprintln!("Set it with: export ANTHROPIC_API_KEY=sk-...");
            return;
        }
    };

    // TSV ファイルから タイトルを読み込む
    let titles = match load_titles("data/nico_api_result.tsv", count) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load titles: {}", e);
            return;
        }
    };

    let batch_size = 20;
    println!("Loaded {} titles to annotate", titles.len());
    println!("Processing in batches of {batch_size}...\n");

    let client = reqwest::blocking::Client::new();
    let mut output_file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open("data/nico_api_annotations.jsonl")
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open output file: {}", e);
            return;
        }
    };

    let mut total_processed = 0;

    for batch_num in 0..=(titles.len() / batch_size) {
        let start = batch_num * batch_size;
        let end = (start + batch_size).min(titles.len());

        if start >= titles.len() {
            break;
        }

        let batch = &titles[start..end];
        print!(
            "Batch {}/{}: ",
            batch_num + 1,
            titles.len().div_ceil(batch_size)
        );

        // プロンプトを組み立て
        let titles_text = batch
            .iter()
            .enumerate()
            .map(|(i, title)| format!("{}. {}", i + 1, title))
            .collect::<Vec<_>>()
            .join("\n");

        let user_prompt = format!(
            "以下の {} 個のニコニコ動画ボカロ曲タイトルをアノテーションしてください。\n\
             各タイトルに対して、抽出された曲名を以下の JSON 配列形式で返してください。\n\n\
             タイトル一覧:\n{}\n\n\
             JSON 配列形式で応答してください: [{{\"title\": \"...\", \"extracted_title\": \"...\"}}]",
            batch.len(),
            titles_text
        );

        let request = AnnotationRequest {
            model: "claude-opus-4-6".to_string(),
            max_tokens: 1024,
            system: "ニコニコ動画のボカロオリジナル曲タイトルから、曲の実際の名前（曲名）を抽出してください。\n\n\
                     ## 除去すべき要素\n\n\
                     **ボーカロイド・アーティスト名**（スラッシュやハイフン、feat. の後に付く）\n\
                     初音ミク、鏡音リン、鏡音レン、巡音ルカ、MEIKO、KAITO、重音テト、flower、IA、GUMI、可不、歌愛ユキ、知声、音街ウナ など\n\n\
                     **区切り文字のパターン**\n\
                     - 曲名 / 名前、曲名/名前、曲名／名前\n\
                     - 曲名 - 名前、曲名_名前\n\
                     - 曲名 feat. 名前、曲名(feat.名前)、曲名　名前（全角スペース）\n\n\
                     **メタデータ（括弧を含めて除去）**\n\
                     - 【MV】【オリジナル曲】【ボカロオリジナル曲】など\n\
                     - [MV][Official]などの角括弧\n\
                     - \"Official Music Video\"、\"short ver\"、\"Full ver\"、\"Remaster\"（スラッシュ以降の場合）\n\n\
                     **残すべき要素**\n\
                     - 曲の実際の名前（日本語・英語・記号を含む）\n\
                     - \"〜1st anniversary Ver.〜\"のようにタイトルに組み込まれたバージョン表記\n\
                     - サブタイトル（ダッシュで囲まれた部分が曲名の一部の場合）例：Thalatta Lyra -モナリザの憂鬱-\n\
                     - 括弧内でもスラッシュより前の内容は残す"
                .to_string(),
            messages: vec![RequestMessage {
                role: "user".to_string(),
                content: user_prompt,
            }],
        };

        // API リクエスト
        match send_request(&client, &api_key, &request) {
            Ok(results) => {
                for result in results {
                    if let Err(e) = writeln!(
                        output_file,
                        "{}",
                        serde_json::to_string(&result).unwrap_or_default()
                    ) {
                        eprintln!("Failed to write result: {}", e);
                    }
                }
                total_processed += batch.len();
                println!("✓ ({} results)", batch.len());
            }
            Err(e) => {
                println!("✗ ({})", e);
            }
        }
    }

    println!(
        "\nSuccessfully processed {} annotations to data/nico_api_annotations.jsonl",
        total_processed
    );
}

fn send_request(
    client: &reqwest::blocking::Client,
    api_key: &str,
    request: &AnnotationRequest,
) -> Result<Vec<AnnotationResult>, String> {
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| format!("API request failed: {}", e))?;

    let status = response.status();
    let text = response
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "HTTP {}: {}",
            status,
            text.chars().take(200).collect::<String>()
        ));
    }

    let resp: AnnotationResponse =
        serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))?;

    // レスポンスの text フィールドから JSON 配列を抽出
    if let Some(content_block) = resp.content.first() {
        let response_text = &content_block.text;

        // JSON 配列を抽出
        let start_idx = response_text.find('[');
        let end_idx = response_text.rfind(']');

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            let json_str = &response_text[start..=end];
            let results: Vec<AnnotationResult> = serde_json::from_str(json_str)
                .map_err(|e| format!("Failed to parse JSON array: {}", e))?;
            Ok(results)
        } else {
            Err("No JSON array found in response".to_string())
        }
    } else {
        Err("No content blocks in response".to_string())
    }
}

fn load_titles(tsv_file: &str, sample_size: usize) -> Result<Vec<String>, String> {
    let file = File::open(tsv_file).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut titles = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue; // ヘッダースキップ
        }

        match line {
            Ok(l) => {
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.len() >= 2 {
                    titles.push(parts[1].to_string());
                }
            }
            Err(e) => {
                return Err(format!("Error reading line: {}", e));
            }
        }

        if titles.len() >= sample_size {
            break;
        }
    }

    Ok(titles)
}
