# Trackit

Command-line issue tracker which provides the synchronization and versioning of
its DB using Git capabilities.

## How it works

Trackit stores the data as a series of JSON files (grouped by creation date).
Upon pushing and pulling it uses the custom merge driver which combines the
data with priority for entries with higher time stamp value.

To provide the fast querying the binary index is calculated based on file
modify time.

## Requirements

* Git
