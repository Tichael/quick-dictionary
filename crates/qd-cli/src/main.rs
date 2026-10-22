//! `qd`: look words up in Quick Dictionary language packs from a terminal.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use qd_core::{Dictionary, MatchKind, SearchResult};

const USAGE: &str = "\
Usage: qd [OPTIONS] WORD...

Looks each WORD up in every language pack.

Options:
  --packs DIR    directory holding the *.db packs (default: $QD_PACKS, or ./packs);
                 search indexes are cached in DIR/.cache
  --complete     list completions for each WORD instead of looking it up
  --full         show every definition, example and form
  --json         print results as JSON
  --limit N      maximum number of results per word (default: 8)
  --time         print timings to stderr";

struct Options {
    packs: PathBuf,
    complete: bool,
    full: bool,
    json: bool,
    limit: usize,
    time: bool,
    words: Vec<String>,
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        packs: std::env::var_os("QD_PACKS").map(PathBuf::from).unwrap_or_else(|| "packs".into()),
        complete: false,
        full: false,
        json: false,
        limit: 8,
        time: false,
        words: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--packs" => options.packs = args.next().ok_or("--packs needs a directory")?.into(),
            "--limit" => {
                options.limit = args
                    .next()
                    .and_then(|n| n.parse().ok())
                    .ok_or("--limit needs a number")?
            }
            "--complete" => options.complete = true,
            "--full" => options.full = true,
            "--json" => options.json = true,
            "--time" => options.time = true,
            "-h" | "--help" => return Err(String::new()),
            _ if arg.starts_with("--") => return Err(format!("unknown option {arg}")),
            _ => options.words.push(arg),
        }
    }
    if options.words.is_empty() {
        return Err(String::new());
    }
    Ok(options)
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("qd: {message}\n");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let started = Instant::now();
    let cache_dir = options.packs.join(".cache");
    let dictionary = match Dictionary::open_dir(&options.packs, Some(&cache_dir)) {
        Ok(d) if d.packs.is_empty() => {
            eprintln!("qd: no *.db packs in {}", options.packs.display());
            return ExitCode::FAILURE;
        }
        Ok(d) => d,
        Err(e) => {
            eprintln!("qd: {e}");
            return ExitCode::FAILURE;
        }
    };
    if options.time {
        let packs: Vec<_> = dictionary
            .packs
            .iter()
            .map(|p| format!("{} ({} keys)", p.lang, p.key_count()))
            .collect();
        eprintln!("opened {} in {:.0?}", packs.join(", "), started.elapsed());
    }

    for word in &options.words {
        let started = Instant::now();
        if options.complete {
            let suggestions = dictionary.complete(word, options.limit);
            if options.json {
                println!("{}", serde_json::to_string_pretty(&suggestions).unwrap());
            } else {
                let keys: Vec<_> = suggestions.iter().map(|s| format!("{} ({})", s.key, s.lang)).collect();
                println!("{word}: {}", keys.join(", "));
            }
        } else {
            match dictionary.search(word, options.limit) {
                Ok(results) if options.json => {
                    println!("{}", serde_json::to_string_pretty(&results).unwrap())
                }
                Ok(results) => print_results(word, &results, options.full),
                Err(e) => {
                    eprintln!("qd: {word}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        if options.time {
            eprintln!("{word}: {:.1?}", started.elapsed());
        }
    }
    ExitCode::SUCCESS
}

fn print_results(query: &str, results: &[SearchResult], full: bool) {
    if results.is_empty() {
        println!("{query}: no match\n");
        return;
    }
    if results[0].distance > 0 {
        let mut spellings: Vec<&str> = Vec::new();
        for result in results {
            let spelling = match &result.matched_form {
                Some(form) => form.form.as_str(),
                None if result.kind == MatchKind::Headword => result.entry.word.as_str(),
                None => result.key.as_str(),
            };
            if !spellings.contains(&spelling) {
                spellings.push(spelling);
            }
        }
        println!("{query}: no exact match. Did you mean: {}?", spellings.join(", "));
    }
    for result in results {
        let entry = &result.entry;
        let mut header = format!("{} · {}", entry.word, entry.pos);
        if !entry.tags.is_empty() {
            header += &format!(" ({})", entry.tags.join(", "));
        }
        if let Some(ipa) = &entry.ipa {
            header += &format!("  {ipa}");
        }
        println!("[{}] {header}", result.lang);

        match (&result.matched_form, result.kind) {
            (Some(form), _) => println!("    ↳ {} ({})", form.form, form.tags.join(" ")),
            (None, MatchKind::FormOf) => {
                println!("    ↳ {}", result.note.as_deref().unwrap_or("form of this word"))
            }
            _ => {}
        }

        let senses = if full { entry.senses.len() } else { 3 };
        for sense in entry.senses.iter().take(senses) {
            let label = if sense.tags.is_empty() { String::new() } else { format!(" [{}]", sense.tags.join(", ")) };
            println!("    - {}{label}", sense.glosses.join(" › "));
            if full {
                for example in &sense.examples {
                    println!("        « {example} »");
                }
            }
        }
        if entry.senses.len() > senses {
            println!("    … {} more definitions", entry.senses.len() - senses);
        }

        let groups = if full { entry.forms.len() } else { 4 };
        for group in entry.forms.iter().take(groups) {
            let tags: Vec<_> = group.tags.iter().chain(&group.raw_tags).map(String::as_str).collect();
            println!("    {}: {}", tags.join(" "), group.forms.join(", "));
        }
        if entry.forms.len() > groups {
            println!("    … {} more form groups (--full)", entry.forms.len() - groups);
        }
    }
    println!();
}
