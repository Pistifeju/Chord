//! Tells a read-only shell command from one that may change something, so the chat can label it Read, Search or List.

/// What one piped or chained part contributes to the command's title.
enum Part {
    /// Changes nothing and says nothing about the command: `cd`, `echo`.
    Neutral,
    Title(&'static str),
}

/// The title of a command whose every piped or chained part only reads; the first part that names itself names the command.
pub fn read_only_title(command: &str) -> Option<&'static str> {
    // Substitutions and newlines can run anything.
    if command.contains(['`', '\n'])
        || command.contains("$(")
        || command.contains("<(")
        || command.contains("<<")
    {
        return None;
    }
    let mut title = None;
    for words in split_parts(command)? {
        if let Part::Title(part_title) = part_of(&words)? {
            title.get_or_insert(part_title);
        }
    }
    title
}

/// The words of each part between `|`, `&&`, `||` and `;`, split outside quotes. None when a redirect may write or a job runs in the background.
fn split_parts(command: &str) -> Option<Vec<Vec<String>>> {
    let chars: Vec<char> = command.chars().collect();
    let mut parts = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut i = 0;
    macro_rules! end_word {
        () => {
            if in_word {
                words.push(std::mem::take(&mut word));
                in_word = false;
            }
        };
    }
    macro_rules! end_part {
        () => {
            end_word!();
            if !words.is_empty() {
                parts.push(std::mem::take(&mut words));
            }
        };
    }
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    word.push(chars[i]);
                    i += 1;
                }
            }
            '"' => {
                in_word = true;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    word.push(chars[i]);
                    i += 1;
                }
            }
            '\\' if i + 1 < chars.len() => {
                in_word = true;
                i += 1;
                word.push(chars[i]);
            }
            c if c.is_whitespace() => end_word!(),
            '|' | ';' => {
                end_part!();
                if c == '|' && chars.get(i + 1) == Some(&'|') {
                    i += 1;
                }
            }
            '&' if chars.get(i + 1) == Some(&'&') => {
                end_part!();
                i += 1;
            }
            '&' if chars.get(i + 1) != Some(&'>') => return None,
            '>' | '&' => {
                // A file descriptor number right before the redirect belongs to it.
                if c == '>' && in_word && word.chars().all(|d| d.is_ascii_digit()) {
                    word.clear();
                    in_word = false;
                }
                end_word!();
                if c == '&' {
                    i += 1;
                }
                while chars.get(i + 1) == Some(&'>') {
                    i += 1;
                }
                if chars.get(i + 1) == Some(&'&') {
                    // `2>&1` and `2>&-` duplicate or close a descriptor.
                    let mut next = i + 2;
                    if !matches!(chars.get(next), Some(d) if d.is_ascii_digit() || *d == '-') {
                        return None;
                    }
                    while matches!(chars.get(next), Some(d) if d.is_ascii_digit() || *d == '-') {
                        next += 1;
                    }
                    i = next - 1;
                } else {
                    let mut next = i + 1;
                    while matches!(chars.get(next), Some(w) if w.is_whitespace()) {
                        next += 1;
                    }
                    let target: String = chars[next..]
                        .iter()
                        .take_while(|t| !t.is_whitespace() && !matches!(t, '|' | ';' | '&'))
                        .collect();
                    if target != "/dev/null" {
                        return None;
                    }
                    i = next + target.chars().count() - 1;
                }
            }
            c => {
                in_word = true;
                word.push(c);
            }
        }
        i += 1;
    }
    if in_word {
        words.push(word);
    }
    if !words.is_empty() {
        parts.push(words);
    }
    Some(parts)
}

fn part_of(words: &[String]) -> Option<Part> {
    let program = words.first()?.rsplit('/').next()?;
    let rest: Vec<&str> = words[1..].iter().map(String::as_str).collect();
    let title = match program {
        "cd" if !rest.is_empty() => return Some(Part::Neutral),
        "echo" => return Some(Part::Neutral),
        "cat" | "head" | "tail" | "wc" | "bat" | "less" | "more" | "stat" | "file" | "nl"
        | "cut" | "tr" => "Read",
        "grep" | "egrep" | "fgrep" | "rg" | "ag" => "Search",
        "find" if !rest.iter().any(|word| is_find_action(word)) => "Search",
        "ls" | "pwd" | "du" | "df" => "List",
        "tree" if !has_short_flag(&rest, 'o') && !rest.contains(&"--output") => "List",
        "sort"
            if !has_short_flag(&rest, 'o')
                && !rest.iter().any(|word| {
                    word.starts_with("--output") || word.starts_with("--compress-program")
                }) =>
        {
            "Read"
        }
        "uniq" if rest.iter().filter(|word| !word.starts_with('-')).count() <= 1 => "Read",
        "sed" if is_sed_print(&rest) => "Read",
        "git" => return git_title(&rest).map(Part::Title),
        _ => return None,
    };
    Some(Part::Title(title))
}

fn has_short_flag(args: &[&str], flag: char) -> bool {
    args.iter()
        .any(|word| word.starts_with('-') && !word.starts_with("--") && word.contains(flag))
}

fn is_find_action(word: &str) -> bool {
    matches!(
        word,
        "-exec"
            | "-execdir"
            | "-ok"
            | "-okdir"
            | "-delete"
            | "-fprint"
            | "-fprint0"
            | "-fprintf"
            | "-fls"
    )
}

// `sed -n 'script' file` prints; `-i`, `-e`, `-f` and the `w` and `e` commands write or run.
fn is_sed_print(args: &[&str]) -> bool {
    let flags: Vec<&str> = args
        .iter()
        .copied()
        .filter(|word| word.starts_with('-'))
        .collect();
    if !has_short_flag(&flags, 'n')
        || has_short_flag(&flags, 'i')
        || has_short_flag(&flags, 'e')
        || has_short_flag(&flags, 'f')
    {
        return false;
    }
    if flags
        .iter()
        .any(|word| word.starts_with("--") && *word != "--quiet" && *word != "--silent")
    {
        return false;
    }
    let Some(script) = args.iter().find(|word| !word.starts_with('-')) else {
        return false;
    };
    script.ends_with(['p', 'q', '}'])
        && !["w ", "w/", ";w", "}w", ";e", "}e"]
            .iter()
            .any(|command| script.contains(command))
}

fn git_title(args: &[&str]) -> Option<&'static str> {
    let mut i = 0;
    while args.get(i) == Some(&"-C") {
        i += 2;
    }
    let subcommand = *args.get(i)?;
    let rest = &args[i + 1..];
    if rest.iter().any(|word| {
        word.starts_with("--output") || *word == "-O" || word.starts_with("--open-files-in-pager")
    }) {
        return None;
    }
    match subcommand {
        "status" | "log" | "diff" | "show" | "rev-parse" | "blame" => Some("Read"),
        "grep" => Some("Search"),
        "ls-files" => Some("List"),
        "branch" => {
            let listing = [
                "-a",
                "-r",
                "-v",
                "-vv",
                "--all",
                "--remotes",
                "--list",
                "-l",
                "--show-current",
                "--verbose",
            ];
            rest.iter()
                .all(|word| listing.contains(word))
                .then_some("List")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::read_only_title as title;

    #[test]
    fn a_reading_command_is_read() {
        assert_eq!(title("cat src/main.rs"), Some("Read"));
        assert_eq!(title("/bin/cat a | head -5"), Some("Read"));
        assert_eq!(title("sed -n '1,20p' a.rs"), Some("Read"));
        assert_eq!(title("git log --oneline"), Some("Read"));
    }

    #[test]
    fn a_search_or_listing_is_named_by_its_first_part() {
        assert_eq!(title("rg TODO src | wc -l"), Some("Search"));
        assert_eq!(title("ls -la && cat a"), Some("List"));
        assert_eq!(title("cd app && grep -rn foo ."), Some("Search"));
        assert_eq!(title("git ls-files"), Some("List"));
    }

    #[test]
    fn a_redirect_to_nowhere_still_reads() {
        assert_eq!(title("ls 2>/dev/null"), Some("List"));
        assert_eq!(title("grep foo src 2>&1 | head"), Some("Search"));
        assert_eq!(title("grep foo src &>/dev/null"), Some("Search"));
    }

    #[test]
    fn a_pipe_inside_quotes_does_not_split() {
        assert_eq!(title("grep -E \"a|b\" src"), Some("Search"));
        assert_eq!(title("grep 'a;rm x' src"), Some("Search"));
    }

    #[test]
    fn a_command_that_may_write_is_not_read_only() {
        for command in [
            "cat a > b",
            "cat a >> /dev/nullx",
            "find . -name x -delete",
            "find . -fprint out",
            "ls $(rm -rf x)",
            "cat a & rm b",
            "npm install",
            "echo hi",
            "cd src",
            "sort -o out in",
            "sort a --output=out",
            "uniq in out",
            "sed -i s/a/b/ f",
            "sed 's/a/b/' f",
            "sed -n 'w out' f",
            "git branch -D x",
            "git commit -m x",
        ] {
            assert_eq!(title(command), None, "{command}");
        }
    }
}
