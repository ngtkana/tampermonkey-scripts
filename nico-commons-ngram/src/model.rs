use std::collections::HashMap;

/// パフォーマンス指標
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Metrics {
    pub loss: f64,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub tp: usize,
    pub tn: usize,
    pub fp: usize,
    pub fn_: usize,
}

impl Metrics {
    /// 予測確率とラベルのペアから Metrics を計算
    pub fn compute(preds: &[(f64, f64)]) -> Self {
        let mut loss = 0.0;
        let mut tp = 0usize;
        let mut tn = 0usize;
        let mut fp = 0usize;
        let mut fn_ = 0usize;

        for (prob, label) in preds {
            // loss
            let prob_clipped = prob.clamp(1e-10, 1.0 - 1e-10);
            loss += -label * prob_clipped.ln() - (1.0 - label) * (1.0 - prob_clipped).ln();

            // confusion matrix
            let pred = if prob >= &0.5 { 1.0 } else { 0.0 };
            match (pred > 0.5, label > &0.5) {
                (true, true) => tp += 1,
                (true, false) => fp += 1,
                (false, true) => fn_ += 1,
                (false, false) => tn += 1,
            }
        }

        let n = preds.len() as f64;
        let accuracy = (tp as f64 + tn as f64) / n;

        let precision = if tp + fp == 0 {
            0.0
        } else {
            tp as f64 / (tp + fp) as f64
        };

        let recall = if tp + fn_ == 0 {
            0.0
        } else {
            tp as f64 / (tp + fn_) as f64
        };

        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };

        Self {
            loss: loss / n,
            accuracy,
            precision,
            recall,
            f1,
            tp,
            tn,
            fp,
            fn_,
        }
    }
}

/// ハイパーパラメータ（後で調整しやすいよう定数化）
pub struct HyperParams {
    pub n_min: usize,
    pub n_max: usize,
    pub learning_rate: f64,
    pub lambda: f64, // 正則化係数
    pub epochs: usize,
    pub early_stop_patience: usize, // 改善なしの許容エポック数
    pub use_tfidf: bool,            // TF-IDF を使うか
}

impl Default for HyperParams {
    fn default() -> Self {
        Self {
            n_min: 3,
            n_max: 5,
            learning_rate: 0.1,
            lambda: 1e-4,
            epochs: 50,
            early_stop_patience: 10,
            use_tfidf: false,
        }
    }
}

/// 学習済みモデル（推論に必要な情報をすべて含む）
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Model {
    pub vocab: HashMap<String, usize>,
    pub weights: Vec<f64>,
    pub bias: f64,
    pub n_min: usize,
    pub n_max: usize,
    pub idf: Vec<f64>, // 空なら binary, 非空なら TF-IDF
}

impl Model {
    pub fn new(vocab: HashMap<String, usize>, n_min: usize, n_max: usize, idf: Vec<f64>) -> Self {
        let n = vocab.len();
        Self {
            weights: vec![0.0; n],
            bias: 0.0,
            vocab,
            n_min,
            n_max,
            idf,
        }
    }

    /// 重み付き特徴量 (index, weight) で予測確率を計算
    /// weight=1.0 でバイナリ、 weight=tf*idf で TF-IDF
    pub fn predict_prob(&self, features: &[(usize, f64)]) -> f64 {
        let score = features
            .iter()
            .fold(self.bias, |acc, &(i, w)| acc + self.weights[i] * w);
        sigmoid(score)
    }

    /// 閾値を指定して分類（確率ベース）
    #[allow(dead_code)]
    pub fn classify_with_threshold(&self, features: &[(usize, f64)], threshold: f64) -> bool {
        self.predict_prob(features) >= threshold
    }

    /// SGD で 1 サンプル分のパラメータを更新
    pub fn update(&mut self, features: &[(usize, f64)], label: f64, params: &HyperParams) {
        let prob = self.predict_prob(features);
        let err = prob - label;
        self.bias -= params.learning_rate * err;
        for &(i, w) in features {
            // L2 weight decay: w_i -= lr * (gradient + lambda * w_i)
            self.weights[i] -= params.learning_rate * (w * err + params.lambda * self.weights[i]);
        }
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigmoid() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-10);
        assert!(sigmoid(100.0) > 0.99);
        assert!(sigmoid(-100.0) < 0.01);
    }

    #[test]
    fn test_model_creation() {
        let vocab = vec![("a".to_string(), 0), ("b".to_string(), 1)]
            .into_iter()
            .collect();
        let m = Model::new(vocab, 3, 5, vec![]);
        assert_eq!(m.weights.len(), 2);
        assert_eq!(m.bias, 0.0);
        assert_eq!(m.n_min, 3);
        assert_eq!(m.n_max, 5);
    }

    #[test]
    fn test_metrics() {
        let preds = vec![(0.9, 1.0), (0.1, 0.0), (0.8, 1.0), (0.2, 0.0)];
        let m = Metrics::compute(&preds);
        assert_eq!(m.tp, 2);
        assert_eq!(m.tn, 2);
        assert_eq!(m.fp, 0);
        assert_eq!(m.fn_, 0);
        assert!((m.accuracy - 1.0).abs() < 1e-10);
        assert!((m.f1 - 1.0).abs() < 1e-10);
    }
}
