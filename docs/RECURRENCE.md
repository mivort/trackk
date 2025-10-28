# Repeating entries

Each entry can be set for automatic re-occurence upon the completion (once
entry status changes either to `complete` (or the one in the list defined by
`values.repeat_status` [option](CONFIG.md)).

Re-occurence value is represented by [date calc](DATE_CALC.md) expression: when
expression produces date or time span, entry is duplicated, and either `when`
or `due` is assigned to the result of the expression (`due` date is selected if
it was defined for the entry).

When re-occurence expression produces a `false` boolean value, entry no longer
gets duplicated on the completion.

## Setting entry to repeat

Existing entry can be updated with repeat rule:
``` bash
trk 15 mod --repeat=3d
```
When entry will be marked as complete, it would be duplicated with `when` field
assigned for three days from now. Completing the task:
``` bash
trk 15 done
```
...will produce the following output:
```
● Entry [...] updated
●  status: pending -> completed
● Task is set to repeat in 3 days
● Updated 1 entry(es)
● New active entry ([...]) added, ID: [...]
```
