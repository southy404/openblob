pub fn normalize(input: &str) -> String {
    let lower = input.trim().to_lowercase();
    let replaced = lower
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss");

    let mut out = String::with_capacity(replaced.len());
    let mut prev_space = false;

    for ch in replaced.chars() {
        if ch.is_ascii_alphanumeric() || ch == ' ' {
            if ch == ' ' {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            } else {
                out.push(ch);
                prev_space = false;
            }
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }

    out.trim().to_string()
}

pub fn tokens(text: &str) -> Vec<&str> {
    text.split_whitespace().filter(|t| !t.is_empty()).collect()
}