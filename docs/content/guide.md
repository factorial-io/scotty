# A guide to Scotty


## What is Scotty?

Scotty is a so-called *Micro-Platform-as-a-Service*. It allows you to **manage** all your
docker-compose-based apps with **a simple UI and CLI**. Scotty provides a simple
REST API so you can interact with your apps. It takes care of the lifetime
of your apps and includes **scope-based authorization** to control access to applications
and operations. It adds basic auth to prevent unauthorized access if needed and
instructs robots to not index your apps.

The primary use-case is to **host ephemeral review apps** for your projects. It
should be relatively easy to integrate Scotty into existing workflows,
e.g. with CI/CD pipelines or run it on a case-by-case basis from your local.

Scotty is **a very simple orchestrator** for your docker-compose-based apps. The UI
is designed to be simple and easy to use, so people other than devs can restart
a stopped app or check the status of an app.

If you **can write a docker-compose-based app** which runs on your local, then
Scotty **should be able to "beam"** that app to a server **and run it there**, with a
nice domain name, so you can reach it from the internet.

## What is Scotty not?

It's not a solution for production-grade deployments. It's not a replacement for
tools like Nomad, Kubernetes or OpenShift. If you need fine-grained control on
how your apps are executed, Scotty might not be the right tool for you.
It does not orchestrate your apps on a cluster of machines.
It's a single-node solution with optional scope-based access control and no support for
scaling your apps.

It is also not a replacement for tools like Dockyard or Portainer. However,
Scotty does provide basic debugging capabilities including real-time log viewing
(via CLI and web UI) and interactive shell access to containers via the CLI.

Scotty wants to be a simple solution for a simple use-case.

## Creating an app from the dashboard

Apps based on a **templated blueprint** (such as `static-html` or `atelier`) can be
created without the CLI. On the apps list, click **Create app**; the button shows
up when at least one blueprint you may use is templated. Blueprints that only
validate a compose file you bring yourself stay CLI-only.

1. **Pick a blueprint.** The form follows it: one field per parameter, an image
   choice when the blueprint offers more than one, and a file upload when it
   takes files. Generated and derived parameters are filled in by Scotty and not shown.
2. **Name the app** and choose the scope, the time to live (`12h`, `7d` or `forever`)
   and optional basic auth. The name is normalised as on the server (`Report X`
   becomes `report-x`); if an app with that name exists, Scotty asks before updating it.
3. **Upload the files** as one zip file or one folder. Both are read in your browser.

The uploaded folder's name, or a zip's single top-level folder, is dropped. When the
blueprint accepts exactly one directory (`html/**` for `static-html`), the files are
placed into it: a folder `my-site` with `index.html` and `img/a.png` becomes
`html/index.html` and `html/img/a.png`. An upload that already contains `html/` is
used as it is. Other blueprints get the paths unchanged.

Before uploading, the form lists every file the blueprint would not accept and
blocks the upload until you remove them. Compose files (`docker-compose.yml`,
`compose.override.yaml`, ...) and env files (`.env`, `.env.local`, ...) are always
refused, in any directory. `.DS_Store` files and `__MACOSX/` folders from Finder-made
zips are dropped silently.
A blueprint may also require files, in which case the upload is mandatory and the
form lists what is still missing; `static-html` requires `html/index.html`. After submitting, the dashboard shows the create task.

## Want to start?

Check out the following sections:

* [First Steps Guide](first-steps.md) to get up and running with Scotty
* [Installation Guide](installation.md) for more detailed installation options
* [Configuration Guide](configuration.md) to learn about all available settings
* [Authorization System](authorization.md) for scope-based access control
* [Architecture Documentation](architecture.md) to understand how Scotty works
* [CLI Documentation](cli.md) for all available commands, including logs, shell access, and troubleshooting
