# Report formatting

Custom reports are defined by two parts: config entry and Jinja2-like template.

There are several helper methods and variables which simplify the formatting
and allow to adjust for different terminal sizes.

## Coloring

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
