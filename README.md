# parcel

A simple file upload tool.

- Uploaded files can be made public to allow download from anywhere.
- Number of downloads can be limited, and downloads can have an expiry date.

## Running Parcel

The easiest way to run Parcel is with Docker, using the
[blakerain/parcel](https://hub.docker.com/r/blakerain/parcel) image on Dockerhub:

```
docker run blakerain/parcel
```

Parcel can be controlled through arguments or environment variables. The environment variables are a
useful way to control Parcel when creating a Docker container.

| Environment Name   | Default              | Description                               |
|--------------------|----------------------|-------------------------------------------|
| `DB`               | `sqlite://parcel.db` | SQLite connection string                  |
| `CACHE_DIR`        | `./cache`            | Directory for file cache                  |
| `COOKIE_SECRET`    |                      | Secret used for session cookie encryption |
| `ANALYTICS_DOMAIN` |                      | Domain to use for analytics script        |
| `PLAUSIBLE_SCRIPT` |                      | URL for [Plausible Analytics] script      |

For example, if you had created a volume `parcel_data` and mounted it under `/data` you could tell
Parcel to store the DB and file cache in that location:

```
docker run -d \
  -v parcel_data:/data \
  -e DB=sqlite:///data/parcel.db \
  -e CACHE_DIR=/data/cache \
  blakerain/parcel
```

## Development

When running as a development server, [cargo watch] is mighty helpful. You may also wish to set up a
cookie key (in the `COOKIE_SECRET` environment variable) to avoid being signed out after a restart:

```bash
# Initial setup of a cookie key
COOKIE_SECRET=$(openssl rand -base64 32 | tr -d '\n' ; echo)
export COOKIE_SECRET

# Run the server, but recompile/restart on any changes
cargo watch -L debug -x run
```

> Please Note: The icon used for this application is the [package-open] icon from the [Lucide] icon
> pack.

[package-open]: https://lucide.dev/icons/package-open
[lucide]: https://lucide.dev/
[cargo watch]: https://github.com/watchexec/cargo-watch
[plausible analytics]: https://plausible.io/
