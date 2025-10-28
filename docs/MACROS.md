# Comman-line argument macros

Trackk implements a powerful way to customize the CLI input by using
configurable regex-based macro rules. This system is more flexible than
traditional aliases since it allows to perform more complex pattern matching
and produce replacements using the capture group syntax.

There is a built-in Taskwarrior-inspired set of rules (enabled by default)
which provides shortcuts for a common operations.

For example, the following command:
``` bash
trk add +mytag task description when:3h
```
...will expand to Clap-compatible dash-prefixed arguments:
``` bash
trk add --tag=mytag task description --when=3h
```
Default set of rules is enabled by `macros_style` option (`taskwarrior` by
default, built-in rules can be toggled off by setting it to `none`).

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
Rules are processed one by one, the first match wins, so ordering is important
- more specific rules should be placed before the broader ones.
