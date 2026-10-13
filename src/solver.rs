// CSP + Monte Carlo：估計每格是寶藏的機率

pub const SIZE: usize = 8;
pub const CELLS: usize = SIZE * SIZE;

// 寶藏尺寸（允許旋轉）
pub const SHAPES: [(usize, usize); 4] = [(1, 2), (1, 3), (2, 2), (3, 3)];

const SAMPLES: usize = 200;
// 每次 CSP 搜尋的節點上限
const MAX_SEARCH_NODES: usize = 20000;
// 固定種子：同一盤面永遠得到同一個推薦
const SEED: u64 = 42;

pub fn positions(shape: usize) -> Vec<u64> {
    let (h, w) = SHAPES[shape];
    let mut masks = Vec::new();

    for (h, w) in [(h, w), (w, h)] {
        for row in 0..=SIZE - h {
            for col in 0..=SIZE - w {
                let mut mask = 0u64;
                for r in row..row + h {
                    for c in col..col + w {
                        mask |= 1 << (r * SIZE + c);
                    }
                }
                masks.push(mask);
            }
        }
    }

    masks.sort();
    masks.dedup();
    masks
}

// xorshift64*，只用來打亂候選順序
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle(&mut self, v: &mut [u64]) {
        for i in (1..v.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }
}

struct Search {
    // (候選位置, 需要放幾個)
    domains: Vec<(Vec<u64>, usize)>,
    placed: Vec<u64>,
    visited: usize,
}

impl Search {
    fn search(&mut self, index: usize, used: u64) -> bool {
        self.visited += 1;
        if self.visited > MAX_SEARCH_NODES {
            return false;
        }
        if index == self.domains.len() {
            return true;
        }
        let count = self.domains[index].1;
        self.place_same(index, 0, count, used)
    }

    fn place_same(&mut self, index: usize, start: usize, left: usize, used: u64) -> bool {
        // 每放一個寶藏都算一個節點，否則同種寶藏的組合搜尋不受上限限制
        self.visited += 1;
        if self.visited > MAX_SEARCH_NODES {
            return false;
        }
        if left == 0 {
            return self.search(index + 1, used);
        }
        let len = self.domains[index].0.len();
        for i in start..len + 1 - left {
            let mask = self.domains[index].0[i];
            if mask & used != 0 {
                continue;
            }
            self.placed.push(mask);
            if self.place_same(index, i + 1, left - 1, used | mask) {
                return true;
            }
            self.placed.pop();
        }
        false
    }
}

// 隨機找一個合法盤面，回傳剩餘寶藏的位置
pub fn sample_board(
    all: &[Vec<u64>; 4],
    remaining: &[usize; 4],
    blocked: u64,
    rng: &mut Rng,
) -> Option<Vec<u64>> {
    let mut domains = Vec::new();

    for (shape, &count) in remaining.iter().enumerate() {
        if count == 0 {
            continue;
        }
        let mut candidates: Vec<u64> =
            all[shape].iter().copied().filter(|m| m & blocked == 0).collect();
        if candidates.len() < count {
            return None;
        }
        rng.shuffle(&mut candidates);
        let (h, w) = SHAPES[shape];
        domains.push((candidates, count, h * w));
    }

    // MRV：優先處理候選位置較少的寶藏
    domains.sort_by(|a, b| {
        let ra = a.0.len() as f64 / a.1 as f64;
        let rb = b.0.len() as f64 / b.1 as f64;
        ra.total_cmp(&rb).then(b.2.cmp(&a.2))
    });

    let mut s = Search {
        domains: domains.into_iter().map(|(c, n, _)| (c, n)).collect(),
        placed: Vec::new(),
        visited: 0,
    };
    s.search(0, blocked).then_some(s.placed)
}

// 每格的命中機率；找不到任何合法盤面時回傳 None
pub fn probabilities(remaining: &[usize; 4], blocked: u64) -> Option<[f64; CELLS]> {
    let all = std::array::from_fn(positions);
    let mut rng = Rng::new(SEED);
    let mut counts = [0usize; CELLS];
    let mut successful = 0;

    for _ in 0..SAMPLES {
        let Some(board) = sample_board(&all, remaining, blocked, &mut rng) else {
            continue;
        };
        successful += 1;
        for mut mask in board {
            while mask != 0 {
                counts[mask.trailing_zeros() as usize] += 1;
                mask &= mask - 1;
            }
        }
    }

    if successful == 0 {
        return None;
    }
    Some(counts.map(|c| c as f64 / successful as f64))
}

// 未知格中機率最高者，平手取編號最小
pub fn best_cell(probs: &[f64; CELLS], blocked: u64) -> Option<usize> {
    (0..CELLS)
        .filter(|&i| blocked & (1 << i) == 0)
        .fold(None, |best: Option<usize>, i| match best {
            Some(b) if probs[b] >= probs[i] => Some(b),
            _ => Some(i),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_counts() {
        assert_eq!(positions(0).len(), 2 * 7 * 8);
        assert_eq!(positions(1).len(), 2 * 6 * 8);
        assert_eq!(positions(2).len(), 7 * 7);
        assert_eq!(positions(3).len(), 6 * 6);
    }

    #[test]
    fn sample_respects_blocked_and_no_overlap() {
        let mut rng = Rng::new(1);
        let blocked = 0xFF; // 第一列
        let board = sample_board(&std::array::from_fn(positions), &[2, 3, 2, 2], blocked, &mut rng).unwrap();
        assert_eq!(board.len(), 9);
        let mut used = blocked;
        for m in board {
            assert_eq!(m & used, 0);
            used |= m;
        }
    }

    #[test]
    fn forced_position_has_probability_one() {
        // 只留左上 3x3 空著，3x3 寶藏必定在那
        let free = positions(3)[0];
        let probs = probabilities(&[0, 0, 0, 1], !free).unwrap();
        for i in 0..CELLS {
            let expect = if free & (1 << i) != 0 { 1.0 } else { 0.0 };
            assert_eq!(probs[i], expect);
        }
        assert_eq!(best_cell(&probs, !free).map(|i| free & (1 << i) != 0), Some(true));
    }

    #[test]
    fn impossible_returns_none() {
        assert!(probabilities(&[0, 0, 0, 1], !0x7).is_none());
        // 面積夠但放不下（8x8 最多 4 個 3x3），必須在上限內放棄而不是卡死
        assert!(probabilities(&[0, 0, 0, 7], 0).is_none());
        assert!(probabilities(&[0, 0, 17, 0], 0).is_none());
    }
}
