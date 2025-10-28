# Command-line argument macros

Trackk uses [clap-rs][1] library to perform single and double hyphen argument
parsing which is highly discoverable (`--help` will produce the list of
possible options, both common and subcommand-specific) and unambiguous. While
there's a big consistency benefit in supporting this convention, it leads to
longer commands: `--tag=mytag` vs. Taskwarrior's `+mytag`.

To tackle the input ergonomics issue, Trackk implements a powerful way to
customize the argument parsing by using a configurable regex-based argument
expansion rules. This system is more flexible than traditional aliases since it
allows to perform more complex context-aware pattern matching and produce
replacements using the capture group syntax.

There is a built-in Taskwarrior-inspired set of rules (enabled by default)
which provides shortcuts for a common operations: tag selection (`+mytag` will
enable the tag, `-mytag` disables it), field setters: `when:...`, `due:...`,
`status:...` etc., filtering: `=pattern`.

For example, the following command:
``` bash
trk add +mytag task description when:3h
```
...will expand to Clap-compatible dash-prefixed arguments:
``` bash
trk add --tag=mytag task description --when=3h
```
Default set of rules is controlled by `macros_style` option (`taskwarrior` by
default, `none` will toggle off all built-in macros).

## Custom rules

Custom macros can be added using the `macros` option in `config.json5`:
``` json5
{
  macros: [
    {
      // Match the rule on the specified regex pattern.
      find: "regex-rule-to-find",

      // Value to replace the pattern match. It can reference capture groups
      // via $1, $2 etc. (only numeric references are supported).
      replace: ["replace-with"],

      // One or several subcommands which would use this rule.
      // Run `trk --help` to see the available subcommands.
      // "Root" context gets expanded for arguments placed before the subcommand.
      contexts: ["root", "add", "mod"],
    },
  ],
}
```
Rules are processed one by one, the first match wins, so ordering is important:
more specific rules should be placed before the broader ones. Custom macros are
placed in front of built-in ones.

## Rule debugging

To see the rule expansion, `--verbose` option can be used to output the
produced arguments:
```
$ trk all +mytag --verbose
● Command expanded to: ["trk", "list", "all", "--tag=mytag", "--verbose"]
[...]
```

[1]: https://github.com/clap-rs/clap
