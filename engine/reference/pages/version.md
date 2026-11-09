---
title: version
summary: The version of the xlsx-graft engine the server was built from.
tools: [version]
---

# version

Returns the version of the xlsx-graft engine the server was built from.
The server's `initialize` result names the same version in
`serverInfo.version`.

The tool takes no arguments.

## Example call

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"version","arguments":{}}}
```

## Result

`structuredContent` is an object with one key:

| Key | Type | Meaning |
| --- | --- | --- |
| `version` | string | The engine version, such as `0.0.0` |

`content` holds the same object as JSON text, for a client that reads
only `content`. `isError` is `false`.

```json
{"content":[{"type":"text","text":"{\"version\":\"0.0.0\"}"}],"isError":false,"structuredContent":{"version":"0.0.0"}}
```
