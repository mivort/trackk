# Trackit

Command-line task and issue tracker which provides the synchronization and
versioning of its DB using Git capabilities.

Inspired by [Taskwarrior](https://github.com/GothenburgBitFactory/taskwarrior)
and [dstask](https://github.com/naggie/dstask).

## Features

* VCS-friendly storage format and multi-device syncrhonization capabilities.
  Data is stored as JSON files and by default syncronized using Git. Custom
  JSON merge driver prevents conflicts on syncrhonization.
* Dates input and filtering query syntax which allows to enter dates using
  natural syntax (`tomorrow at 14:00`), or perform context filtering on tasks
  (`tag:home and status:started`).

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
