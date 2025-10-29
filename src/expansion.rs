use regex::Regex;
use serde_derive::Deserialize;

use crate::config::{Config, ExpansionStyle};
use crate::prelude::*;

/// Perform expansion rules on arguments.
pub fn pre_process_args(config: &Config) -> Result<Vec<String>> {
    let args = std::env::args();
    pre_process(config, args)
}

/// Iterate over arguments and store in output vec.
fn pre_process(config: &Config, mut args: impl Iterator<Item = String>) -> Result<Vec<String>> {
    let mut output = Vec::new();
    let mut context = CmdContext::Root;

    let index = rule_index(config)?;
    output.push(args.next().unwrap());

    'next_arg: for arg in args {
        if let (CmdContext::Root, Some(new_context)) = (context, match_context(&arg)) {
            context = new_context;
            output.push(arg);
            continue;
        }

        let rules = &index[context as usize];

        'next_rule: for (regex, replace) in rules {
            let captures = unwrap_some_or!(regex.captures(&arg), { continue 'next_rule });

            let before = output.len();
            for rep in replace {
                output.push(apply_captures(rep, &captures));
            }

            if context != CmdContext::Root {
                continue 'next_arg;
            }

            for new in &output[before..output.len()] {
                if let Some(new_context) = match_context(new) {
                    context = new_context;
                }
            }

            continue 'next_arg;
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

    for rule in &config.macros {
        let regex = Regex::new(&rule.find)?;
        if rule.contexts.is_empty() {
            index[CmdContext::Root as usize].push((regex, rule.replace.clone()));
            continue;
        }
        for ctx in &rule.contexts {
            index[*ctx as usize].push((regex.clone(), rule.replace.clone()));
        }
    }

    let style = config.macros_style.as_ref();
    match style {
        Some(ExpansionStyle::Taskwarrior) | None => expansions_tw(&mut index)?,
        Some(ExpansionStyle::None) => {}
    }

    Ok(index)
}

/// Append Taskwarrior-style expansions.
fn expansions_tw(idx: &mut RuleIndex) -> Result<()> {
    let rg = Regex::new;

    let root = CmdContext::Root as usize;
    let list = CmdContext::List as usize;
    let add = CmdContext::Add as usize;
    let r#mod = CmdContext::Mod as usize;

    idx[root].push((
        rg("^log$")?,
        vec!["add".into(), "--status=completed".into()],
    ));

    idx[root].push((rg("^all$")?, vec!["list".into(), "all".into()]));
    idx[root].push((rg("^ls$")?, vec!["list".into()]));
    idx[root].push((rg("^recent$")?, vec!["list".into(), "recent".into()]));

    idx[root].push((rg("^dup$")?, vec!["add".into(), "--copy".into()]));

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

    let mut filter_rules = |ctx: usize| -> Result<_, regex::Error> {
        // Ignore verbosity flags
        idx[ctx].push((rg(r"^(-v+)")?, vec!["$1".into()]));

        // Tags with '+'
        idx[ctx].push((rg(r"^\+(\w+)")?, vec!["--tag=$1".into()]));

        // Tags with '-' - potential conflict with short options
        idx[ctx].push((rg(r"^-([^\d\W]\w+)")?, vec!["--tag=-$1".into()]));

        idx[ctx].push((rg(r"^==(.*)")?, vec!["--filter=$1".into()]));
        idx[ctx].push((rg(r"^=(.*)")?, vec!["--title=$1".into()]));

        idx[ctx].push((rg(r"^([0-9]+)$")?, vec!["--id=$1".into()]));
        idx[ctx].push((rg(r"^([0-9a-f]{4,8}.*)")?, vec!["--id=$1".into()]));

        idx[ctx].push((rg(r"^(\w+)~$")?, vec!["--query=$1".into()]));
        idx[ctx].push((
            rg(r"^(\w+)~(\d+)$")?,
            vec!["--query=$1".into(), "--skip=$2".into(), "--limit=1".into()],
        ));

        Ok(())
    };

    filter_rules(root)?;
    filter_rules(list)?;

    let mut mod_rules = |ctx: usize| -> Result<_, regex::Error> {
        idx[ctx].push((rg(r"^-([^-].+)")?, vec!["--tag".into(), "-$1".into()]));
        idx[ctx].push((rg(r"^\+(.+)")?, vec!["--tag".into(), "$1".into()]));
        idx[ctx].push((
            rg(r"^(due|when|end|repeat|status):(.*)")?,
            vec!["--$1=$2".into()],
        ));
        Ok(())
    };

    mod_rules(add)?;
    mod_rules(r#mod)?;

    Ok(())
}

/// Macro to create command context enum and corresponding matcher method.
macro_rules! cmd_context {
    ($($id:ident: $str:literal;)*) => {
        /// Command context enum values.
        #[derive(Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
        #[cfg_attr(test, derive(Debug))]
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
    Init: "init";
    Add: "add";
    Mod: "mod";
    List: "list";
    Count: "count";
    Info: "info";
    Config: "config";
    Sync: "sync";
    Calc: "calc";
    Import: "import";
    Export: "export";
}

impl Default for CmdContext {
    #[inline(always)]
    fn default() -> Self {
        Self::Root
    }
}

/// Matcher table for different command contexts.
type RuleIndex = [Vec<(Regex, Vec<String>)>; cmd_contexts()];

#[test]
fn try_expand() {
    let config = Config::default();
    let cmd: Vec<String> = vec!["add".into(), "+test".into(), "test".into(), "entry".into()];
    let cmp: Vec<String> = vec![
        "add".into(),
        "--tag=test".into(),
        "test".into(),
        "entry".into(),
    ];
    assert_eq!(pre_process(&config, cmd.into_iter()).unwrap(), cmp);
}
