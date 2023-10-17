# parcel

A simple light-weight file upload tool

## Development

Before development starts, you will need to install the Node dependencies:

```
npm install
```

When running as a development server, [cargo watch] is mighty helpful. You may also wish to set up a
cookie key (in the `COOKIE_SECRET` environment variable) to avoid being signed out after a restart:

```bash
# Initial setup of a cookie key
COOKIE_SECRET=$(openssl rand -base64 32 | tr -d '\n' ; echo)

# Run the server, but recompile/restart on any changes
cargo watch -L debug -x run
```

> Please Note: The icon used for this application is the [package-open] icon from the [Lucide] icon
> pack.

[package-open]: https://lucide.dev/icons/package-open
[lucide]: https://lucide.dev/
[cargo watch]: https://github.com/watchexec/cargo-watch
