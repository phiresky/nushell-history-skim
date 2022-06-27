use std::{path::PathBuf, time::Duration};

use ansi_term::ANSIGenericString;
use chrono::{DateTime, Utc};
use clap::Parser;
use enum_map::enum_map;
use enum_map::Enum;
use nu_path;
use reedline::CommandLineSearch;
use reedline::SearchDirection;
use reedline::SearchFilter;
use reedline::{History, HistoryItemId};
use reedline::{HistoryItem, SearchQuery};
use skim::prelude::*;

#[derive(clap::Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(default_value = "")]
    query: String,
}

#[derive(PartialEq, Enum, Copy, Clone)]
pub enum Location {
    Session,
    Directory,
    Machine,
    Everywhere,
}

fn get_current_session_id() -> i64 {
    1
}
fn get_current_dir() -> String {
    std::env::current_dir()
        .unwrap()
        .to_string_lossy()
        .to_string()
}
fn get_current_host() -> String {
    gethostname::gethostname().to_string_lossy().to_string()
}

pub fn generate_title(location: &Location) -> String {
    let extra_info = |theloc: &Location| -> String {
        return match theloc {
            Location::Session => get_current_session_id().to_string(),
            Location::Directory => get_current_dir(),
            Location::Machine => get_current_host(),
            _ => String::from(""),
        };
    }(&location);

    let location_map = enum_map! {
        Location::Session => "Session history",
        Location::Directory => "Directory history",
        Location::Machine => "Machine history",
        Location::Everywhere => "Everywhere",
    };

    let header_map = enum_map! {
        Location::Session =>
"
 ┏━━━━━━━┱─────────┬────┬──────────┐
 ┃Session┃Directory│Host│Everywhere│ 
━┛       ┗━━━━━━━━━┷━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━",
        Location::Directory =>
"
 ┌───────┲━━━━━━━━━┱────┬──────────┐
 │Session┃Directory┃Host│Everywhere│ 
━┷━━━━━━━┛         ┗━━━━┷━━━━━━━━━━┷━━━━━━━━━━━━━━━━━",

        Location::Machine =>
"
 ┌───────┬─────────┲━━━━┱──────────┐
 │Session│Directory┃Host┃Everywhere│ 
━┷━━━━━━━┷━━━━━━━━━┛    ┗━━━━━━━━━━┷━━━━━━━━━━━━━━━━━",

        Location::Everywhere =>
"
 ┌───────┬─────────┬────┲━━━━━━━━━━┓
 │Session│Directory│Host┃Everywhere┃ 
━┷━━━━━━━┷━━━━━━━━━┷━━━━┛          ┗━━━━━━━━━━━━━━━━━",
    };

    let title = format!(
        "{} {}\n{}\n",
        &location_map[location.clone()].trim(),
        &extra_info,
        &header_map[location.clone()],
    );
    return title.to_string();
}

struct HistoryItemSkim(HistoryItem);

fn pretty_date_str(d: DateTime<Utc>) -> String {
    let d = d.with_timezone(&chrono::offset::Local);
    if d.date() == chrono::offset::Local::today() {
        d.format("%H:%M").to_string()
    } else {
        d.format("%F %H:%M").to_string()
    }
}
fn pretty_duration_str(d: Duration) -> String {
    if d < Duration::from_secs(1) {
        return format!("{:>DURATION_FORMAT_LENGTH$.1} s", d.as_secs_f64());
    }
    if d < Duration::from_secs(60) {
        return format!("{:>DURATION_FORMAT_LENGTH$} s", d.as_secs());
    }
    if d < Duration::from_secs(60 * 60) {
        return format!("{:>DURATION_FORMAT_LENGTH$} m", d.as_secs() / 60);
    }
    return format!("{:>DURATION_FORMAT_LENGTH$} h", d.as_secs() / 60 / 60);
}
fn ansi_duration_str(d: Duration) -> String {
    let s = pretty_duration_str(d);
    if d < Duration::from_secs(5) {
        return ansi_term::Style::default().paint(s).to_string();
    }
    if d < Duration::from_secs(60) {
        return ansi_term::Color::Yellow.paint(s).to_string();
    }
    return ansi_term::Color::Red.paint(s).to_string();
}
const DATE_FORMAT_LENGTH: usize = 16;
const DURATION_FORMAT_LENGTH: usize = 3;
impl SkimItem for HistoryItemSkim {
    fn text(&self) -> Cow<str> {
        (&self.0.command_line).into()
    }

    fn display<'a>(&'a self, context: DisplayContext<'a>) -> AnsiString<'a> {
        let item = &self.0;
        let date = item
            .start_timestamp
            .map(pretty_date_str)
            .unwrap_or("??:??".to_string());
        let duration = item
            .duration
            .map(ansi_duration_str)
            .unwrap_or("     ".to_string());
        let cmd = &item.command_line;

        AnsiString::parse(&format!(
            "{date: >DATE_FORMAT_LENGTH$} | {duration} | {cmd}"
        ))
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        let dbg = format!("{:?}", self.0);
        let item = &self.0;
        use ansi_term::{Colour::*, Style};

        ItemPreview::AnsiText(format!(
            "{}
Host: {}
Directory: {}
Session: {}
Timestamp: {}
Duration: {}
{}
Command:

{}
",
            Style::new().bold().paint(
                item.id
                    .map(|id| format!("Details for entry {id:?}"))
                    .unwrap()
            ),
            item.hostname.as_ref().unwrap_or(&"<unknown>".to_string()),
            item.cwd.as_ref().unwrap_or(&"<unknown>".to_string()),
            item.session_id
                .map(|e| format!("{e:?}"))
                .unwrap_or("<unknown>".to_string()),
            item.start_timestamp
                .map(|e| e.with_timezone(&chrono::Local).to_string())
                .unwrap_or("<unknown>".to_string()),
            item.duration
                .map(ansi_duration_str)
                .unwrap_or("<unknown>".to_string()),
            if item.exit_status == Some(0) {
                Green.paint("Exit Status: 0")
            } else {
                Red.paint(format!(
                    "Exit Status: {}",
                    item.exit_status
                        .map(|e| e.to_string())
                        .unwrap_or("<unknown>".to_string())
                ))
            },
            item.command_line,
        ))
    }

    fn output(&self) -> Cow<str> {
        // output only contains command line
        (&self.0.command_line).into()
    }

    //fn get_matching_ranges(&self) -> Option<&[(usize, usize)]> {
    //    return Some(&[(DATE_FORMAT_LENGTH, 10000)])
    //}
}

fn send_entries(location: Location, start_query: &str, sender: SkimItemSender) {
    let mut path = nu_path::config_dir().unwrap();
    path.push("nushell");
    path.push("history.sqlite3");
    let history = reedline::SqliteBackedHistory::with_file(path).unwrap();
    let mut filter = SearchFilter::anything();
    filter.command_line = Some(CommandLineSearch::Substring(start_query.to_string()));
    filter.hostname =  if location == Location::Everywhere {
        None
    } else {
        Some(get_current_host())
    };
    filter.cwd_exact = if location == Location::Directory {
        Some(get_current_dir())
    } else {
        None
    };
    let res = history
        .search(SearchQuery {
            direction: SearchDirection::Backward,
            start_time: None,
            end_time: None,
            start_id: None,
            end_id: None,
            limit: None,
            filter
        })
        .unwrap();
    for item in res {
        sender.send(Arc::new(HistoryItemSkim(item))).unwrap();
    }
}

fn show_history(query: String) {
    let mut location = Location::Directory;
    loop {
        let title = generate_title(&location);
        let options = SkimOptionsBuilder::default()
            .height(Some("100%"))
            .multi(false)
            .reverse(true)
            .prompt(Some("history〉"))
            .query(Some(&query))
            .bind(vec!["ctrl-r:abort"])
            .header(Some(&title))
            .preview(Some(""))
            .build()
            .unwrap();

        let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

        let query_clone = query.clone();
        let handle = std::thread::spawn(move || {
            send_entries(location, &query_clone, tx_item);
        });

        let output = Skim::run_with(&options, Some(rx_item));
        handle.join().unwrap();
        if let Some(o) = output {
            match o.final_key {
                Key::ESC | Key::Ctrl('c') | Key::Ctrl('d') | Key::Ctrl('z') => {
                    break;
                }
                Key::Enter => {
                    let sel = o.selected_items;
                    let arr: Vec<_> = sel.iter().map(|e| e.output()).collect();
                    let ele = &arr[0];
                    println!("Selected: {ele}");
                    break;
                }
                Key::Ctrl('r') => {
                    location = match location {
                        Location::Session => Location::Directory,
                        Location::Directory => Location::Machine,
                        Location::Machine => Location::Everywhere,
                        Location::Everywhere => Location::Session,
                    };
                }
                _ => {}
            }
        } else {
            // internal error in skim
            break;
        }
    }
}
fn main() {
    let args = Args::parse();
    show_history(args.query)
}
