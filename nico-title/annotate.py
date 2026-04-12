#!/usr/bin/env python3
"""
Claude API を使ってニコニコボカロ曲タイトルから曲名を抽出
数百件のアノテーション例を生成して教師データを作成
"""

import anthropic
import json
import sys
import random

def load_titles(tsv_file: str, sample_size: int = 500) -> list[str]:
    """TSV ファイルからタイトルをサンプリングして読み込む"""
    titles = []
    with open(tsv_file, 'r', encoding='utf-8') as f:
        f.readline()  # ヘッダースキップ
        for line in f:
            parts = line.rstrip('\n').split('\t')
            if len(parts) >= 2:
                titles.append(parts[1])

    # ランダムサンプリング
    return random.sample(titles, min(sample_size, len(titles)))

def annotate_titles(titles: list[str], batch_size: int = 10) -> list[dict]:
    """Claude API を使ってタイトルをアノテーション"""
    client = anthropic.Anthropic()
    results = []

    system_prompt = """あなたは日本語の自然言語処理専門家です。
ニコニコ動画のボカロオリジナル曲のタイトルから、曲の実際の名前（曲名）を抽出してください。

タイトルはしばしば以下のような形式を含みます：
- 【MV】や【オリジナル曲】などのメタデータを含む括弧
- 曲名 / ボーカロイド名 という形式のスラッシュ分割
- (feat. ボーカロイド) という追記
- [MV] や [Official] などの角括弧

あなたの仕事は、これらのノイズから曲の実際の名前を抽出することです。

JSON 形式で、以下のフィールドを含む結果を返してください：
{
  "title": "元のタイトル",
  "extracted_title": "抽出された曲名",
  "confidence": 0.8,
  "reasoning": "抽出理由の簡潔な説明"
}"""

    print(f"Total titles to annotate: {len(titles)}")
    print(f"Processing in batches of {batch_size}...")

    for i in range(0, len(titles), batch_size):
        batch = titles[i:i+batch_size]
        batch_num = i // batch_size + 1
        total_batches = (len(titles) + batch_size - 1) // batch_size

        # バッチをプロンプトに組み込む
        titles_text = "\n".join(f"{j+1}. {title}" for j, title in enumerate(batch))
        user_prompt = f"""以下の {len(batch)} 個のタイトルをアノテーションしてください。
各タイトルに対して、抽出された曲名を JSON オブジェクトで返してください。
JSON オブジェクトの配列で回答してください。

タイトル一覧：
{titles_text}"""

        print(f"  Batch {batch_num}/{total_batches}...", end='', flush=True)

        try:
            message = client.messages.create(
                model="claude-opus-4-6",
                max_tokens=2048,
                system=system_prompt,
                messages=[
                    {"role": "user", "content": user_prompt}
                ]
            )

            response_text = message.content[0].text

            # JSON 配列を抽出
            start_idx = response_text.find('[')
            end_idx = response_text.rfind(']') + 1
            if start_idx != -1 and end_idx > start_idx:
                json_str = response_text[start_idx:end_idx]
                batch_results = json.loads(json_str)
                results.extend(batch_results)
                print(f" ✓ ({len(batch_results)} annotations)")
            else:
                print(f" ✗ (JSON parse error)")

        except Exception as e:
            print(f" ✗ (API error: {str(e)})")

    return results

def save_annotations(results: list[dict], output_file: str):
    """アノテーション結果を JSONL 形式で保存"""
    with open(output_file, 'w', encoding='utf-8') as f:
        for result in results:
            f.write(json.dumps(result, ensure_ascii=False) + '\n')
    print(f"\nAnnotations saved to {output_file}")
    print(f"Total annotations: {len(results)}")

def main():
    # タイトルを読み込み
    sample_size = 500 if len(sys.argv) < 2 else int(sys.argv[1])
    titles = load_titles("data/nico_api_result.tsv", sample_size=sample_size)

    print(f"Loaded {len(titles)} titles from data/nico_api_result.tsv")
    print()

    # Claude API でアノテーション
    results = annotate_titles(titles, batch_size=10)

    # 結果を保存
    save_annotations(results, "data/nico_api_annotations.jsonl")

    # 統計情報を表示
    if results:
        avg_confidence = sum(r.get('confidence', 0) for r in results) / len(results)
        print(f"Average confidence: {avg_confidence:.2f}")

if __name__ == "__main__":
    main()
