//! Builds the portfolio, a set of "drawing sheets":
//!
//! - `content/site.toml` and `content/projects.toml` → `dist/index.html` (the
//!   drawing index) and `dist/work/<slug>/index.html` (one sheet per project);
//! - everything in `static/` is copied into `dist/` as-is.
//!
//! `cargo run` builds the public site (drafts left out).
//! `cargo run -- --serve [port]` builds with drafts and previews it locally.
//!
//! All links are relative, so the site works at a domain root and under
//! GitHub Pages' `/portfolio/` alike.

mod plate;

use serde::Deserialize;
use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Write},
    net::TcpListener,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

// ---------------------------------------------------------------- content ---

#[derive(Deserialize)]
struct Site {
    name: String,
    role: String,
    statement: String,
    about: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    label: String,
    url: String,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
enum Discipline {
    Architecture,
    Tools,
    Art,
}

impl Discipline {
    const ALL: [Discipline; 3] = [Discipline::Architecture, Discipline::Tools, Discipline::Art];

    fn letter(self) -> &'static str {
        match self {
            Discipline::Architecture => "A",
            Discipline::Tools => "T",
            Discipline::Art => "D",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Discipline::Architecture => "Architecture",
            Discipline::Tools => "Tools & Software",
            Discipline::Art => "Arts & Design",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Discipline::Architecture => "architecture",
            Discipline::Tools => "tools",
            Discipline::Art => "art",
        }
    }
}

#[derive(Deserialize)]
struct Fact {
    label: String,
    value: String,
}

#[derive(Deserialize)]
struct Image {
    src: String,
    #[serde(default)]
    caption: String,
}

#[derive(Deserialize)]
struct Project {
    slug: String,
    title: String,
    discipline: Discipline,
    year: String,
    kind: String,
    summary: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    facts: Vec<Fact>,
    #[serde(default)]
    links: Vec<Link>,
    #[serde(default)]
    images: Vec<Image>,
    #[serde(default)]
    draft: bool,
}

#[derive(Deserialize)]
struct Projects {
    #[serde(default)]
    project: Vec<Project>,
}

/// A project placed in the drawing set, with its sheet number (A-01, T-02…).
struct Sheet<'a> {
    p: &'a Project,
    code: String,
}

// ---------------------------------------------------------------- helpers ---

/// Escapes text for HTML element content and quoted attributes.
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

/// Blank-line-separated text → escaped `<p>` paragraphs.
fn paragraphs(text: &str) -> String {
    text.split("\n\n")
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{}</p>", esc(&p)))
        .collect()
}

fn valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// The current year (UTC), for the title block.
fn current_year() -> i64 {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400) as i64;
    // Howard Hinnant's civil_from_days, year only.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    yoe + era * 400 + i64::from(mp >= 10)
}

/// Orders projects by discipline (file order within each) and numbers them.
fn sheets(projects: &[Project]) -> Vec<Sheet<'_>> {
    let mut out = Vec::new();
    for d in Discipline::ALL {
        for (i, p) in projects.iter().filter(|p| p.discipline == d).enumerate() {
            out.push(Sheet { p, code: format!("{}-{:02}", d.letter(), i + 1) });
        }
    }
    out
}

fn validate(projects: &[Project]) -> Res<()> {
    let mut seen = HashSet::new();
    for p in projects {
        if !valid_slug(&p.slug) {
            return Err(format!("project {:?}: slug must use only a-z, 0-9 and -", p.slug).into());
        }
        if !seen.insert(&p.slug) {
            return Err(format!("project {:?}: slug used twice", p.slug).into());
        }
        for img in &p.images {
            if img.src.starts_with('/') || img.src.contains("..") || img.src.contains("://") {
                return Err(format!("project {:?}: image {:?} must be a path inside static/", p.slug, img.src).into());
            }
            if !Path::new("static").join(&img.src).exists() {
                eprintln!("warning: project {:?}: static/{} not found", p.slug, img.src);
            }
        }
    }
    Ok(())
}

// -------------------------------------------------------------- rendering ---

/// The first image, or the generated drawing.
fn figure(s: &Sheet, root: &str) -> String {
    match s.p.images.first() {
        Some(img) => format!(r#"<img src="{root}{}" alt="">"#, esc(&img.src)),
        None => plate::plate(&s.p.slug, &s.code),
    }
}

/// The page frame shared by every sheet: head, top bar, title block.
fn page(site: &Site, root: &str, title: &str, description: &str, sheet_code: &str, sheet_title: &str, main: &str) -> String {
    let nav: String = Discipline::ALL
        .iter()
        .map(|d| format!(r##"<a href="{root}#{}"><span class="mono">{}</span><span class="dn">{}</span></a>"##, d.id(), d.letter(), esc(d.name())))
        .collect();
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <meta name="description" content="{description}">
  <script>try {{ if (localStorage.getItem("theme") === "blueprint") document.documentElement.dataset.theme = "blueprint"; }} catch (e) {{}}</script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=Inter+Tight:wght@300;400;500&display=swap">
  <link rel="stylesheet" href="{root}style.css">
  <script src="{root}main.js" defer></script>
</head>
<body>
  <header class="bar">
    <a class="brand" href="{root}">{name}</a>
    <nav class="disciplines">{nav}</nav>
    <div class="bar-end">
      <span class="coords mono" aria-hidden="true">X 0000 · Y 0000</span>
      <button class="theme-toggle mono" type="button" aria-pressed="false">Blueprint</button>
    </div>
  </header>

  <main>
{main}
  </main>

  <footer class="titleblock">
    <div class="cell wide"><span class="k">Drawn by</span><span class="v">{name}</span></div>
    <div class="cell wide"><span class="k">Title</span><span class="v">{sheet_title}</span></div>
    <div class="cell"><span class="k">Sheet</span><span class="v mono">{sheet_code}</span></div>
    <div class="cell"><span class="k">Scale</span><span class="v mono">1:1</span></div>
    <div class="cell"><span class="k">Date</span><span class="v mono">{year}</span></div>
  </footer>
  <div id="preview" aria-hidden="true"></div>
</body>
</html>
"##,
        title = esc(title),
        description = esc(description),
        name = esc(&site.name),
        sheet_title = esc(sheet_title),
        year = current_year(),
    )
}

fn render_index(site: &Site, all: &[Sheet]) -> String {
    let mut main = String::new();

    let legend: String = Discipline::ALL
        .iter()
        .map(|d| {
            let n = all.iter().filter(|s| s.p.discipline == *d).count();
            format!(
                r##"<a href="#{}"><span class="mono">{}</span>{}<span class="mono count">{n:02}</span></a>"##,
                d.id(),
                d.letter(),
                esc(d.name())
            )
        })
        .collect();
    main.push_str(&format!(
        r#"    <section class="cover">
      <p class="label mono">00 — Index · {role}</p>
      <h1 class="statement">{statement}</h1>
      <nav class="legend" aria-label="Disciplines">{legend}</nav>
    </section>
"#,
        role = esc(&site.role),
        statement = esc(&site.statement),
    ));

    for d in Discipline::ALL {
        let mine: Vec<&Sheet> = all.iter().filter(|s| s.p.discipline == d).collect();
        let rows: String = if mine.is_empty() {
            format!(
                r#"<li><div class="row empty"><span class="code mono">{}-00</span><span class="title">In preparation</span></div></li>"#,
                d.letter()
            )
        } else {
            mine.iter()
                .map(|s| {
                    let draft = if s.p.draft { r#" <span class="draft mono">Draft</span>"# } else { "" };
                    format!(
                        r#"<li><a class="row" href="work/{slug}/"><span class="code mono">{code}</span><span class="title">{title}{draft}</span><span class="kind">{kind}</span><span class="year mono">{year}</span></a><template class="preview">{fig}</template></li>"#,
                        slug = s.p.slug,
                        code = s.code,
                        title = esc(&s.p.title),
                        kind = esc(&s.p.kind),
                        year = esc(&s.p.year),
                        fig = figure(s, ""),
                    )
                })
                .collect()
        };
        main.push_str(&format!(
            r#"    <section class="discipline" id="{id}">
      <header class="discipline-head">
        <span class="letter">{letter}</span>
        <h2>{name}</h2>
        <span class="mono count">{n:02} {sheets}</span>
      </header>
      <ol class="index">{rows}</ol>
    </section>
"#,
            id = d.id(),
            letter = d.letter(),
            name = esc(d.name()),
            n = mine.len(),
            sheets = if mine.len() == 1 { "sheet" } else { "sheets" },
        ));
    }

    let mut contact = String::new();
    if !site.location.is_empty() {
        contact.push_str(&format!("<dt>Based in</dt><dd>{}</dd>", esc(&site.location)));
    }
    if !site.email.is_empty() {
        contact.push_str(&format!(r#"<dt>Write</dt><dd><a href="mailto:{0}">{0}</a></dd>"#, esc(&site.email)));
    }
    if !site.links.is_empty() {
        let links: String = site
            .links
            .iter()
            .map(|l| format!(r#"<a href="{}">{}</a>"#, esc(&l.url), esc(&l.label)))
            .collect::<Vec<_>>()
            .join(" ");
        contact.push_str(&format!("<dt>Elsewhere</dt><dd>{links}</dd>"));
    }
    main.push_str(&format!(
        r#"    <section class="discipline notes" id="notes">
      <header class="discipline-head">
        <span class="letter">N</span>
        <h2>Notes</h2>
      </header>
      <div class="notes-body">
        <div class="about">{about}</div>
        <dl class="contact">{contact}</dl>
      </div>
    </section>"#,
        about = paragraphs(&site.about),
    ));

    let title = format!("{} — {}", site.name, site.role);
    page(site, "", &title, &site.statement, "00", "Index of drawings", &main)
}

fn render_sheet(site: &Site, all: &[Sheet], i: usize) -> String {
    let s = &all[i];
    let p = s.p;
    let d = p.discipline;
    let root = "../../";

    let mut facts = format!(
        r#"<dt>Discipline</dt><dd>{}</dd><dt>Year</dt><dd class="mono">{}</dd><dt>Type</dt><dd>{}</dd>"#,
        esc(d.name()),
        esc(&p.year),
        esc(&p.kind)
    );
    for f in &p.facts {
        facts.push_str(&format!("<dt>{}</dt><dd>{}</dd>", esc(&f.label), esc(&f.value)));
    }
    if !p.links.is_empty() {
        let links: String = p
            .links
            .iter()
            .map(|l| format!(r#"<a href="{}">{} ↗</a>"#, esc(&l.url), esc(&l.label)))
            .collect::<Vec<_>>()
            .join("<br>");
        facts.push_str(&format!("<dt>Links</dt><dd>{links}</dd>"));
    }

    let plates: String = if p.images.is_empty() {
        format!(r#"<figure class="fig schematic">{}<figcaption class="mono">Fig. 1 — Schematic</figcaption></figure>"#, figure(s, root))
    } else {
        p.images
            .iter()
            .enumerate()
            .map(|(n, img)| {
                let cap = if img.caption.is_empty() { String::new() } else { format!(" — {}", esc(&img.caption)) };
                format!(
                    r#"<figure class="fig"><img src="{root}{}" alt="{}" loading="lazy"><figcaption class="mono">Fig. {}{cap}</figcaption></figure>"#,
                    esc(&img.src),
                    esc(&img.caption),
                    n + 1
                )
            })
            .collect()
    };

    let link_to = |j: usize, rel: &str| {
        let t = &all[j];
        format!(
            r#"<a class="{rel}" href="../{}/"><span class="mono">{}</span>{}</a>"#,
            t.p.slug,
            t.code,
            esc(&t.p.title)
        )
    };
    let prev = if i > 0 { link_to(i - 1, "prev") } else { r#"<span class="prev"></span>"#.into() };
    let next = if i + 1 < all.len() { link_to(i + 1, "next") } else { r#"<span class="next"></span>"#.into() };
    let draft = if p.draft { r#"<span class="draft mono">Draft</span>"# } else { "" };

    let main = format!(
        r##"    <article class="sheet">
      <header class="sheet-head">
        <p class="label mono"><a href="{root}#{id}">{letter} — {dname}</a></p>
        <p class="sheet-code mono">{code} {draft}</p>
        <h1>{title}</h1>
        <p class="summary">{summary}</p>
      </header>
      <dl class="facts">{facts}</dl>
      <div class="plates">{plates}</div>
      <div class="body">{body}</div>
      <nav class="pager" aria-label="Other sheets">{prev}<a class="up mono" href="{root}#{id}">Index</a>{next}</nav>
    </article>"##,
        id = d.id(),
        letter = d.letter(),
        dname = esc(d.name()),
        code = s.code,
        title = esc(&p.title),
        summary = esc(&p.summary),
        body = paragraphs(&p.body),
    );

    let title = format!("{} · {} — {}", s.code, p.title, site.name);
    page(site, root, &title, &p.summary, &s.code, &p.title, &main)
}

// ------------------------------------------------------------------ build ---

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

fn read_toml<T: for<'de> Deserialize<'de>>(path: &str) -> Res<T> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(toml::from_str(&text).map_err(|e| format!("{path}: {e}"))?)
}

fn build(include_drafts: bool) -> Res<()> {
    let site: Site = read_toml("content/site.toml")?;
    let mut projects: Projects = read_toml("content/projects.toml")?;
    validate(&projects.project)?;
    if !include_drafts {
        projects.project.retain(|p| !p.draft);
    }
    let all = sheets(&projects.project);

    let dist = Path::new("dist");
    if dist.exists() {
        fs::remove_dir_all(dist)?;
    }
    copy_dir(Path::new("static"), dist)?;
    fs::write(dist.join("index.html"), render_index(&site, &all))?;
    for i in 0..all.len() {
        let dir = dist.join("work").join(&all[i].p.slug);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), render_sheet(&site, &all, i))?;
    }

    let drafts = all.iter().filter(|s| s.p.draft).count();
    println!(
        "Built dist/ with {} sheets{}.",
        all.len(),
        if include_drafts { format!(" ({drafts} drafts included)") } else { " (drafts left out)".into() }
    );
    Ok(())
}

// ------------------------------------------------------------------ serve ---

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("gif") => "image/gif",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// A minimal static file server for previewing `dist/` (no dependencies).
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

        let rel = path.trim_start_matches('/');
        if rel.split('/').any(|seg| seg == "..") {
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            continue;
        }
        let mut file = Path::new("dist").join(rel);
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

fn main() -> Res<()> {
    // Work from the project folder, so `cargo run --manifest-path …` works from anywhere.
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let serving = args.first().map(String::as_str) == Some("--serve");
    build(serving || args.iter().any(|a| a == "--drafts"))?;
    if serving {
        let port = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(8080);
        serve(port)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ tests ---

#[cfg(test)]
mod tests {
    use super::*;

    fn project(slug: &str, d: Discipline, draft: bool) -> Project {
        Project {
            slug: slug.into(),
            title: slug.into(),
            discipline: d,
            year: "2026".into(),
            kind: "Kind".into(),
            summary: "Summary".into(),
            body: String::new(),
            facts: vec![],
            links: vec![],
            images: vec![],
            draft,
        }
    }

    #[test]
    fn escapes_html() {
        assert_eq!(esc(r#"<a href="x">&'"#), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
    }

    #[test]
    fn paragraphs_split_on_blank_lines() {
        assert_eq!(paragraphs("one\nline\n\n<two>\n\n\n"), "<p>one line</p><p>&lt;two&gt;</p>");
    }

    #[test]
    fn slugs() {
        assert!(valid_slug("casa-2026"));
        for bad in ["", "Casa", "a b", "-a", "a-", "../x", "a/b"] {
            assert!(!valid_slug(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn sheets_are_numbered_per_discipline_in_order() {
        use Discipline::*;
        let ps = vec![project("t1", Tools, false), project("a1", Architecture, false), project("t2", Tools, false), project("d1", Art, false)];
        let codes: Vec<_> = sheets(&ps).iter().map(|s| (s.code.clone(), s.p.slug.clone())).collect();
        let want = [("A-01", "a1"), ("T-01", "t1"), ("T-02", "t2"), ("D-01", "d1")];
        assert_eq!(codes, want.map(|(c, s)| (c.to_string(), s.to_string())));
    }

    #[test]
    fn duplicate_slugs_are_rejected() {
        let ps = vec![project("x", Discipline::Art, false), project("x", Discipline::Tools, false)];
        assert!(validate(&ps).is_err());
    }

    #[test]
    fn year_is_plausible() {
        assert!((2024..2200).contains(&current_year()));
    }

    #[test]
    fn content_parses_and_renders() {
        let site: Site = read_toml("content/site.toml").unwrap();
        let projects: Projects = read_toml("content/projects.toml").unwrap();
        validate(&projects.project).unwrap();
        let all = sheets(&projects.project);
        let index = render_index(&site, &all);
        for (i, s) in all.iter().enumerate() {
            assert!(index.contains(&format!(r#"href="work/{}/""#, s.p.slug)));
            let page = render_sheet(&site, &all, i);
            assert!(page.contains(&s.code));
        }
    }

    #[test]
    fn empty_discipline_says_in_preparation() {
        let site: Site = read_toml("content/site.toml").unwrap();
        let ps = vec![project("t1", Discipline::Tools, false)];
        let html = render_index(&site, &sheets(&ps));
        assert!(html.contains("A-00"));
        assert!(html.contains("D-00"));
        assert!(html.contains("In preparation"));
    }
}
