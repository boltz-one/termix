// Run-Length Encoding - port til/rle.h
// Compact representation for sequences of identical values

use std::fmt;

/// Run-Length Encoded vector
/// Stores sequences of identical values compactly as (value, count) pairs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rle<T> {
    runs: Vec<Run<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Run<T> {
    value: T,
    count: usize,
}

impl<T: Clone + Eq> Rle<T> {
    /// Create empty RLE
    pub fn new() -> Self {
        Self { runs: Vec::new() }
    }

    /// Create RLE from iterator
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut rle = Self::new();
        for value in iter {
            rle.push(value);
        }
        rle
    }

    /// Push value, merging with last run if same
    pub fn push(&mut self, value: T) {
        if let Some(last) = self.runs.last_mut() {
            if last.value == value {
                last.count += 1;
                return;
            }
        }
        self.runs.push(Run { value, count: 1 });
    }

    /// Push N copies of value
    pub fn push_n(&mut self, value: T, count: usize) {
        if count == 0 {
            return;
        }
        if let Some(last) = self.runs.last_mut() {
            if last.value == value {
                last.count += count;
                return;
            }
        }
        self.runs.push(Run { value, count });
    }

    /// Get value at index
    pub fn get(&self, index: usize) -> Option<&T> {
        let mut offset = 0;
        for run in &self.runs {
            if index < offset + run.count {
                return Some(&run.value);
            }
            offset += run.count;
        }
        None
    }

    /// Total number of values (expanded)
    pub fn len(&self) -> usize {
        self.runs.iter().map(|r| r.count).sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// Number of runs (compressed)
    pub fn run_count(&self) -> usize {
        self.runs.len()
    }

    /// Clear all runs
    pub fn clear(&mut self) {
        self.runs.clear();
    }

    /// Iterate over (value, count) runs
    pub fn runs(&self) -> impl Iterator<Item = (&T, usize)> {
        self.runs.iter().map(|r| (&r.value, r.count))
    }

    /// Iterate over expanded values
    pub fn iter(&self) -> RleIter<T> {
        RleIter {
            runs: &self.runs,
            run_index: 0,
            position: 0,
        }
    }

    /// Replace value at index
    pub fn set(&mut self, index: usize, value: T) {
        let mut offset = 0;
        for i in 0..self.runs.len() {
            let run = &self.runs[i];
            if index < offset + run.count {
                let pos_in_run = index - offset;

                // Value matches - no split needed
                if self.runs[i].value == value {
                    return;
                }

                // Need to split run
                let old_value = self.runs[i].value.clone();
                let old_count = self.runs[i].count;

                let mut new_runs = Vec::new();

                // Before position
                if pos_in_run > 0 {
                    new_runs.push(Run {
                        value: old_value.clone(),
                        count: pos_in_run,
                    });
                }

                // New value at position
                new_runs.push(Run { value, count: 1 });

                // After position
                if pos_in_run + 1 < old_count {
                    new_runs.push(Run {
                        value: old_value,
                        count: old_count - pos_in_run - 1,
                    });
                }

                // Replace run with split runs
                self.runs.splice(i..=i, new_runs);
                return;
            }
            offset += run.count;
        }
    }

    /// Resize, filling with value
    pub fn resize(&mut self, new_len: usize, value: T) {
        let current_len = self.len();
        match new_len.cmp(&current_len) {
            std::cmp::Ordering::Greater => {
                self.push_n(value, new_len - current_len);
            }
            std::cmp::Ordering::Less => {
                self.truncate(new_len);
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    /// Truncate to length
    pub fn truncate(&mut self, len: usize) {
        let mut offset = 0;
        for i in 0..self.runs.len() {
            let run_end = offset + self.runs[i].count;
            if len <= offset {
                // Truncate all remaining runs
                self.runs.truncate(i);
                return;
            } else if len < run_end {
                // Truncate within this run
                self.runs[i].count = len - offset;
                self.runs.truncate(i + 1);
                return;
            }
            offset = run_end;
        }
    }
}

impl<T: Clone + Eq> Default for Rle<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Eq> FromIterator<T> for Rle<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_iter(iter)
    }
}

pub struct RleIter<'a, T> {
    runs: &'a [Run<T>],
    run_index: usize,
    position: usize,
}

impl<'a, T: Clone> Iterator for RleIter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.run_index >= self.runs.len() {
            return None;
        }

        let run = &self.runs[self.run_index];
        let value = run.value.clone();

        self.position += 1;
        if self.position >= run.count {
            self.run_index += 1;
            self.position = 0;
        }

        Some(value)
    }
}

impl<T: Clone + Eq + fmt::Display> fmt::Display for Rle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, run) in self.runs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}×{}", run.value, run.count)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_new_is_empty() {
        let rle: Rle<i32> = Rle::new();
        assert!(rle.is_empty());
        assert_eq!(rle.len(), 0);
        assert_eq!(rle.run_count(), 0);
    }

    #[test]
    fn rle_push_single() {
        let mut rle = Rle::new();
        rle.push(42);
        assert_eq!(rle.len(), 1);
        assert_eq!(rle.run_count(), 1);
        assert_eq!(rle.get(0), Some(&42));
    }

    #[test]
    fn rle_push_merge_same() {
        let mut rle = Rle::new();
        rle.push(5);
        rle.push(5);
        rle.push(5);
        assert_eq!(rle.len(), 3);
        assert_eq!(rle.run_count(), 1); // Merged into 1 run
    }

    #[test]
    fn rle_push_different() {
        let mut rle = Rle::new();
        rle.push(1);
        rle.push(2);
        rle.push(3);
        assert_eq!(rle.len(), 3);
        assert_eq!(rle.run_count(), 3); // 3 different runs
    }

    #[test]
    fn rle_push_n() {
        let mut rle = Rle::new();
        rle.push_n(7, 5);
        assert_eq!(rle.len(), 5);
        assert_eq!(rle.run_count(), 1);
        assert_eq!(rle.get(0), Some(&7));
        assert_eq!(rle.get(4), Some(&7));
    }

    #[test]
    fn rle_push_n_merge() {
        let mut rle = Rle::new();
        rle.push(3);
        rle.push_n(3, 4);
        assert_eq!(rle.len(), 5);
        assert_eq!(rle.run_count(), 1); // Merged
    }

    #[test]
    fn rle_get_out_of_bounds() {
        let rle = Rle::from_iter([1, 2, 3]);
        assert_eq!(rle.get(3), None);
    }

    #[test]
    fn rle_set_same_value() {
        let mut rle = Rle::from_iter([5, 5, 5]);
        rle.set(1, 5);
        assert_eq!(rle.run_count(), 1); // No split
        assert_eq!(rle.len(), 3);
    }

    #[test]
    fn rle_set_split_middle() {
        let mut rle = Rle::from_iter([1, 1, 1]);
        rle.set(1, 9);
        assert_eq!(rle.len(), 3);
        assert_eq!(rle.get(0), Some(&1));
        assert_eq!(rle.get(1), Some(&9));
        assert_eq!(rle.get(2), Some(&1));
        assert_eq!(rle.run_count(), 3); // Split into 3 runs
    }

    #[test]
    fn rle_set_split_start() {
        let mut rle = Rle::from_iter([2, 2, 2]);
        rle.set(0, 8);
        assert_eq!(rle.get(0), Some(&8));
        assert_eq!(rle.get(1), Some(&2));
        assert_eq!(rle.run_count(), 2);
    }

    #[test]
    fn rle_set_split_end() {
        let mut rle = Rle::from_iter([3, 3, 3]);
        rle.set(2, 7);
        assert_eq!(rle.get(0), Some(&3));
        assert_eq!(rle.get(2), Some(&7));
        assert_eq!(rle.run_count(), 2);
    }

    #[test]
    fn rle_resize_grow() {
        let mut rle = Rle::from_iter([1, 2]);
        rle.resize(5, 9);
        assert_eq!(rle.len(), 5);
        assert_eq!(rle.get(4), Some(&9));
    }

    #[test]
    fn rle_resize_shrink() {
        let mut rle = Rle::from_iter([1, 2, 3, 4]);
        rle.resize(2, 0);
        assert_eq!(rle.len(), 2);
        assert_eq!(rle.get(0), Some(&1));
        assert_eq!(rle.get(1), Some(&2));
    }

    #[test]
    fn rle_truncate() {
        let mut rle = Rle::from_iter([1, 1, 2, 2, 3, 3]);
        rle.truncate(4);
        assert_eq!(rle.len(), 4);
        assert_eq!(rle.run_count(), 2); // 1×2, 2×2
    }

    #[test]
    fn rle_iter() {
        let rle = Rle::from_iter([1, 1, 2, 3, 3]);
        let collected: Vec<_> = rle.iter().collect();
        assert_eq!(collected, vec![1, 1, 2, 3, 3]);
    }

    #[test]
    fn rle_runs_iterator() {
        let rle = Rle::from_iter([5, 5, 7, 7, 7]);
        let runs: Vec<_> = rle.runs().map(|(v, c)| (*v, c)).collect();
        assert_eq!(runs, vec![(5, 2), (7, 3)]);
    }

    #[test]
    fn rle_clear() {
        let mut rle = Rle::from_iter([1, 2, 3]);
        rle.clear();
        assert!(rle.is_empty());
        assert_eq!(rle.len(), 0);
    }

    #[test]
    fn rle_from_iterator() {
        let rle: Rle<char> = "aaabbc".chars().collect();
        assert_eq!(rle.len(), 6);
        assert_eq!(rle.run_count(), 3); // a×3, b×2, c×1
    }
}
