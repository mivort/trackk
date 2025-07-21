# Report formatting

Custom reports are defined by two parts: config entry and Jinja2-like template.

There are several helper methods and variables which simplify the formatting
and allow to adjust for different terminal sizes.

## Coloring

Named colors can be accessed using variables in `c` namespace: `{{
c.desc_pending }}`. Here are some built-in named color categories:

* `date_due`
* `date_end`
* `date_overdue`
* `date_when`
* `desc_completed`
* `desc_deleted`
* `desc_deleted`
* `desc_pending`
* `divider`

Own named color entries can be defined in `colors` section of configuration.

Functions:
* `fg(color: number)`
* `bg(color: number)`

Variables:
* `reset`
* `bold`
* `italic`
* `underline`

It's possible to use any other ANSI control sequence with Jinja string escape
syntax: `{{ "\u001b[1m" }}`.

## Layout

Filters:
* `.. | width`: screen width of the output as number of columns.
* `.. | trunc(width: number [, end: string])`: truncate the string to the
  desired screen width, optionally adding a symbol at the end.

Functions:
* `fill(content: string, width: number)`

## Formatting

Filters:
* `firstline`: extract the first line for multi-line value. This can be used to
  extract entry title from the entry description.
* `lpad(filler: string)`.
* `rpad(filler: string)`.
