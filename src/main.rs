use std::{fmt::Display, path::{Path, PathBuf}, time::Duration};

use clap::Parser;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(required = true)]
    files: Vec<PathBuf>,
    #[arg(long, required = true, value_delimiter = ',')]
    speakers: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.speakers.len() != cli.files.len() {
        return Err("differing number of speakers and files. every file needs one speaker defined".into());
    }

    let mut vtt = WebVTT::new();
    for (speaker, path) in cli.speakers.iter().zip(cli.files.iter()) {
        let mut file = load_vtt(path).map_err(|e| format!("while parsing {}: {}", path.to_string_lossy(), e))?;
        let orgiginal = file.clone();
        file.sort();
        if file != orgiginal {
            eprintln!("unsorted: {}", path.to_string_lossy());
        }
        file.set_speaker_for_all_lines(speaker);
        vtt.merge_with(file);
    }

    println!("{vtt}");
    Ok(())
}

fn load_vtt(file: &Path) -> Result<WebVTT, Box<dyn std::error::Error>> {
    Ok(WebVTT::from(&std::fs::read_to_string(file)?)?)
}

#[derive(Debug, Error)]
enum WebVTTError {
    #[error("parsing error: expected {0}, got '{1}'")]
    Parsing(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebVTT (Vec<WebVTTCue>);
impl WebVTT {
    pub fn new() -> Self { Self(vec![]) }
    pub fn from(string: &str) -> Result<Self, WebVTTError> {
        if !string.starts_with("WEBVTT") {
            return Err(WebVTTError::Parsing(
                "WEBVTT".to_owned(),
                string.lines().next().unwrap_or("").to_owned(),
            ));
        }
        let string = &string["WEBVTT".len()..];

        let mut lines: Vec<WebVTTCue> = vec![];

        let mut range: Option<Timerange> = None;
        for line in string.lines() {
            // eprintln!("Parsing: {}", line);
            if line.trim().is_empty() {
                range = None;
                continue;
            }
            match range {
                None => {
                    range = Some(Timerange::from(line)?);
                    continue;
                },
                Some(ref range) => lines.push(WebVTTCue::from(range, line)?),
            }
        }
        Ok(Self(lines))
    }
    pub fn sort(&mut self) {
        self.0.sort_by_key(|l| l.0.0);
    }
    pub fn set_speaker_for_all_lines(&mut self, speaker: &str) {
        for l in self.0.iter_mut() {
            l.1 = Some(speaker.to_owned());
        }
    }
    pub fn merge_with(&mut self, other: WebVTT) {
        self.0.extend(other.0);
        self.sort();
    }
}
impl Display for WebVTT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WEBVTT\n")?;
        for l in self.0.iter() {
            Display::fmt(&l, f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebVTTCue (Timerange, Option<String>, String);
impl WebVTTCue {
    pub fn from(range: &Timerange, string: &str) -> Result<Self, WebVTTError> {
        Ok(Self(range.to_owned(), None, string.to_owned()))
    }
}
impl Display for WebVTTCue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n{}", self.0)?;
        if let Some(speaker) = &self.1 {
            write!(f, "<v {speaker}>")?;
        }
        writeln!(f, "{}", self.2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Timerange (Timestamp, Timestamp);
impl Timerange {
    pub fn from(string: &str) -> Result<Self, WebVTTError> {
        let mut elements = string.split(' ');
        let start = elements.next().ok_or(WebVTTError::Parsing(
            "a starting time".to_owned(),
            "".to_owned()
        ))?.to_owned();
        let start = Timestamp::from(&start)?;

        let sep = elements.next().unwrap_or("");
        if sep != "-->" {
            return Err(WebVTTError::Parsing(
                "-->".to_owned(),
                sep.to_owned()
            ));
        }

        let end = elements.next().ok_or(WebVTTError::Parsing(
            "a end time".to_owned(),
            "".to_owned()
        ))?.to_owned();
        let end = Timestamp::from(&end)?;

        Ok(Self(start, end))
    }
}
impl Display for Timerange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} --> {}", self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Timestamp(std::time::Duration);
impl Timestamp {
    pub fn from(string: &str) -> Result<Self, WebVTTError> {
        let elements: Vec<_> = string.split(':').rev().collect();
        if elements.is_empty() {
            return Err(WebVTTError::Parsing("a timestamp".to_owned(), string.to_owned()));
        }

        let secs: f32 = elements[0]
            .parse()
            .map_err(|_| WebVTTError::Parsing("a decimal number".to_owned(), elements[0].to_owned()))?;
        let mut duration = Duration::from_secs_f32(secs);

        for (i, el) in elements.iter().enumerate().skip(1) {
            let el: u64 = el.parse()
                .map_err(|_| WebVTTError::Parsing("a number".to_owned(), el.to_string()))?;
            duration += Duration::from_secs(u64::pow(60, i as u32) * el);
        }

        Ok(Self(duration))
    }
}
impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let secs = self.0.as_secs();
        write!(f, "{:02}:{:02}:{:02}.{:03}", secs / (60 * 60), secs / 60 % 60, secs % 60, self.0.subsec_millis())
    }
}
