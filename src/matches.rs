pub fn simple_match(text: &String, items: &Vec<String>) -> Vec<String> {
    let mut matches = vec![];

    for item in items {
        match item.find(text) {
            Some(_) => {matches.push(item.clone())},
            None => continue
        }
    }
    matches
}
