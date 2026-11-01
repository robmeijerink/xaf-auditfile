# Security policy

## Supported versions

Only the latest 0.1.x release receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through GitHub:
[Report a vulnerability](https://github.com/robmeijerink/xaf-auditfile/security/advisories/new).

Include the version of the crate and, if you can, a file or a few lines of code that reproduce the
problem. Do not send real accounting data; a made-up file that shows the problem is enough.

## What is in scope

This crate reads files from third parties, so these are worth reporting:

- a panic, stack overflow or abort on crafted input;
- memory or CPU use that is out of proportion to the size of the input;
- entity expansion, or reading of files or URLs that the input points to;
- a result that silently differs from the content of the file, such as a truncated file that is
  read as a complete one.

## Known behaviour that is not a vulnerability

There is no limit on the size of the input, and memory use grows with the number of transaction
lines when a file is read whole. The README describes this under Limitations, together with the
streaming interface for very large files. Limit the size of files you do not control before
passing them to the crate.
