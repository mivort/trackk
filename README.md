# Trackit

Command-line task and issue tracker which provides the synchronization and
versioning of its DB using Git capabilities.

Inspired by [Taskwarrior][1] and [dstask][2].

## Features

* VCS-friendly storage format and multi-device synchronization capabilities.
  Data is stored as JSON files and by default synchronized using Git. Custom
  JSON merge driver prevents conflicts on synchronization.
* Dates input and filtering query syntax which allows to enter dates using
  natural syntax (`tomorrow at 14:00`), or perform context filtering on tasks
  (`tag:home and status:started`).
* Recurrent tasks which use same date input syntax, allowing to use flexible
  re-occurrence rules (`monday at 7:00am`) and enabling the usage as habit
  tracker.
* Fully-customizable output format using [Minijinja][3] template syntax.

## Usage

TODO

## How it works

Trackit stores the data as a series of JSON files (grouped by creation date).
Upon pushing and pulling it uses the custom merge driver which combines the
data with priority for entries with higher time stamp value.

To provide the fast querying of active entries the index is calculated based on
entry status.

## Requirements

* Git *(optional)* - for task synchronization between devices/users.

## Alternatives

* [Taskwarrior](https://github.com/GothenburgBitFactory/taskwarrior): the main
  inspiration. Before version 3.0, Taskwarrior stored its date as plain text
  files, and after that it switched to SQLite. Trackit stores its data as set
  of JSON files and provides means to preserve the change history with Git.
* [dstask](https://github.com/naggie/dstask): the inspiration for Git repo
  storage philosophy. Trackit similarly stores tasks as set of JSON files, but
  several tasks are grouped together in buckets, and custom Git merge driver
  prevents merge conflicts.

---
[1]: https://github.com/GothenburgBitFactory/taskwarrior
[2]: https://github.com/naggie/dstask
[3]: https://docs.rs/minijinja/2.10.2/
