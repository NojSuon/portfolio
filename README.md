# Noj Suon · Portfolio

My portfolio site. Once deployed it lives at https://nojsuon.github.io/portfolio/.

## How it works

- **Rust** (`src/main.rs`) reads the content and writes the static site to `dist/`.
- **JavaScript** (`static/main.js`) adds the light/dark theme button and the technology filter on the project cards. The page still works without it.

| Folder / file | What it holds |
|---|---|
| `content/site.toml` | Name, tagline, about text, GitHub link |
| `content/projects.toml` | One `[[project]]` block per card |
| `static/` | `style.css`, `main.js` and anything else copied as-is into `dist/` |
| `.github/workflows/pages.yml` | Tests, builds and deploys to GitHub Pages on every push to `main` |

## Build and preview

Build `dist/`:

```bash
cargo run
```

Build and preview at http://localhost:8080/ (a tiny built-in server, no extra tools needed):

```bash
cargo run -- --serve
```

Run the tests with `cargo test`.

## Deploying

One-time setup on GitHub: **Settings → Pages → Build and deployment → Source: GitHub Actions**. After that, every push to `main` publishes the site.
