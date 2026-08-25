# ADR 0003: Conversation attachment storage

- Status: Accepted for the current product scale
- Date: 2026-08-24

## Context

Incident responders need to attach bounded runbooks, logs and screenshots to
incident notes and direct messages. The attachment must inherit the exact
authorization and deletion boundary of its parent conversation item. The
current deployment already relies on PostgreSQL backup and restore, while no
private object-storage service or signed-download boundary exists.

## Decision

Store attachment bytes in PostgreSQL `bytea`, beside immutable metadata and a
foreign key that cascades from the parent timeline entry or private message.
Persist the parent and attachments in one transaction. List endpoints return
metadata and `octet_length(content)` only; bytes are loaded exclusively by a
membership- or participant-gated download query.

The server permits at most four files, 5 MiB per file and 10 MiB combined per
message. HTTP request bodies allow 14 MiB to accommodate base64 and JSON
overhead. Files are always downloaded with `Content-Disposition: attachment`,
`X-Content-Type-Options: nosniff` and `Cache-Control: private, no-store`.
`application/octet-stream` means an unknown, download-only payload; the
allowlist is not a claim that a file is safe to execute or render inline.

## Consequences

- Authorization, deletion and backup remain atomic and easy to demonstrate.
- Primary database size, replicas and backups include attachment bytes.
- There is currently no Team quota or retention policy; operational deployment
  must monitor database growth.
- Before sustained production usage, introduce Team quotas and retention. Move
  bytes to private object storage when observed volume justifies it, retaining
  metadata and authorization in PostgreSQL and using short-lived downloads.
