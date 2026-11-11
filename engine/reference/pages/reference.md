---
title: Reading the reference
summary: How to list the reference pages and fetch one by its URI.
tools: [list_reference, get_reference]
---

# Reading the reference

The reference pages are compiled into the server. A page whose `tools` is not empty is markdown
about the tools named in that page's `tools` in the `list_reference`
result: how to call each one, with an example call, and what the call
returns.

A page whose `tools` is empty explains diagnostic codes instead of tools:
it has one `## <code>` section per code, and its `summary` says which
codes it holds (a name ending in `*` stands for every code that starts with it).

Read the page of a tool before calling it. Each tool's description in
`tools/list` ends with the `uri` of its page.

## list_reference

Lists every page. The tool takes no arguments.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_reference","arguments":{}}}
```

`structuredContent` has two keys:

| Key | Type | Meaning |
| --- | --- | --- |
| `pages` | array | One object per page: `uri`, `title`, `summary`, and `tools`, the names of the tools the page documents |
| `howToRead` | string | How to use the pages |

`content` holds the same object as JSON text, for a client that reads
only `content`. `isError` is `false`.

## get_reference

Returns one page's markdown. The one argument, `uri`, is a page's `uri`
from `list_reference` or from the end of a tool's description in
`tools/list`: `xlsx-graft://reference/<page>`.

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_reference","arguments":{"uri":"xlsx-graft://reference/version"}}}
```

The result's `content` is one text part holding the page's markdown: the
page without its front matter, starting with its `# ` heading. `isError`
is `false`, and the result has no `structuredContent`.

A `uri` that names no page is a tool error: the result has
`isError: true` and says why in its text, so the client can list the
pages and try again. A call without a string `uri` is a JSON-RPC error
(`-32602`, invalid params).
