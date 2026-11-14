# Noj Suon · Portfolio

Personal portfolio for architecture, tools & software, and arts & design, laid out as a set of drawing sheets. Live at https://nojsuon.github.io/portfolio/.

Every project is a numbered sheet: **A-01…** Architecture, **T-01…** Tools & Software, **D-01…** Arts & Design. The index lists them like a drawing register. Each project gets its own page with a title block of facts, figures and text.

## How it works

- **Rust** (`src/`) reads the content and writes the static site to `dist/`: the index and one page per project at `work/<slug>/`. Projects without images get a generated line drawing (`src/plate.rs`), always the same one for the same project.
- **JavaScript** (`static/main.js`) adds the Paper/Blueprint theme switch, the cursor coordinate readout, and the drawing preview that follows the cursor over the index. The site works without it.

| Folder / file | What it holds |
|---|---|
| `content/site.toml` | Name, statement, about text, contact and links |
| `content/projects.toml` | One `[[project]]` per sheet; the comments at the top list every field |
| `static/` | `style.css`, `main.js`, and your images (e.g. `static/images/<slug>/1.jpg`) |
| `.github/workflows/pages.yml` | Tests, builds and deploys to GitHub Pages on every push to `main` |

## Adding a project

1. Add a `[[project]]` block to `content/projects.toml` with `discipline = "architecture"`, `"tools"` or `"art"`.
2. Put its images in `static/images/<slug>/` and list them under `images`. The first one is the preview on the index.
3. Keep `draft = true` while you work on it: drafts show in the local preview, never on the live site.

## Preview and build

Build with drafts and preview at http://localhost:8080/:

```bash
cargo run -- --serve
```

Build the public site (drafts left out), as the deploy does:

```bash
cargo run
```

Run the tests:

```bash
cargo test
```
