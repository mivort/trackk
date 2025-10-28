# Trackk

Command-line task, notes, event and project tracker which provides Git-based
conflict-free synchronization and versioning of its plain-text storage.

Inspired by [Taskwarrior][1] and [dstask][2].

## Features

* **Git-friendly storage format.** Full versioning and multi-device
  synchronization capabilities. Data is stored as set of JSON files, while
  custom JSON 3-way merge driver auto-resolves conflicts on synchronization.
* **Date calculator DSL for natural input and filtering.** [Built-in expression
  syntax][6] allows to enter dates using natural wording (`tomorrow at
  10:00am`), resolve math expressions (`1y-30d+15h`) or perform context
  filtering on tasks (`tag:home and status:started`).
* **Named queries:** in addition to IDs and custom filters, it's possible to
  access task entries using built-in and user-defined named queries
  (`recent~1`, `overdue~0..3` etc.).
* **CLI argument macros:** regex-based user-defined arguments [expansion
  rules][7] to customize the input syntax.
* **Flexible recurrent tasks** which also use date calculator syntax, allowing
  to set complex re-occurrence rules (`monday at 7:00am`) and enabling the
  usage as habit tracker.
* **Highly-customizable reporting** using [Minijinja][3]-based [template
  syntax][5] with helper methods for screen-size dependent output, quite
  similar to `PS1` customization in shells.
* **Reports can perform multiple queries**, with adjustable headers and
  grouping rules.
* **Type-safe user defined fields**: custom integers/floats, strings, time
  spans and dates can be attached to entries and be filtered upon.
* **User-defined formula for task urgency** which provides multi-factor
  priority between entries.
* Respects [NO_COLOR][4].

## Usage

Initialize new entry repository:
``` bash
trk init
```
Create new entry (scheduled to be done in 30 minutes from now and tagged as
`mytag`):
``` bash
trk add Create example task +mytag when:30min
```
List available entries (`next` is the default report type):
``` bash
trk next
```
Modify first entry parameters - add tag `tagtwo` and remove `mytag`:
``` bash
trk 1 mod +tagtwo -mytag
```
Edit first entry with default editor:
``` bash
trk 1 edit
```
Mark entry as complete:
``` bash
trk 1 done
```

## How it works

Trackk stores the data as a series of JSON files (grouped by creation date). By
default, Git repository is created in directory where entries are stored to
provide change history preservation.

When `sync` command is used, the custom merge driver is called which combines
the data using 3-way merge prioritizing changes with higher time stamp value.
There's no ambiguity in merging, it always gets auto-resolved.

To provide the fast querying of active entries the index file is created based
on entry status.

## Requirements

* Git *(optional)* - provides change history preservation and task
  synchronization between devices/users.

## Alternatives

* [Taskwarrior][1]: the main inspiration. Before version 3.0, Taskwarrior
  stored its date as plain text files and used custom solution for sync. With
  3.0+, TW switched to SQLite with custom sync as well. Trackk stores its data
  as set of pretty-printed JSON files and provides means to fully preserve the
  change history with Git.
* [dstask][2]: the inspiration for Git repo storage philosophy. Trackk
  similarly stores tasks as set of JSON files, but several tasks are grouped
  together in buckets reducing burden on the filesystem with large amount of
  entries (100k+), and custom Git merge driver auto-resolves any merge
  conflicts.

---
[1]: https://github.com/GothenburgBitFactory/taskwarrior
[2]: https://github.com/naggie/dstask
[3]: https://docs.rs/minijinja/2.10.2/
[4]: https://no-color.org/
[5]: docs/FORMATTING.md
[6]: docs/DATE_CALC.md
[7]: docs/MACROS.md
