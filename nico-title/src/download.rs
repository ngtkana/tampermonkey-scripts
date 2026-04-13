use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    data: Vec<Video>,
    meta: ApiMeta,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMeta {
    status: i32,
    #[serde(default, rename = "totalCount")]
    total_count: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Video {
    #[serde(rename = "contentId")]
    content_id: String,
    title: String,
}

pub fn download() {
    let client = reqwest::blocking::Client::new();
    let base_url = "https://snapshot.search.nicovideo.jp/api/v2/snapshot/video/contents/search";

    let mut all_videos = Vec::new();
    let limit = 100;
    let max_offset = 1600; // 最大 1600 件まで（100件 × 16ページ）

    // 初回リクエストで totalCount を取得
    for offset in (0..max_offset).step_by(limit) {
        println!("Fetching offset: {}", offset);

        let response = client
            .get(base_url)
            .query(&[
                ("q", "vocaloidオリジナル曲"),
                ("targets", "tagsExact"),
                ("fields", "contentId,title"),
                ("_sort", "-startTime"),
                ("_limit", &limit.to_string()),
                ("_offset", &offset.to_string()),
                ("_context", "my_app"),
            ])
            .send();

        match response {
            Ok(resp) => {
                let text = match resp.text() {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Failed to get response text: {}", e);
                        break;
                    }
                };

                // Debug: print first part of response
                // println!("Response text (first 500 chars): {}", &text[..text.len().min(500)]);

                match serde_json::from_str::<ApiResponse>(&text) {
                    Ok(api_resp) => {
                        if offset == 0
                            && let Some(count) = api_resp.meta.total_count
                        {
                            println!("Total count: {}", count);
                        }

                        let batch_size = api_resp.data.len();
                        all_videos.extend(api_resp.data);

                        // データが空またはすべて取得したら終了
                        if batch_size < limit || all_videos.len() >= max_offset {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse JSON: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Request failed: {}", e);
                break;
            }
        }
    }

    // data/ ディレクトリを作成
    if let Err(e) = create_dir_all("data") {
        eprintln!("Failed to create data directory: {}", e);
        return;
    }

    // TSV ファイルに書き出し
    let video_count = all_videos.len();
    match File::create("data/nico_api_result.tsv") {
        Ok(mut file) => {
            // ヘッダー行を書き出し
            if let Err(e) = writeln!(file, "contentId\ttitle") {
                eprintln!("Failed to write header: {}", e);
                return;
            }

            // データを TSV 形式で書き出し
            for video in all_videos {
                if let Err(e) = writeln!(file, "{}\t{}", video.content_id, video.title) {
                    eprintln!("Failed to write line: {}", e);
                    return;
                }
            }

            println!(
                "Successfully saved {} videos to data/nico_api_result.tsv",
                video_count
            );
        }
        Err(e) => {
            eprintln!("Failed to create file: {}", e);
        }
    }
}
