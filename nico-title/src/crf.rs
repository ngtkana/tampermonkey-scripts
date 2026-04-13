use std::collections::HashMap;

/// BIO ラベルの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Label {
    B = 0,
    I = 1,
    O = 2,
}

impl Label {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "B" => Some(Label::B),
            "I" => Some(Label::I),
            "O" => Some(Label::O),
            _ => None,
        }
    }

    pub fn all() -> [Label; 3] {
        [Label::B, Label::I, Label::O]
    }
}

/// log-space での計算用ユーティリティ（log-sum-exp トリック）
fn log_sum_exp(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    let max_val = a.max(b);
    let min_val = a.min(b);
    // log(exp(a) + exp(b)) = max + log(1 + exp(min - max))
    max_val + (1.0 + (min_val - max_val).exp()).ln()
}

/// CRF モデル
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrfModel {
    /// 特徴量重み: [feature_id][label] → weight
    pub feature_weights: Vec<[f64; 3]>,
    /// 遷移スコア: [from_label][to_label] → score
    pub transition: [[f64; 3]; 3],
    /// 特徴量マップ（特徴量文字列 → ID）
    pub feature_map: HashMap<String, usize>,
    /// ハイパーパラメータ
    pub learning_rate: f64,
    pub lambda: f64, // L2 正則化係数
}

impl CrfModel {
    /// 新しい CRF モデルを作成
    pub fn new(feature_map: HashMap<String, usize>, learning_rate: f64, lambda: f64) -> Self {
        let num_features = feature_map.len();
        let feature_weights = vec![[0.0; 3]; num_features];
        let transition = [[0.0; 3]; 3];

        Self {
            feature_weights,
            transition,
            feature_map,
            learning_rate,
            lambda,
        }
    }

    /// 与えられた特徴量と前ラベルから、ラベル y へのスコアを計算
    fn score(&self, features: &[usize], prev_label: Option<Label>, curr_label: Label) -> f64 {
        let mut score = 0.0;

        // 特徴量スコア
        for &feat_id in features {
            if feat_id < self.feature_weights.len() {
                score += self.feature_weights[feat_id][curr_label as usize];
            }
        }

        // 遷移スコア
        if let Some(prev) = prev_label {
            score += self.transition[prev as usize][curr_label as usize];
        }

        score
    }

    /// Forward パス（log-space）：α[t][y] = log Σ_path_{1..t-1} exp(score)
    fn forward_pass(&self, sequence: &[Vec<usize>]) -> Vec<[f64; 3]> {
        let n = sequence.len();
        let mut alpha = vec![[f64::NEG_INFINITY; 3]; n + 1];

        // 初期化：時刻 0 は全ラベルに 1 の確率
        alpha[0] = [0.0; 3];

        for t in 0..n {
            for curr_label in Label::all() {
                let curr_idx = curr_label as usize;

                for prev_label in Label::all() {
                    let prev_idx = prev_label as usize;
                    let score =
                        alpha[t][prev_idx] + self.score(&sequence[t], Some(prev_label), curr_label);
                    alpha[t + 1][curr_idx] = log_sum_exp(alpha[t + 1][curr_idx], score);
                }
            }
        }

        alpha
    }

    /// Backward パス（log-space）：β[t][y] = log Σ_path_{t+1..n} exp(score)
    fn backward_pass(&self, sequence: &[Vec<usize>]) -> Vec<[f64; 3]> {
        let n = sequence.len();
        let mut beta = vec![[f64::NEG_INFINITY; 3]; n + 1];

        // 終端：全ラベルに 1 の確率
        beta[n] = [0.0; 3];

        for t in (0..n).rev() {
            for prev_label in Label::all() {
                let prev_idx = prev_label as usize;

                for curr_label in Label::all() {
                    let curr_idx = curr_label as usize;
                    let score = self.score(&sequence[t], Some(prev_label), curr_label)
                        + beta[t + 1][curr_idx];
                    beta[t][prev_idx] = log_sum_exp(beta[t][prev_idx], score);
                }
            }
        }

        beta
    }

    /// 対数分配関数 log Z = log Σ_all_paths exp(score)
    fn log_partition(&self, sequence: &[Vec<usize>]) -> f64 {
        let alpha = self.forward_pass(sequence);
        let n = sequence.len();

        // 終端での log-sum-exp
        let mut z = f64::NEG_INFINITY;
        for label in Label::all() {
            z = log_sum_exp(z, alpha[n][label as usize]);
        }
        z
    }

    /// Viterbi アルゴリズムで最尤ラベル系列を復号
    pub fn viterbi(&self, sequence: &[Vec<usize>]) -> Vec<Label> {
        let n = sequence.len();
        let mut viterbi = vec![[f64::NEG_INFINITY; 3]; n + 1];
        let mut backpointer = vec![[0usize; 3]; n + 1];

        // 初期化
        viterbi[0] = [0.0; 3];

        // Forward パス
        for t in 0..n {
            for curr_label in Label::all() {
                let curr_idx = curr_label as usize;
                let mut best_score = f64::NEG_INFINITY;
                let mut best_prev = 0usize;

                for prev_label in Label::all() {
                    let prev_idx = prev_label as usize;
                    let score = viterbi[t][prev_idx]
                        + self.score(&sequence[t], Some(prev_label), curr_label);

                    if score > best_score {
                        best_score = score;
                        best_prev = prev_idx;
                    }
                }

                viterbi[t + 1][curr_idx] = best_score;
                backpointer[t + 1][curr_idx] = best_prev;
            }
        }

        // Backtrack
        let mut path = vec![Label::O; n];
        let mut curr = 0usize;
        for label in Label::all() {
            if viterbi[n][label as usize] > viterbi[n][curr] {
                curr = label as usize;
            }
        }

        for t in (0..n).rev() {
            path[t] = match curr {
                0 => Label::B,
                1 => Label::I,
                _ => Label::O,
            };
            curr = backpointer[t + 1][curr];
        }

        path
    }

    /// ラベルシーケンスのスコアを計算
    pub fn sequence_score(&self, sequence: &[Vec<usize>], labels: &[Label]) -> f64 {
        let mut score = 0.0;
        let mut prev_label: Option<Label> = None;

        for (t, &label) in labels.iter().enumerate() {
            score += self.score(&sequence[t], prev_label, label);
            prev_label = Some(label);
        }

        score
    }

    /// 1つのサンプルに対する負の対数尤度を計算
    pub fn nll(&self, sequence: &[Vec<usize>], gold_labels: &[Label]) -> f64 {
        let score = self.sequence_score(sequence, gold_labels);
        let log_z = self.log_partition(sequence);
        log_z - score
    }

    /// 1つのサンプルに対する勾配を計算（期待 - 観測）
    pub fn compute_gradient(
        &self,
        sequence: &[Vec<usize>],
        gold_labels: &[Label],
    ) -> (Vec<[f64; 3]>, [[f64; 3]; 3]) {
        let n = sequence.len();
        let alpha = self.forward_pass(sequence);
        let beta = self.backward_pass(sequence);
        let log_z = self.log_partition(sequence);

        let mut grad_features = vec![[0.0; 3]; self.feature_weights.len()];
        let mut grad_transition = [[0.0; 3]; 3];

        // 観測特徴量（正解パス）
        let mut prev_label: Option<Label> = None;
        for t in 0..n {
            let curr_label = gold_labels[t];

            // 特徴量（負例、減算）
            for &feat_id in &sequence[t] {
                if feat_id < grad_features.len() {
                    grad_features[feat_id][curr_label as usize] -= 1.0;
                }
            }

            // 遷移（負例）
            if let Some(prev) = prev_label {
                grad_transition[prev as usize][curr_label as usize] -= 1.0;
            }

            prev_label = Some(curr_label);
        }

        // 期待特徴量（マージナル確率ベース）
        for t in 0..n {
            for curr_label in Label::all() {
                let curr_idx = curr_label as usize;

                // 時刻 t でのマージナル確率
                let marg = alpha[t + 1][curr_idx] + beta[t + 1][curr_idx] - log_z;
                let marg_prob = marg.exp();

                // 特徴量の期待値（正例、加算）
                for &feat_id in &sequence[t] {
                    if feat_id < grad_features.len() {
                        grad_features[feat_id][curr_idx] += marg_prob;
                    }
                }
            }
        }

        // 遷移の期待値
        for t in 0..n - 1 {
            for prev_label in Label::all() {
                for curr_label in Label::all() {
                    let prev_idx = prev_label as usize;
                    let curr_idx = curr_label as usize;

                    let score = self.score(&sequence[t], Some(prev_label), curr_label);
                    let exp_score = alpha[t + 1][prev_idx] + score + beta[t + 2][curr_idx] - log_z;
                    let exp_prob = exp_score.exp();

                    grad_transition[prev_idx][curr_idx] += exp_prob;
                }
            }
        }

        (grad_features, grad_transition)
    }

    /// バッチの勾配を計算・更新（SGD with Gradient Clipping）
    pub fn train_step(&mut self, batch: &[(Vec<Vec<usize>>, Vec<Label>)]) {
        let batch_size = batch.len() as f64;
        let mut total_grad_features = vec![[0.0; 3]; self.feature_weights.len()];
        let mut total_grad_transition = [[0.0; 3]; 3];

        // バッチ全体の勾配を集積
        for (sequence, labels) in batch {
            let (grad_f, grad_t) = self.compute_gradient(sequence, labels);

            for (feat_id, grads) in grad_f.iter().enumerate() {
                for label in 0..3 {
                    total_grad_features[feat_id][label] += grads[label];
                }
            }

            for from_label in 0..3 {
                for to_label in 0..3 {
                    total_grad_transition[from_label][to_label] += grad_t[from_label][to_label];
                }
            }
        }

        // 勾配を平均化
        for feat_id in 0..self.feature_weights.len() {
            for label in 0..3 {
                total_grad_features[feat_id][label] /= batch_size;
            }
        }

        for from_label in 0..3 {
            for to_label in 0..3 {
                total_grad_transition[from_label][to_label] /= batch_size;
            }
        }

        // 勾配 clipping（最大ノルム = 5.0）
        let max_grad_norm = 5.0;
        let mut norm_sq = 0.0;
        for feat_id in 0..self.feature_weights.len() {
            for label in 0..3 {
                norm_sq +=
                    total_grad_features[feat_id][label] * total_grad_features[feat_id][label];
            }
        }
        for from_label in 0..3 {
            for to_label in 0..3 {
                norm_sq += total_grad_transition[from_label][to_label]
                    * total_grad_transition[from_label][to_label];
            }
        }

        let grad_norm = norm_sq.sqrt();
        let clip_factor = if grad_norm > max_grad_norm {
            max_grad_norm / grad_norm
        } else {
            1.0
        };

        // パラメータを更新
        for (feat_id, weights) in self.feature_weights.iter_mut().enumerate() {
            for label in 0..3 {
                let grad = total_grad_features[feat_id][label] * clip_factor;
                weights[label] -= self.learning_rate * (grad + self.lambda * weights[label]);
            }
        }

        for from_label in 0..3 {
            for to_label in 0..3 {
                let grad = total_grad_transition[from_label][to_label] * clip_factor;
                self.transition[from_label][to_label] -= self.learning_rate
                    * (grad + self.lambda * self.transition[from_label][to_label]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_conversion() {
        assert_eq!(Label::from_str("B"), Some(Label::B));
        assert_eq!(Label::from_str("I"), Some(Label::I));
        assert_eq!(Label::from_str("O"), Some(Label::O));
        assert_eq!(Label::from_str("X"), None);
    }

    #[test]
    fn test_viterbi_simple() {
        let mut model = CrfModel::new(HashMap::new(), 0.1, 0.001);
        let sequence = vec![vec![], vec![]]; // 2 時刻、特徴量なし
        let path = model.viterbi(&sequence);
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_sequence_score() {
        let model = CrfModel::new(HashMap::new(), 0.1, 0.001);
        let sequence = vec![vec![], vec![]];
        let labels = vec![Label::B, Label::I];
        let score = model.sequence_score(&sequence, &labels);
        assert!(score.is_finite());
    }
}
