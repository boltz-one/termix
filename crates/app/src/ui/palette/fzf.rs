//! Port of `windows/src/cascadia/fzf/fzf.cpp` (fzf `FuzzyMatchV2`, MIT — see
//! NOTICE). Runs are inclusive UTF-16 offsets like the C++ `TextRun`.
//!
//! Deviation: ICU `u_charType`/`u_foldCase` are replaced by `std::char`
//! predicates and simple lowercase folding (multi-char folds are left as-is);
//! `Nd` digits are approximated by `char::is_numeric`.

const SCORE_MATCH: i32 = 16;
const SCORE_GAP_START: i32 = -3;
const SCORE_GAP_EXTENSION: i32 = -1;
const BOUNDARY_BONUS: i32 = SCORE_MATCH / 2;
const NON_WORD_BONUS: i32 = SCORE_MATCH / 2;
const CAMEL_CASE_BONUS: i32 = BOUNDARY_BONUS + SCORE_GAP_EXTENSION;
const BONUS_CONSECUTIVE: i32 = -(SCORE_GAP_START + SCORE_GAP_EXTENSION);
const BONUS_FIRST_CHAR_MULTIPLIER: i32 = 2;

/// Inclusive UTF-16 range of a highlighted run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchResult {
    pub score: i32,
    pub runs: Vec<TextRun>,
}

/// Space-separated search terms, case-folded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pattern {
    pub terms: Vec<Vec<char>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharClass {
    NonWord,
    Lower,
    Upper,
    Digit,
}

fn fold(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn class_of(c: char) -> CharClass {
    if c.is_uppercase() {
        CharClass::Upper
    } else if c.is_lowercase() || c.is_alphabetic() {
        CharClass::Lower
    } else if c.is_numeric() {
        CharClass::Digit
    } else {
        CharClass::NonWord
    }
}

fn bonus_of(prev: CharClass, cur: CharClass) -> i32 {
    use CharClass::*;
    if prev == NonWord && cur != NonWord {
        BOUNDARY_BONUS
    } else if (prev == Lower && cur == Upper) || (prev != Digit && cur == Digit) {
        CAMEL_CASE_BONUS
    } else if cur == NonWord {
        NON_WORD_BONUS
    } else {
        0
    }
}

/// `asciiFuzzyIndex`: index to start scanning from, or `None` if the pattern
/// is not a subsequence of `input`.
fn fuzzy_index(input: &[char], pattern: &[char]) -> Option<usize> {
    let (mut idx, mut first) = (0usize, 0usize);
    for (pi, &pc) in pattern.iter().enumerate() {
        idx += input.get(idx..)?.iter().position(|&c| c == pc)?;
        if pi == 0 && idx > 0 {
            first = idx - 1;
        }
        idx += 1;
    }
    Some(first)
}

/// `fzfFuzzyMatchV2`; returns the score (0 = no match) and matched positions.
fn fuzzy_match_v2(text: &[char], pattern: &[char]) -> (i32, Vec<usize>) {
    if pattern.is_empty() {
        return (0, vec![]);
    }
    let folded: Vec<char> = text.iter().map(|&c| fold(c)).collect();
    let Some(first_index_of) = fuzzy_index(&folded, pattern) else {
        return (0, vec![]);
    };
    let n = text.len();
    let mut initial = vec![0i32; n];
    let mut consecutive_scores = vec![0i32; n];
    let mut bonuses = vec![0i32; n];
    let mut first_occ = vec![0usize; pattern.len()];
    let (mut max_score, mut max_pos) = (0i32, 0usize);
    let (mut pattern_index, mut last_index) = (0usize, 0usize);
    let first_pc = pattern[0];
    let mut cur_pc = pattern[0];
    let (mut prev_initial, mut prev_class, mut in_gap) = (0i32, CharClass::NonWord, false);

    for i in first_index_of..n {
        let ch = folded[i];
        let class = class_of(text[i]);
        let bonus = bonus_of(prev_class, class);
        bonuses[i] = bonus;
        prev_class = class;
        if ch == cur_pc {
            if pattern_index < pattern.len() {
                first_occ[pattern_index] = i;
                pattern_index += 1;
                if pattern_index < pattern.len() {
                    cur_pc = pattern[pattern_index];
                }
            }
            last_index = i;
        }
        if ch == first_pc {
            let score = SCORE_MATCH + bonus * BONUS_FIRST_CHAR_MULTIPLIER;
            initial[i] = score;
            consecutive_scores[i] = 1;
            if pattern.len() == 1 && score > max_score {
                max_score = score;
                max_pos = i;
                if bonus == BOUNDARY_BONUS {
                    break;
                }
            }
            in_gap = false;
        } else {
            let gap = if in_gap { SCORE_GAP_EXTENSION } else { SCORE_GAP_START };
            initial[i] = (prev_initial + gap).max(0);
            consecutive_scores[i] = 0;
            in_gap = true;
        }
        prev_initial = initial[i];
    }
    if pattern_index != pattern.len() {
        return (0, vec![]);
    }
    if pattern.len() == 1 {
        return (max_score, vec![max_pos]);
    }

    let off0 = first_occ[0];
    let width = last_index - off0 + 1;
    let rows = pattern.len();
    let mut score_m = vec![0i32; width * rows];
    let mut consec_m = vec![0i32; width * rows];
    score_m[..width].copy_from_slice(&initial[off0..off0 + width]);
    consec_m[..width].copy_from_slice(&consecutive_scores[off0..off0 + width]);

    for off in 0..rows - 1 {
        let pco = first_occ[off + 1];
        let slice_len = last_index - pco + 1;
        let pc = pattern[off + 1];
        let row = (off + 1) * width;
        let base = row + pco - off0; // cell of slice[0]
        let mut in_gap = false;
        score_m[base - 1] = 0; // scoreMatrixLeftSlice[0]
        for j in 0..slice_len {
            let ch = folded[pco + j];
            let col = pco + j;
            let left = score_m[base + j - 1];
            let score = left + if in_gap { SCORE_GAP_EXTENSION } else { SCORE_GAP_START };
            let (mut diag, mut consec) = (0i32, 0i32);
            if ch == pc {
                diag = score_m[base + j - 1 - width] + SCORE_MATCH;
                let mut bonus = bonuses[col];
                consec = consec_m[base + j - 1 - width] + 1;
                if bonus == BOUNDARY_BONUS {
                    consec = 1;
                } else if consec > 1 {
                    bonus = bonus.max(BONUS_CONSECUTIVE).max(bonuses[col + 1 - consec as usize]);
                }
                if diag + bonus < score {
                    diag += bonuses[col];
                    consec = 0;
                } else {
                    diag += bonus;
                }
            }
            consec_m[base + j] = consec;
            in_gap = diag < score;
            let cell = 0.max(diag.max(score));
            if off + 2 == rows && cell > max_score {
                max_score = cell;
                max_pos = col;
            }
            score_m[base + j] = cell;
        }
    }

    // Backtrace.
    let mut pos = Vec::new();
    let mut pi = rows - 1;
    let mut col = max_pos as isize;
    let mut prefer_current = true;
    let size = width * rows;
    loop {
        let row_start = pi * width;
        let col_off = col - off0 as isize;
        if col_off < 0 {
            break;
        }
        let at = row_start + col_off as usize;
        let cell = score_m[at];
        let diag = if pi > 0 && col >= first_occ[pi] as isize {
            score_m[at - width - 1]
        } else {
            0
        };
        let left = if col > first_occ[pi] as isize { score_m[at - 1] } else { 0 };
        if cell > diag && (cell > left || (cell == left && prefer_current)) {
            pos.push(col as usize);
            if pi == 0 {
                break;
            }
            pi -= 1;
        }
        col -= 1;
        if at >= size {
            break;
        }
        prefer_current = consec_m[at] > 1
            || (at + width + 1 < size && consec_m[at + width + 1] > 0);
    }
    (max_score, pos)
}

/// `ParsePattern`.
pub fn parse_pattern(s: &str) -> Pattern {
    Pattern {
        terms: s
            .split(' ')
            .filter(|w| !w.is_empty())
            .map(|w| w.chars().map(fold).collect())
            .collect(),
    }
}

/// `Match`: `None` when any term fails; empty pattern → score 0, no runs.
pub fn match_text(text: &str, pattern: &Pattern) -> Option<MatchResult> {
    if pattern.terms.is_empty() {
        return Some(MatchResult::default());
    }
    let cps: Vec<char> = text.chars().collect();
    let mut total = 0;
    let mut all = Vec::new();
    for term in &pattern.terms {
        let (score, pos) = fuzzy_match_v2(&cps, term);
        if score <= 0 {
            return None;
        }
        total += score;
        all.extend(pos);
    }
    all.sort_unstable();
    all.dedup();

    let mut runs = Vec::new();
    let (mut next, mut u16_off) = (0usize, 0usize);
    let mut run_start: Option<usize> = None;
    for (i, c) in cps.iter().enumerate() {
        if next < all.len() && all[next] == i {
            run_start.get_or_insert(u16_off);
            next += 1;
        } else if let Some(start) = run_start.take() {
            runs.push(TextRun { start, end: u16_off - 1 });
        }
        u16_off += c.len_utf16();
    }
    if let Some(start) = run_start {
        runs.push(TextRun { start, end: u16_off - 1 });
    }
    Some(MatchResult { score: total, runs })
}
