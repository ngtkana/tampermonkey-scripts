#!/usr/bin/env python3
"""
タイトルパターン分析スクリプト
典型的なボカロ曲タイトルのパターンを分析
"""

import re
from collections import defaultdict

# TSV ファイルを読み込み
titles = []
with open('data/nico_api_result.tsv', 'r', encoding='utf-8') as f:
    f.readline()  # ヘッダースキップ
    for line in f:
        parts = line.rstrip('\n').split('\t')
        if len(parts) >= 2:
            titles.append(parts[1])

print(f"Total titles: {len(titles)}")
print()

# パターン分析
patterns = defaultdict(int)

for title in titles:
    # パターンの特徴
    if ' / ' in title:
        patterns['slash_separator'] += 1
    if ' - ' in title:
        patterns['dash_separator'] += 1
    if 'feat.' in title or 'feat ' in title:
        patterns['feat'] += 1
    if '【' in title and '】' in title:
        patterns['japanese_brackets'] += 1
    if '（' in title and '）' in title:
        patterns['japanese_parens'] += 1
    if title.startswith('【'):
        patterns['starts_with_bracket'] += 1
    if '[' in title and ']' in title:
        patterns['square_brackets'] += 1
    if '(' in title and ')' in title:
        patterns['round_parens'] += 1

print("Pattern frequency:")
for pattern, count in sorted(patterns.items(), key=lambda x: -x[1]):
    pct = count * 100 / len(titles)
    print(f"  {pattern:30s}: {count:4d} ({pct:5.1f}%)")

print()
print("Sample titles with specific patterns:")
print()

# スラッシュで分けるパターンの例
print("== Slash separator (曲名 / ボーカル) ==")
count = 0
for title in titles:
    if ' / ' in title and count < 10:
        parts = title.split(' / ')
        print(f"  '{parts[0]}' | '{parts[1]}'")
        count += 1

print()
print("== Japanese brackets (【】) ==")
count = 0
for title in titles:
    if '【' in title and count < 10:
        print(f"  {title}")
        count += 1

print()
print("== feat. pattern ==")
count = 0
for title in titles:
    if 'feat.' in title or 'feat ' in title:
        if count < 10:
            print(f"  {title}")
            count += 1

print()
print("== Dash separator (曲名 - subtitle) ==")
count = 0
for title in titles:
    if ' - ' in title and '【' not in title and count < 10:
        parts = title.split(' - ')
        print(f"  '{parts[0]}' | '{parts[1]}'")
        count += 1
