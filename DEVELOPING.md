# Developing Konarr

## Requirements

- Cargo / Rust
- Nodejs / NPM

## Building

### Server

Live server on port `8000` (default Rocket port)

```bash
cargo run -p konarr-server
```

This will generate a `config/konarr.yml` and `data/konarr.db` files with the default settings.

**Watching / Live reloading:**

```bash
cargo watch -q -c -- cargo run -p konarr-server
```

**With frontend:**

If you want the Server / Rocket to serve the frontend, you need to build/bundle the frontend code.

```bash
cd client/
npm run build
```

*Note:* This isn't hot reloaded so follow the frontend section to do that.

### Frontend

**Setup**

```bash
git submodule update --init --recursive
cd client/
npm i
```

**Run vite:**

```bash
npm run dev
```

This will run Vite on port `5173` with hot reloading for the frontend.

### CLI

The CLI will use the `config/konarr.yml` to perform actions.

```bash
cargo run -p konarr-cli
```

**Database interactions:**

If you want to use the interactions with the database, you need to add the feature.

```bash
cargo run -p konarr-cli -F database
```

