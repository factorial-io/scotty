# yafbds

## About

**yafbds -- yet another feature based deployment service** is a rust
server providing an api to create, start, stop or destroy a
docker-composed-based application.

The repo contains two applications:

* `yafbds` a rust based http-server providing an API to talk with the
  service and to start, stop and run docker-composed based applications
  The service provides a ui at e.g. `http://localhost:21342/`. the api is documented at `http://localhost:21342/rapidoc`
* `yafbdsctl` a cli application to talk with the service and execute
  commands from your shell

## Installation

Use the provided docker-image for best results. Map the directory with all your docker-composed apps to `/app/apps`.

```shell
docker run -v $PWD/apps:/app/apps registry.factorial.io/administration/yafbds/yafbds:main
```

To run the cli use

```shell
docker run -it registry.factorial.io/administration/yafbds/yafbds:main /app/yafbdsctl
```


## CLI usage

You need to pass the address to the server to the cli, either by providing the `--server`-argument or by setting the `YAFBDS_SERVER` env-var.

```shell
yafbdsctl help
```

will show some help and a list of available commands. You can get help with `yafbdsctl help <command>`

Here's a short list of avaiable commands

* `yafbdsctl list` will list all apps and their their urls and states
* `yafbdsctl run <app_name>` will start and run the named app
* `yafbdsctl stop <app_name>` will stop the named app
* `yabfbdsctl rm <app_name>` will remove runtime files for the named app
  (similar to `docker-compose rm`)
