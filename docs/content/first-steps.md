# First Steps with Scotty

This guide will walk you through setting up Scotty and creating your first app.
We'll use a simple nginx webserver as an example.

Please note that this example will use subdomains of `localhost`. Not all
systems support this and might not resolve them to 127.0.0.1

You can always add the domains to your local `/etc/hosts` to make them work. For
this walk-through, add the following line to your `/etc/hosts`:

```
127.0.0.1	localhost scotty.localhost nginx.my-nginx-test.localhost
```


## Installing the Server

First, let's get the Scotty server up and running. We'll use docker-compose for
this setup.

1. Create a new directory for your Scotty installation:

    ```shell
    mkdir scotty-server && cd scotty-server
    mkdir apps
    ```

2. Create a `docker-compose.yml` with the following content:

```yaml
services:
  traefik:
    image: "traefik:v3.1"
    command:
      - "--log.level=DEBUG"
      - "--api.insecure=true"
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--providers.docker.network=proxy"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - "/var/run/docker.sock:/var/run/docker.sock:ro"
    networks:
      - default
      - proxy
    restart: unless-stopped

  scotty:
    image: ghcr.io/factorial-io/scotty:main
    platform: "linux/amd64"
    volumes:
      - ./apps:$PWD/apps
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      RUST_LOG: info
      SCOTTY__APPS__ROOT_FOLDER: $PWD/apps
      SCOTTY__APPS__DOMAIN_SUFFIX: localhost
      SCOTTY__API__ACCESS_TOKEN: my-secret-token
      SCOTTY__TRAEFIK__USE_TLS: false
    networks:
      - default
      - proxy
    restart: unless-stopped
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.scotty.rule=Host(`scotty.localhost`)"
      - "traefik.http.services.scotty.loadbalancer.server.port=21342"

networks:
  proxy:
    external: true
```

3. Create the required network and start the services:

```shell
docker network create proxy
docker compose up -d
```

The server should now be running and accessible at `http://scotty.localhost`. The password is
`my-secret-token` and can be changed in the above `docker-compose.yml`-file by changing
the env-var `SCOTTY__API__ACCESS_TOKEN`.


## Installing the Client

Install the Scotty CLI (scottyctl) by downloading the latest release for your
platform:

```shell
# For macOS ARM64
curl -L https://github.com/factorial-io/scotty/releases/download/<LATEST-VERSION>/scottyctl-aarch64-apple-darwin.tar.gz -o scottyctl.tar.gz
tar -xvf scottyctl.tar.gz
chmod +x scottyctl
sudo mv scottyctl /usr/local/bin
```

Test your installation:

```shell
scottyctl --version
```

## Creating Your First App

Let's create a simple nginx-based app to test our setup.

1. Create a new directory for your app:

```shell
mkdir my-nginx-app && cd my-nginx-app
```

2. Create a `docker-compose.yml` file:

```yaml
services:
  nginx:
    image: nginx:alpine
    volumes:
      - ./html:/usr/share/nginx/html
```

3. Create some test content:

```shell
mkdir html
echo "<h1>Hello from Scotty!</h1>" > html/index.html
```

4. Create the app using scottyctl:

```shell
scottyctl --server http://scotty.localhost --access-token my-secret-token \
  app:create my-nginx-test \
  --folder . \
  --service nginx:80
```

This command:
- Connects to your Scotty server
- Creates a new app named "my-nginx-test"
- Uses your docker-compose.yml file and the html folder
- Exposes the nginx service on port 80

5. Your app should now be running and accessible at `http://nginx.my-nginx-test.localhost`

You can verify the app status with:

```shell
scottyctl --server http://scotty.localhost --access-token my-secret-token app:list
```

This should show your running app along with its URL and status.

## Next Steps

Now that you have your first app running, you can:

- Try stopping the app with `app:stop`
- Rebuild it with `app:rebuild`
- Destroy it with `app:destroy`
- Create more complex apps with multiple services
- Explore the Scotty UI at `http://scotty.localhost`

For more advanced usage, check out:
- [The Configuration Guide](configuration.md) to learn about all available settings
- [The CLI Documentation](cli.md) for all available commands
- [The Architecture Documentation](architecture.md) to understand how Scotty works
