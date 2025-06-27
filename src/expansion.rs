use regex::Regex;
use serde_derive::Deserialize;

use crate::config::{Config, ExpansionStyle};
use crate::prelude::*;

/// Perform expansion rules on arguments.
pub fn pre_process_args(config: &Config) -> Result<Vec<String>> {
    let mut args = std::env::args();
    let mut output = Vec::new();
    let mut context = CmdContext::Root;

    let index = rule_index(config)?;
    output.push(args.next().unwrap());

    'arg: for arg in args {
        if let Some(new_context) = match_context(&arg) {
            context = new_context;
            output.push(arg);
            continue;
        }

        let rules = &index[context as usize];

        for (regex, replace) in rules {
            let captures = unwrap_some_or!(regex.captures(&arg), { continue });

            let before = output.len();
            for rep in replace {
                output.push(apply_captures(rep, &captures));
            }

            for new in &output[before..output.len()] {
                if let Some(new_context) = match_context(new) {
                    context = new_context;
                }
            }

            continue 'arg;
        }
        output.push(arg);
    }

    Ok(output)
}

/// Iterate over symbols and replace the captures.
fn apply_captures(value: &str, captures: &regex::Captures) -> String {
    let mut chars = value.chars();
    let mut output = String::new();
    while let Some(char) = chars.next() {
        if char != '$' {
            output.push(char);
            continue;
        }
        let next = unwrap_some_or!(chars.next(), {
            output.push(char);
            continue;
        });
        match next {
            '$' => {
                output.push('$');
            }
            '0'..='9' => output.push_str(
                captures
                    .get(next as usize - '0' as usize)
                    .map(|c| c.as_str())
                    .unwrap_or_default(),
            ),
            _ => {}
        }
    }

    output
}

/// Produce index table with parsed regex expansion rules.
fn rule_index(config: &Config) -> Result<RuleIndex> {
    let mut index: RuleIndex = [const { Vec::new() }; cmd_contexts()];

    for rule in &config.expansions {
        let regex = Regex::new(&rule.expr)?;
        index[rule.context as usize].push((regex, rule.replace.clone()));
    }

    match config
        .expansion_style
        .as_ref()
        .unwrap_or(&Default::default())
    {
        ExpansionStyle::Taskwarrior => expansions_tw(&mut index)?,
        ExpansionStyle::None => {}
    }

    Ok(index)
}

/// Append Taskwarrior-style expansions.
fn expansions_tw(idx: &mut RuleIndex) -> Result<()> {
    let rg = Regex::new;

    let root = CmdContext::Root as usize;
    idx[root].push((
        rg("^log$")?,
        vec!["add".into(), "--status=completed".into()],
    ));

    idx[root].push((rg("^all$")?, vec!["list".into(), "all".into()]));
    idx[root].push((rg("^ls$")?, vec!["list".into()]));
    idx[root].push((rg("^recent$")?, vec!["list".into(), "recent".into()]));

    idx[root].push((rg("^edit$")?, vec!["mod".into(), "--edit".into()]));
    idx[root].push((
        rg("^done$")?,
        vec!["mod".into(), "--status=completed".into()],
    ));
    idx[root].push((
        rg("^start$")?,
        vec!["mod".into(), "--status=started".into()],
    ));
    idx[root].push((
        rg("^(stop|reset|undelete)$")?,
        vec!["mod".into(), "--status=pending".into()],
    ));
    idx[root].push((
        rg("^(rm|delete)$")?,
        vec!["mod".into(), "--status=deleted".into()],
    ));

    idx[root].push((rg(r"^([0-9]+)$")?, vec!["--id=$1".into()]));
    idx[root].push((rg(r"^([0-9a-f]{4,8}.*)")?, vec!["--id=$1".into()]));

    idx[root].push((rg(r"^\+(.+)")?, vec!["--filter".into(), "tag:$1".into()]));
    idx[root].push((
        rg(r"^-([^-].+)")?,
        vec!["--filter".into(), "!tag:$1".into()],
    ));

    let add = CmdContext::Add as usize;
    idx[add].push((rg(r"^\+(.+)")?, vec!["--tag".into(), "$1".into()]));
    idx[add].push((
        rg(r"^(due|when|end|repeat|status):(.*)")?,
        vec!["--$1=$2".into()],
    ));

    let r#mod = CmdContext::Mod as usize;
    idx[r#mod].push((rg(r"^\+(.+)")?, vec!["--tag".into(), "$1".into()]));
    idx[r#mod].push((rg(r"^-([^-].+)")?, vec!["--tag".into(), "-$1".into()]));
    idx[r#mod].push((
        rg(r"^(due|when|end|repeat|status):(.*)")?,
        vec!["--$1=$2".into()],
    ));

    Ok(())
}

/// Macro to create command context enum and corresponding matcher method.
macro_rules! cmd_context {
    ($($id:ident: $str:literal;)*) => {
        /// Command context enum values.
        #[derive(Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum CmdContext {
            $(
                #[serde(rename = $str)]
                $id,
            )*
        }

        /// Check if current argument should change the context.
        fn match_context(arg: &str) -> Option<CmdContext> {
            match arg {
                $( $str => Some(CmdContext::$id), )*
                _ => None,
            }
        }

        /// Provide constant number of defined command contexts.
        const fn cmd_contexts() -> usize {
            let mut count = 0;
            $(
                let _ = $str;
                count += 1;
            )*
            count
        }
    }
}

cmd_context! {
    Root: "root";
    Add: "add";
    Mod: "mod";
    List: "list";
    Count: "count";
    Info: "info";
    Config: "config";
    Sync: "sync";
}

impl Default for CmdContext {
    #[inline(always)]
    fn default() -> Self {
        Self::Root
    }
}

/// Matcher table for different command contexts.
type RuleIndex = [Vec<(Regex, Vec<String>)>; cmd_contexts()];
