pub fn simple_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches = vec![];

    for item in items {
        match item.find(text) {
            Some(position) => matches.push((item.clone(), position as f64)),
            None => continue
        }
    }
    let mut results = vec![];

    matches.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap());

    for result_item in matches {
        results.push(result_item.0);
    }
    results
}

pub fn dmenu_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches_exact = vec![];
    let mut matches_prefix = vec![];
    let mut matches_substring = vec![];

    if text.len() == 0 { return items.clone() }

    let words: Vec<&str> = text.split_whitespace().collect();
    for item in items {
        let mut exact = false;
        let mut prefix = false;
        let mut substring = false;

        for word in &words {
            match item.find(word) {
                Some(0) => {
                    if word == item { exact = true }
                    else { prefix = true }
                },
                Some(_) => substring = true,
                None => {
                    exact = false;
                    prefix = false;
                    substring = false;
                    break
                }
            }
        }

        if exact {
            matches_exact.push(item.clone())
        } else if prefix {
            matches_prefix.push(item.clone())
        } else if substring {
            matches_substring.push(item.clone())
        }
    }
    let mut results = vec![];
    results.extend(matches_exact);
    results.extend(matches_prefix);
    results.extend(matches_substring);
    results
}

pub fn fuzzy_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches = vec![];

    for item in items {
        let item_match = fuzzy_find_match(text, item);
        if item_match.1 > 0.0 {
            matches.push(item_match);
        }
    }
    let mut results = vec![];

    matches.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());

    for result_item in matches {
        results.push(result_item.0);
    }
    results
}

pub fn fuzzy_find_match(text: &String, item: &String) -> (String, f64) {
    let mut score = 1.0;
    let mut item_copy = item.clone();

    for c in text.chars() {
        score += match item_copy.find(c) {
            Some(position) => (10.0 - position as f64),
            None => {
                score = 0.0;
                break
            }
        };
        item_copy = item_copy.chars().skip_while(|&x| x != c).collect()
    }

    return (item.clone(), score)
}
