//! Builds the portfolio: reads `content/site.toml` and `content/projects.toml`,
//! renders `dist/index.html`, and copies everything in `static/` next to it.
//!
//! All links are relative, so the output works at the domain root and under
//! GitHub Pages' `/portfolio/` alike.

use serde::Deserialize;
use std::{
    fs,
    io::{self, Read, Write},
    net::TcpListener,
    path::Path,
};

#[derive(Deserialize)]
struct Site {
    name: String,
    title: String,
    tagline: String,
    about: String,
    github: String,
}

#[derive(Deserialize)]
struct Projects {
    project: Vec<Project>,
}

#[derive(Deserialize)]
struct Project {
    name: String,
    summary: String,
    tags: Vec<String>,
    repo: String,
    live: Option<String>,
    #[serde(default)]
    featured: bool,
}

/// Escapes text for use in HTML element content and quoted attributes.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn card(p: &Project) -> String {
    let tags_attr = esc(&p.tags.join("|"));
    let tags: String = p
        .tags
        .iter()
        .map(|t| format!(r#"<li class="tag">{}</li>"#, esc(t)))
        .collect();
    let live = p.live.as_deref().map_or(String::new(), |url| {
        format!(r#"<a class="btn btn-primary" href="{}">Visit site</a>"#, esc(url))
    });
    let class = if p.featured { "card featured" } else { "card" };
    format!(
        r#"      <article class="{class}" data-tags="{tags_attr}">
        <h3>{name}</h3>
        <p>{summary}</p>
        <ul class="tags">{tags}</ul>
        <div class="links">{live}<a class="btn" href="{repo}">Source</a></div>
      </article>
"#,
        name = esc(&p.name),
        summary = esc(&p.summary),
        repo = esc(&p.repo),
    )
}

fn render(site: &Site, projects: &[Project]) -> String {
    let cards: String = projects.iter().map(card).collect();
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <meta name="description" content="{tagline}">
  <script>
    // Apply the saved theme before first paint, so the page never flashes.
    try {{ var t = localStorage.getItem("theme"); if (t) document.documentElement.dataset.theme = t; }} catch (e) {{}}
  </script>
  <link rel="stylesheet" href="style.css">
  <script src="main.js" defer></script>
</head>
<body>
  <header class="top">
    <a class="brand" href="./">{name}</a>
    <nav>
      <a href="#work">Work</a>
      <a href="#about">About</a>
      <a href="{github}">GitHub</a>
      <button class="theme-toggle" type="button" aria-label="Switch to dark theme">◐</button>
    </nav>
  </header>

  <main>
    <section class="hero">
      <h1>{name}</h1>
      <p class="tagline">{tagline}</p>
    </section>

    <section id="work">
      <div class="section-head">
        <h2>Work</h2>
        <div class="filters" role="group" aria-label="Filter projects by technology"></div>
      </div>
      <div class="grid">
{cards}      </div>
    </section>

    <section id="about">
      <h2>About</h2>
      <p>{about}</p>
    </section>
  </main>

  <footer>
    <p>Built with Rust and JavaScript · <a href="{github}/portfolio">Source</a></p>
  </footer>
</body>
</html>
"##,
        title = esc(&site.title),
        name = esc(&site.name),
        tagline = esc(&site.tagline),
        about = esc(&site.about),
        github = esc(&site.github),
    )
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(toml::from_str(&text).map_err(|e| format!("{path}: {e}"))?)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// A minimal static file server for previewing `dist/` locally (no dependencies).
fn serve(port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    println!("Serving dist/ at http://localhost:{port}/ (Ctrl+C to stop)");
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request.split_whitespace().nth(1).unwrap_or("/");
        let path = target.split(['?', '#']).next().unwrap_or("/");

        // Refuse anything that tries to leave dist/.
        let rel = path.trim_start_matches('/');
        let mut file = Path::new("dist").join(rel);
        if rel.split('/').any(|seg| seg == "..") {
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
            continue;
        }
        if file.is_dir() {
            file = file.join("index.html");
        }
        let response = match fs::read(&file) {
            Ok(body) => {
                let mut head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                    content_type(&file),
                    body.len()
                )
                .into_bytes();
                head.extend(body);
                head
            }
            Err(_) => b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot found".to_vec(),
        };
        let _ = stream.write_all(&response);
    }
    Ok(())
}

fn build() -> Result<(), Box<dyn std::error::Error>> {
    let site: Site = read_toml("content/site.toml")?;
    let projects: Projects = read_toml("content/projects.toml")?;

    let dist = Path::new("dist");
    if dist.exists() {
        fs::remove_dir_all(dist)?;
    }
    copy_dir(Path::new("static"), dist)?;
    fs::write(dist.join("index.html"), render(&site, &projects.project))?;

    println!("Built dist/ with {} projects.", projects.project.len());
    Ok(())
}

/// `cargo run` builds dist/. `cargo run -- --serve [port]` builds, then serves it.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Work from the project folder, so `cargo run --manifest-path …` works from anywhere.
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;
    build()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--serve") {
        let port = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(8080);
        serve(port)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html() {
        assert_eq!(esc(r#"<a href="x">&'"#), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
    }

    #[test]
    fn content_parses_and_renders() {
        let site: Site = read_toml("content/site.toml").unwrap();
        let projects: Projects = read_toml("content/projects.toml").unwrap();
        assert!(!projects.project.is_empty());
        let html = render(&site, &projects.project);
        for p in &projects.project {
            assert!(html.contains(&esc(&p.name)), "missing card for {}", p.name);
        }
    }

    #[test]
    fn card_without_live_link_has_only_source() {
        let p = Project {
            name: "X".into(),
            summary: "S".into(),
            tags: vec!["Rust".into(), "3D".into()],
            repo: "https://example.com".into(),
            live: None,
            featured: false,
        };
        let html = card(&p);
        assert!(html.contains(r#"data-tags="Rust|3D""#));
        assert!(!html.contains("Visit site"));
        assert!(html.contains("Source"));
    }
}
