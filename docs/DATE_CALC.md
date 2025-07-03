# Date calc: built-in date calculator

Most of date inputs in **trackk** are done in simple single-expression language
with support for math and values relative to the current date/time, similar to
how it's done in [Taskwarrior][1]. So instead of entering the full date, it's
possible to enter it as mathematical expressions or relative time points:
* `1h + 30min` will add 1 hour and 30 minutes to the current date and time.
* `11:00am` will take the closest 11:00 AM position in calendar: today if it's
  less than 11 currently, or the next day.
* `monday` will pick the next closest Monday.
* `2nd` will pick the second day of the closest month.

The expression format is a DSL with focus on shell friendliness and natural
date input (so, there's a lot of built-in literals such as `today`, `tomorrow`,
`before`, `at` etc., and it's possible to enter dates in ISO 8601 layout
(`YYYY-MM-DD`) without need to add quotes).

Recurrent tasks store the expression as parameter, and when the recurring task
is complete, it gets copied with the expression re-evaluated. That way it's
possible to define more complex repeating patterns. (TODO: add link to separate
task recurrence article).

Same expression syntax is used for entry filtering. In case of filters, the
expression must produce either `true` or `false` to include/exclude the entry
from results.

Date calc syntax follows Python in many cases of operator naming, but provides
C-like expression equivalents (so both `and` and `&&` are valid).

This document provides the reference of syntax available in the date
calculator.

## Literals

**Durations**

Durations are entered as number with a wide array of suffxes supported. Default
unit is seconds. When specified, each suffix provides a multiplier to the
certain amount:

* Minutes: `1m` (or `min`): 60 seconds (`1m == 60s`).
* Hours: `1h` (or `hrs`): 60 minutes (`1h == 60m`).
* Days: `1d`: 24 hours (`1d == 24h`).
* Weeks: `1w`: 7 days (`1w == 7d`).
* Months: `1mo`: 30 days (`1mo == 30d`). Duration prefix is always 30 days, not
  depending on the current date.
* Years: `1y`: 365 days (`365d == 1y`). Same as month, year multiplier is
  always 365, not depending on if the current year is a leap year.

**Dates**

Exact or relative date can be specified as one of the supported formats:

* ISO-8601: `YYYY-MM-DD`.
* TODO: describe more supported formats.

## Operators

* Math: `+`, `-`, `*`, `/`, `%`. Numeric values can be added to dates
* Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`. There are also aliases:
  `before`, `after`, `before_eq`, `after_eq`.
  * Notable difference to poplular languages: comparison operators have unary
    version which compares the date value to the current date: `after
    2000-01-01` will produce `true` if current machine clock has passed Jan 1
    2000.
* Boolean: `and`, `or`, `not`. It's also possible to use `&&`, `||` and `!`.
  * Notable difference to Python: any non-boolean value is considered `true`.
    That allows to use `or` as coalesce operator. Example: `due or tomorrow`:
    will use entry's due date if it's defined, otherwise fallback to
    `tomorrow`.
* Inclusion checks: `in` and `has` (`:` is an alias for `has`). Allows to check
  if string contains another string, or if entry list of tags includes the tag.
  Examples:
  * `title:example` (`true` if entry's title contains `'example'`).
  * `tag:mytag` (`true` if issue has `mytag` tag).
* Time copy operator: `at` (alias: `@`). Provides means to override time part
  of the date. Example: `monday at 17:00` (closest Monday is taken as base
  date, and time is rewritten to `17:00`).
* Python-style ternary operator: `value_a if condition else value_b`. Notable
  difference to Python is that `else` is not required: if `condition` is
  `false` and there's no `else`, the expression will produce `false`.
* Built-in math functions: `sqrt()`, `ln()`, `sig()`.
* Other functions:
  * `len`: string value length in bytes, number of issue tags.
    Example: `len(tag) > 2`.

## Absolute date input

## Relative inputs and aliases

---
[1]: https://taskwarrior.org/docs/dates/
