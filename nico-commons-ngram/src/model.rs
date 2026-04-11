use std::collections::HashMap;

/// ハイパーパラメータ（後で調整しやすいよう定数化）
pub struct HyperParams {
    pub n_min: usize,
    pub n_max: usize,
    pub learning_rate: f64,
    pub lambda: f64,      // L2 正則化係数
    pub epochs: usize,
    pub early_stop_patience: usize,  // 改善なしの許容エポック数
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
}

impl Model {
    pub fn new(vocab: HashMap<String, usize>, n_min: usize, n_max: usize) -> Self {
        let n = vocab.len();
        Self {
            weights: vec![0.0; n],
            bias: 0.0,
            vocab,
            n_min,
            n_max,
        }
    }

    /// 特徴量インデックス列で予測確率を計算
    pub fn predict_prob(&self, features: &[usize]) -> f64 {
        let score = features.iter().fold(self.bias, |acc, &i| acc + self.weights[i]);
        sigmoid(score)
    }

    /// SGD で 1 サンプル分のパラメータを更新
    pub fn update(&mut self, features: &[usize], label: f64, params: &HyperParams) {
        let prob = self.predict_prob(features);
        let err = prob - label;
        self.bias -= params.learning_rate * err;
        for &i in features {
            // L2 weight decay: w_i -= lr * (gradient + lambda * w_i)
            self.weights[i] -= params.learning_rate * (err + params.lambda * self.weights[i]);
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
        let m = Model::new(vocab, 3, 5);
        assert_eq!(m.weights.len(), 2);
        assert_eq!(m.bias, 0.0);
        assert_eq!(m.n_min, 3);
        assert_eq!(m.n_max, 5);
    }
}
