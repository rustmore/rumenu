use std::collections::HashMap;
use std::cmp::Ordering;

#[derive(PartialEq)]
pub struct MatchObj {
    text: String,
    score: f64,
}

impl PartialOrd for MatchObj {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.score.partial_cmp(&other.score) {
            Some(Ordering::Equal) => self.text.partial_cmp(&other.text),
            Some(ordering) => Some(ordering),
            None => None
        }
    }
}

pub struct MatchInfo{
    haystack: String,
    needle: String,
    max_score_per_char: f64,
}

pub fn simple_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches = vec![];

    for item in items {
        match item.find(text) {
            Some(position) => matches.push(MatchObj { text: item.clone(), score: position as f64 }),
            None => continue
        }
    }
    let mut results = vec![];

    matches.sort_by(|x, y| x.partial_cmp(y).unwrap());

    for result_item in matches {
        results.push(result_item.text);
    }
    results
}

pub fn ctrlp_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches = vec![];

    for item in items {
        matches.push(ctrlp_find_match(item, text));
    }
    let mut results = vec![];

    matches.sort_by(|x, y| x.partial_cmp(y).unwrap());

    for result_item in matches {
        results.push(result_item.text);
    }
    results
}

pub fn ctrlp_find_match(text: &String, abbrev: &String) -> MatchObj {
    let m = MatchInfo {
        haystack: text.clone(),
        needle: abbrev.clone(),
        max_score_per_char: (1.0 as f64 / text.len() as f64 + 1.0 as f64 / abbrev.len() as f64 ) / 2.0 as f64 ,
    };

    // calculate score
    let mut score = 1.0;

    // special case for zero-length search string
    if m.needle.len() == 0 {
        if m.haystack.starts_with('.') {
           score = 0.0
       }
    } else if m.haystack.len() > 0 { // normal case
        let mut memo = HashMap::new();

        score = ctrlp_recursive_match(&m, &mut memo, 0, 0, 0, 0.0);
    }

    return MatchObj {
        text: text.clone(),
        score: score,
    }
}

fn ctrlp_recursive_match(m: &MatchInfo, memo: &mut HashMap<i64,f64>, haystack_idx: i64, needle_idx: i64,
                         last_idx: i64, rec_score: f64) -> f64 {
    let mut seen_score = 0.0;
    let memo_idx = haystack_idx;

    let mut score = match memo.get(&(needle_idx * m.needle.len() as i64 + memo_idx)) {
        Some(s) => *s,
        None => rec_score,
    };

    // bail early if not enough room (left) in haystack for (rest of) needle
    if m.haystack.len() as i64 - haystack_idx < m.needle.len() as i64 - needle_idx {
        score = 0.0;
        memo.insert(needle_idx * m.needle.len() as i64 + memo_idx, score);
        return score;
    }

    let mut needle_chars = m.needle.chars();
    let mut haystack_chars = m.needle.chars();
    let needle_chars_max = m.needle.chars().count() as i64;
    for i in 0..needle_chars_max {
        let c = needle_chars.nth(i as usize).unwrap();
        let mut found = false;
        let mut last_idx = last_idx;

        let haystack_chars_max = m.haystack.chars().count() as i64 - m.needle.chars().count() as i64 - i;
        for j in haystack_idx..haystack_chars_max {
            let mut d = needle_chars.nth(j as usize).unwrap();

            if d >= 'A' && d <= 'Z' {
                d = d.to_lowercase().next().unwrap()
            }

            if c == d {
                found = true;

                // calculate score
                let mut score_for_char = m.max_score_per_char;
                let distance = j - last_idx;

                if distance > 1 {
                    let mut factor;
                    let last = haystack_chars.nth((j - 1) as usize).unwrap();
                    let curr = haystack_chars.nth(j as usize).unwrap();

                    match last {
                        '/' => factor = 0.9,
                        '-' | '_' | ' ' | '0' ... '9' => factor = 0.8,
                        'a' ... 'z' if curr >= 'A' && curr <= 'Z' => factor = 0.8,
                        '.'  => factor = 0.7,
                        _ => factor = (1.0 / distance as f64) * 0.75
                    }

                    score_for_char *= factor;
                }

                // j += 1;

                if j + 1< m.haystack.chars().count() as i64 {
                    let sub_score = ctrlp_recursive_match(m, memo, j + 1, i, last_idx, score);
                    if sub_score > seen_score {
                        seen_score = sub_score;
                    }
                }

                score += score_for_char;
                let haystack_idx = haystack_idx + 1;
                last_idx = haystack_idx;
                break;
            }
        }

        if !found {
            score = 0.0;
            memo.insert(needle_idx * m.needle.len() as i64 + memo_idx, score);
            return score;
        }
    }

    if score < seen_score {
        score = seen_score;
    }
    memo.insert(needle_idx * m.needle.len() as i64 + memo_idx, score);
    return score
}
