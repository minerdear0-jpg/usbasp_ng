//! Explanation Layer — communication only.
//!
//! Does not create evidence, does not change verdicts.
//! Maps Analysis v1 findings → human WHAT / WHY / WHY NOT / CERTAINTY.

mod rules;

pub use rules::explain;

use serde::Serialize;

pub const EXPLANATION_SCHEMA: &str = "explanation-v1";

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Tone {
    Ok,
    Warn,
    Fail,
    Info,
}

/// Human-facing card. Derived from analysis; never a source of truth.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Explanation {
    pub schema: &'static str,
    pub tone: Tone,
    /// One-line executive: "PROGRAMMING SUCCEEDED"
    pub headline: String,
    /// Companion: "1 ANOMALY DETECTED" / "ROOT CAUSE NOT CONFIRMED"
    pub subhead: String,
    pub why: Vec<String>,
    pub why_warning: Vec<String>,
    pub why_not_failure: Vec<String>,
    pub why_failed: Vec<String>,
    pub what_we_know: Vec<String>,
    pub not_proven: Vec<String>,
    pub certainty: Vec<String>,
}

impl Explanation {
    pub fn emit_human(&self) {
        println!("DIAGNOSIS  schema={}", self.schema);
        println!();
        let mark = match self.tone {
            Tone::Ok => "✓",
            Tone::Warn => "⚠",
            Tone::Fail => "✗",
            Tone::Info => "·",
        };
        println!("{mark} {}", self.headline);
        if !self.subhead.is_empty() {
            println!("  {}", self.subhead);
        }
        println!();
        if !self.why.is_empty() {
            println!("WHY:");
            for s in &self.why {
                println!("  • {s}");
            }
            println!();
        }
        if !self.why_failed.is_empty() {
            println!("WHY IT FAILED:");
            for s in &self.why_failed {
                println!("  • {s}");
            }
            println!();
        }
        if !self.why_warning.is_empty() {
            println!("WHY THERE IS A WARNING:");
            for s in &self.why_warning {
                println!("  • {s}");
            }
            println!();
        }
        if !self.why_not_failure.is_empty() {
            println!("WHY THIS IS NOT A FAILURE:");
            for s in &self.why_not_failure {
                println!("  • {s}");
            }
            println!();
        }
        if !self.what_we_know.is_empty() {
            println!("WHAT WE KNOW:");
            for s in &self.what_we_know {
                println!("  • {s}");
            }
            println!();
        }
        if !self.not_proven.is_empty() {
            println!("WHAT IS NOT PROVEN:");
            for s in &self.not_proven {
                println!("  • {s}");
            }
            println!();
        }
        if !self.certainty.is_empty() {
            println!("CONFIDENCE:");
            for s in &self.certainty {
                println!("  • {s}");
            }
            println!();
        }
    }
}
