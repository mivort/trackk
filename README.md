# Trackk

Command-line task and issue tracker which provides the synchronization and
versioning of its DB using Git capabilities.

Inspired by [Taskwarrior][1] and [dstask][2].

## Features

* **Git-friendly storage format.** Full versioning and multi-device
  synchronization capabilities. Data is stored as JSON, while custom JSON 3-way
  merge driver auto-resolves conflicts on synchronization.
* **Powerful input and filtering DSL.** [Built-in expression
  syntax](docs/EXPRESSIONS.md) allows to enter dates using natural wording
  (`tomorrow at 14:00`), or perform context filtering on tasks (`tag:home and
  status:started`).
* **Define named queries:** in addition to IDs and filters, access task by
  convenient shortcuts (`recent~1`, `overdue^^` etc.).
* **CLI argument macros:** regex-based user-defined argument expansion rules to
  customize the input syntax.
* Recurrent tasks which use same date input syntax, allowing to use flexible
  re-occurrence rules (`monday at 7:00am`) and enabling the usage as habit
  tracker.
* Highly-customizable reporting using [Minijinja][3] [template
  syntax](docs/FORMATTING.md) with helper methods for screen-size dependent
  output, similar to PS1 customization in shells.
* Ability to perform multiple queries in customizable reports, with adjustable
  headers and grouping.
* User-defined formula for task urgency with option to override urgency.

## Usage

TODO

## How it works

Trackk stores the data as a series of JSON files (grouped by creation date).
Upon pushing and pulling it uses the custom merge driver which combines the
data with priority for entries with higher time stamp value.

To provide the fast querying of active entries the index is calculated based on
entry status.

## Requirements

* Git *(optional)* - for task synchronization between devices/users.

## Alternatives

* [Taskwarrior](https://github.com/GothenburgBitFactory/taskwarrior): the main
  inspiration. Before version 3.0, Taskwarrior stored its date as plain text
  files, and after that it switched to SQLite. Trackk stores its data as set of
  JSON files and provides means to preserve the change history with Git.
* [dstask](https://github.com/naggie/dstask): the inspiration for Git repo
  storage philosophy. Trackk similarly stores tasks as set of JSON files, but
  several tasks are grouped together in buckets, and custom Git merge driver
  prevents merge conflicts.

---
[1]: https://github.com/GothenburgBitFactory/taskwarrior
[2]: https://github.com/naggie/dstask
[3]: https://docs.rs/minijinja/2.10.2/
