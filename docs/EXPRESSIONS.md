# Date inputs, filtering and urgency formula expresssions

Most of date inputs are done in experssion format with support for math and
values relative to the current date/time, similar to how it's done in
[Taskwarrior][1]. So instead of entering the full date in `YYYY-MM-DD` format,
it's possible to enter it as relative expressions: `1d` will add 24 hours to
the current date, `monday` will pick the next closest Monday, `2nd` will pick
the second day of the closest month.

Recurrent tasks store the expression as parameter, and when the recurring task
is complete, it gets copied with the expression re-evaluated. That way it's
possible to define more complex repeating patterns.

Same expression syntax is used for filtering. In case of filters, the
expression must produce either `true` or `false` to include/exclude the entry
from results.

This document provides the description of syntax available in the expressions.

## Operators

* Math: `+`, `-`, `*`, `/`, `%`. Numeric values can be added to dates
* Boolean: `&&`, `||`, `!`. It's also possible to use `and`, `or` and `not`.
* Comparison: `>`, `>=`, `<`, `<=`, `==`.
* Fuzzy comparison: `:`. Allows to check if string contains another string, or
  if issue includes the tag. Examples:
  * `title:example` (`true` if issue's title contains `'example'`).
  * `tag:mytag` (`true` if issue has `mytag` tag).
* Built-in functions: `sqrt`, `ln`.
* 'At' operator: `@` (`at`). Provides means to override time part of the date.
  Example: `monday at 17:00`

## Literals

## Absolute date input

## Relative inputs and aliases

---
[1]: https://taskwarrior.org/docs/dates/
