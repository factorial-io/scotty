# MCP Server

Scotty serves an [MCP](https://modelcontextprotocol.io) endpoint at
`{base_url}/mcp`. LLM clients such as claude.ai and Claude Code use it to list
blueprints, create apps from them (for example to publish an HTML page),
follow the creation task, inspect apps and destroy them.

The endpoint speaks Streamable HTTP without sessions and offers tools only.
Every tool runs as the user who connected, with the same permission checks as
the REST API. A connection never gets more than that user has.

## Tools

| Tool | Scope | Annotation | What it does |
|---|---|---|---|
| `list_blueprints` | `read` | read-only | Templated blueprints you may create from, with their parameter schema, images, payload patterns and the scopes you may create them in |
| `list_apps` | `read` | read-only | Apps you may view, with status and URLs |
| `get_app` | `read` | read-only | One app: status, services, URLs, blueprint, scopes |
| `get_task` | `read` | read-only | A task's state (`Running`, `Finished`, `Failed`) and its last 100 output lines |
| `create_app` | `create` | not destructive | Creates an app from a templated blueprint and returns the task id and the app's URLs |
| `destroy_app` | `destroy` | destructive | Stops and removes an app with its volumes and files |

Tool results never contain environment values, generated blueprint
parameters, basic auth credentials or tokens.

### create_app

`create_app` takes:

| Field | Required | Meaning |
|---|---|---|
| `blueprint` | yes | A templated blueprint from `list_blueprints` |
| `app_name` | no | The app name, slugified (the result returns the final name). Without it Scotty generates `<blueprint>-<10 random characters>`, so the URL can't be guessed |
| `parameters` | no | Values for the blueprint's parameter schema |
| `image` | no | One of the blueprint's images; the first is the default |
| `files` | no | Payload files: `{path, content, encoding}` with `encoding` `utf8` (default) or `base64` |
| `ttl` | no | `<days>d`, `<hours>h` or `forever`; default 7 days |
| `basic_auth` | no | `{user, password}` |
| `scope` | no | The scope to create the app in. The tool's input schema lists the scopes you may create apps in as choices |

Rules:

- Only templated blueprints. Validation-only blueprints and compose files are
  refused.
- `create_app` never replaces an app. An existing name is an error; to
  republish, destroy the app and create it again.
- The decoded files together may not exceed `api.create_app_max_size`.
- Without `scope`, the app goes to your only create scope, otherwise to
  `default`. If you may create in several scopes but not in `default`, pass
  `scope`.
- `ttl` must be at least `1h` or `1d`.
- The call returns at once. Poll `get_task` with the returned task id until
  the state is `Finished` or `Failed`.

> [!WARNING]
> Everything a tool receives or returns passes through the MCP client, its
> transcript and its model provider. That includes the `basic_auth` password
> you pass to `create_app` and the task output `get_task` returns. Scotty
> never adds env values to results, but a compose or build step can print
> them. Use throwaway basic auth credentials, and keep secrets out of
> blueprint output.

Example: Claude publishes a page with blueprint `static-html` and the file
`html/index.html`, polls the task, and gives you the URL, for example
`https://web.static-html-k3v9x0q2mz.artifacts.example.net`.

## Authentication and scopes

`/mcp` accepts two kinds of credentials:

- **MCP tokens** that Scotty issues as an OAuth authorization server
  (`oauth` mode). The user approves the connection on Scotty's consent page and
  can untick scopes there. See
  [MCP connections](oauth-authentication.md#mcp-connections) for the flow,
  the consent page and how to revoke connections.
- **Configured bearer tokens** (`api.bearer_tokens`), in every auth mode. They
  act as their `identifier:<name>` service account. They pass the MCP scope
  check of every tool (`read`, `create`, `destroy`), but reach only the
  Scotty scopes and apps their `identifier:<name>` assignment in the
  [authorization policy](authorization.md) grants.

> [!WARNING]
> A bearer token given to an MCP client can do everything its assignment
> allows in its Scotty scopes, through any tool; unticking MCP scopes on the
> consent page applies to MCP tokens only. Give LLM clients their own token with a narrow
> `identifier:<name>` assignment, not a CI token.

Tool calls count against the authenticated rate limit tier when rate limiting
is enabled.

Session tokens of the web UI and `scottyctl`, and identity provider tokens,
are refused. A request without a valid token gets `401` with
`WWW-Authenticate: Bearer resource_metadata="{base_url}/.well-known/oauth-protected-resource/mcp"`,
which MCP clients use to start the login.

| Scope | Tools |
|---|---|
| `read` | `list_blueprints`, `list_apps`, `get_app`, `get_task` |
| `create` | `create_app` |
| `destroy` | `destroy_app` |
| `offline_access` | none; lets the client refresh its token |

A tool runs only if the token holds its scope **and** the policy grants the
permission. For example, `create_app` needs the `create` scope and `create`
(or `create:<blueprint>`) in the app's scope; `destroy_app` needs the
`destroy` scope and `destroy` on the app. If a scope is missing, the error
names it, so you know what to approve when you reconnect.

## Connect claude.ai

Add a custom connector with the URL `https://scotty.example.com/mcp` and leave
the OAuth client fields empty. claude.ai opens Scotty's consent page; approve
the scopes you want to grant. Details, plan differences and the firewall
rules claude.ai needs are in
[MCP connections](oauth-authentication.md#claude.ai).

## Connect Claude Code

With OAuth (`oauth` mode):

```bash
claude mcp add --transport http scotty https://scotty.example.com/mcp
```

Then run `/mcp` in Claude Code and choose to authenticate. Your browser opens
Scotty's consent page.

With a configured bearer token, for service accounts and `bearer` mode:

```bash
claude mcp add --transport http scotty https://scotty.example.com/mcp \
  --header "Authorization: Bearer $SCOTTY_ACCESS_TOKEN"
```

## Check the endpoint by hand

```bash
curl -s https://scotty.example.com/mcp \
  -H "Authorization: Bearer $SCOTTY_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

The response lists the six tools with their input schemas.
