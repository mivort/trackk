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

        if let Some(new_context) = match_context(&arg) {
            context = new_context;
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

    match config.expansion_style {
        ExpansionStyle::Taskwarrior => {
            index[CmdContext::Root as usize].push((
                Regex::new("^log$")?,
                vec!["add".into(), "--status=completed".into()],
            ));

            index[CmdContext::Root as usize]
                .push((Regex::new("^all$")?, vec!["list".into(), "all".into()]));
            index[CmdContext::Root as usize].push((Regex::new("^ls$")?, vec!["list".into()]));

            index[CmdContext::Root as usize].push((
                Regex::new("^done$")?,
                vec!["mod".into(), "--status=completed".into()],
            ));
            index[CmdContext::Root as usize].push((
                Regex::new("^start$")?,
                vec!["mod".into(), "--status=started".into()],
            ));
            index[CmdContext::Root as usize].push((
                Regex::new("^stop$")?,
                vec!["mod".into(), "--status=pending".into()],
            ));
            index[CmdContext::Root as usize].push((
                Regex::new("^(rm|delete)$")?,
                vec!["mod".into(), "--status=deleted".into()],
            ));

            index[CmdContext::Root as usize].push((
                Regex::new("^\\+(.+)")?,
                vec!["--filter".into(), "tag:$1".into()],
            ));
            index[CmdContext::Root as usize].push((
                Regex::new("^-([^-].+)")?,
                vec!["--filter".into(), "!tag:$1".into()],
            ));

            index[CmdContext::Add as usize]
                .push((Regex::new("^\\+(.+)")?, vec!["--tag".into(), "$1".into()]));

            index[CmdContext::Mod as usize]
                .push((Regex::new("^\\+(.+)")?, vec!["--tag".into(), "$1".into()]));
            index[CmdContext::Mod as usize].push((
                Regex::new("^-([^-].+)")?,
                vec!["--tag".into(), "-$1".into()],
            ));
            index[CmdContext::Mod as usize].push((
                Regex::new("^status:(.+)")?,
                vec!["--status".into(), "$1".into()],
            ));
        }
        ExpansionStyle::None => {}
    }

    Ok(index)
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
    Edit: "edit";
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
