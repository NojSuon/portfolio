//! Builds the portfolio, a set of "drawing sheets":
//!
//! - `content/site.toml` and `content/projects.toml` → `dist/index.html` (the
//!   drawing index) and `dist/work/<slug>/index.html` (one sheet per project),
//!   in Spanish; the same in English under `dist/en/` and Korean under `dist/ko/`;
//! - everything in `static/` is copied into `dist/` as-is.
//!
//! `cargo run` builds the public site (drafts left out).
//! `cargo run -- --serve [port]` builds with drafts and previews it locally.
//!
//! All links are relative, so the site works at a domain root and under
//! GitHub Pages' `/portfolio/` alike.

mod floorplan;
mod plate;

use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, Read, Write},
    net::TcpListener,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

type Res<T> = Result<T, Box<dyn std::error::Error>>;

// -------------------------------------------------------------- languages ---

/// The site is built once per language: Spanish at the root, English under
/// `en/`, Korean under `ko/`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    Es,
    En,
    Ko,
}

impl Lang {
    const ALL: [Lang; 3] = [Lang::Es, Lang::En, Lang::Ko];

    fn code(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
            Lang::Ko => "ko",
        }
    }

    /// Folder of this language inside the site.
    fn prefix(self) -> &'static str {
        match self {
            Lang::Es => "",
            Lang::En => "en/",
            Lang::Ko => "ko/",
        }
    }

    /// Its own name, for the language switch.
    fn name(self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
            Lang::Ko => "한국어",
        }
    }

    fn ui(self) -> &'static Ui {
        match self {
            Lang::Es => &UI_ES,
            Lang::En => &UI_EN,
            Lang::Ko => &UI_KO,
        }
    }
}

/// Fixed words of the page frame.
struct Ui {
    disciplines: [&'static str; 3],
    index: &'static str,
    index_of_drawings: &'static str,
    in_preparation: &'static str,
    sheet: &'static str,
    sheets: &'static str,
    notes: &'static str,
    drawn_by: &'static str,
    title: &'static str,
    sheet_no: &'static str,
    scale: &'static str,
    date: &'static str,
    based_in: &'static str,
    write: &'static str,
    elsewhere: &'static str,
    blueprint: &'static str,
    draft: &'static str,
    discipline: &'static str,
    year: &'static str,
    kind: &'static str,
    links: &'static str,
    other_sheets: &'static str,
    legend: &'static str,
    schematic: &'static str,
    floor_plan: &'static str,
    language: &'static str,
}

const UI_ES: Ui = Ui {
    disciplines: ["Arquitectura", "Herramientas y software", "Arte y diseño"],
    index: "Índice",
    index_of_drawings: "Índice de planos",
    in_preparation: "En preparación",
    sheet: "lámina",
    sheets: "láminas",
    notes: "Notas",
    drawn_by: "Dibujado por",
    title: "Título",
    sheet_no: "Lámina",
    scale: "Escala",
    date: "Fecha",
    based_in: "Con base en",
    write: "Escríbeme",
    elsewhere: "En otros sitios",
    blueprint: "Cianotipo",
    draft: "Borrador",
    discipline: "Disciplina",
    year: "Año",
    kind: "Tipo",
    links: "Enlaces",
    other_sheets: "Otras láminas",
    legend: "Disciplinas",
    schematic: "Esquema",
    floor_plan: "Planta",
    language: "Idioma",
};

const UI_EN: Ui = Ui {
    disciplines: ["Architecture", "Tools & Software", "Arts & Design"],
    index: "Index",
    index_of_drawings: "Index of drawings",
    in_preparation: "In preparation",
    sheet: "sheet",
    sheets: "sheets",
    notes: "Notes",
    drawn_by: "Drawn by",
    title: "Title",
    sheet_no: "Sheet",
    scale: "Scale",
    date: "Date",
    based_in: "Based in",
    write: "Write",
    elsewhere: "Elsewhere",
    blueprint: "Blueprint",
    draft: "Draft",
    discipline: "Discipline",
    year: "Year",
    kind: "Type",
    links: "Links",
    other_sheets: "Other sheets",
    legend: "Disciplines",
    schematic: "Schematic",
    floor_plan: "Floor plan",
    language: "Language",
};

const UI_KO: Ui = Ui {
    disciplines: ["건축", "도구와 소프트웨어", "예술과 디자인"],
    index: "목차",
    index_of_drawings: "도면 목록",
    in_preparation: "준비 중",
    sheet: "장",
    sheets: "장",
    notes: "노트",
    drawn_by: "작성",
    title: "제목",
    sheet_no: "도면 번호",
    scale: "축척",
    date: "날짜",
    based_in: "활동 지역",
    write: "연락처",
    elsewhere: "링크",
    blueprint: "청사진",
    draft: "초안",
    discipline: "분야",
    year: "연도",
    kind: "유형",
    links: "링크",
    other_sheets: "다른 도면",
    legend: "분야",
    schematic: "개략도",
    floor_plan: "평면도",
    language: "언어",
};

/// Text in content files: either one string for every language, or a table
/// `{ es = "…", en = "…", ko = "…" }`. A missing language falls back to
/// Spanish, then English.
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum Text {
    Plain(String),
    Tr(BTreeMap<String, String>),
}

impl Default for Text {
    fn default() -> Self {
        Text::Plain(String::new())
    }
}

impl Text {
    fn get(&self, l: Lang) -> &str {
        match self {
            Text::Plain(s) => s,
            Text::Tr(m) => m
                .get(l.code())
                .or_else(|| m.get("es"))
                .or_else(|| m.get("en"))
                .or_else(|| m.values().next())
                .map_or("", String::as_str),
        }
    }

    /// Languages a translated text is missing.
    fn missing(&self) -> Vec<&'static str> {
        match self {
            Text::Plain(_) => vec![],
            Text::Tr(m) => Lang::ALL.iter().map(|l| l.code()).filter(|c| m.get(*c).is_none_or(|v| v.trim().is_empty())).collect(),
        }
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::Plain(s.into())
    }
}

// ---------------------------------------------------------------- content ---

#[derive(Deserialize)]
struct Site {
    name: String,
    role: Text,
    statement: Text,
    about: Text,
    #[serde(default)]
    location: Text,
    #[serde(default)]
    email: String,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    label: Text,
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

    fn name(self, l: Lang) -> &'static str {
        l.ui().disciplines[self as usize]
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
    label: Text,
    value: Text,
}

#[derive(Deserialize)]
struct Image {
    src: String,
    #[serde(default)]
    caption: Text,
}

#[derive(Deserialize)]
struct Project {
    slug: String,
    title: Text,
    discipline: Discipline,
    year: String,
    kind: Text,
    summary: Text,
    #[serde(default)]
    body: Text,
    #[serde(default)]
    facts: Vec<Fact>,
    #[serde(default)]
    links: Vec<Link>,
    #[serde(default)]
    images: Vec<Image>,
    /// A drawing from `content/plans/<plan>.toml`.
    #[serde(default)]
    plan: Option<String>,
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
    plan: Option<floorplan::Drawing>,
}

/// Where a page sits: its language, the way back to that language's index
/// (`home`) and to the site root with the shared assets (`assets`), and its
/// own path inside the language folder (`path`, "" for the index).
struct Here<'a> {
    lang: Lang,
    home: &'a str,
    assets: &'a str,
    path: String,
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
            out.push(Sheet { p, code: format!("{}-{:02}", d.letter(), i + 1), plan: None });
        }
    }
    out
}

/// Draws the floor plans of the sheets that have one, in one language.
fn attach_plans(all: &mut [Sheet], lang: Lang) -> Res<()> {
    for s in all.iter_mut() {
        if let Some(name) = &s.p.plan {
            if !valid_slug(name) {
                return Err(format!("project {:?}: plan {name:?} must use only a-z, 0-9 and -", s.p.slug).into());
            }
            s.plan = Some(floorplan::load(name, &s.p.slug, lang.code())?);
        }
    }
    Ok(())
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
        if !p.draft {
            let mut texts = vec![("title", &p.title), ("kind", &p.kind), ("summary", &p.summary), ("body", &p.body)];
            texts.extend(p.facts.iter().flat_map(|f| [("fact", &f.label), ("fact", &f.value)]));
            texts.extend(p.links.iter().map(|l| ("link", &l.label)));
            texts.extend(p.images.iter().map(|i| ("caption", &i.caption)));
            for (field, t) in texts {
                let missing = t.missing();
                if !missing.is_empty() {
                    eprintln!("warning: project {:?}: {field} has no {} translation", p.slug, missing.join(", "));
                }
            }
        }
    }
    Ok(())
}

// -------------------------------------------------------------- rendering ---

/// The floor plan, the first image, or the generated drawing.
fn figure(s: &Sheet, assets: &str) -> String {
    if let Some(plan) = &s.plan {
        return plan.svg.clone();
    }
    match s.p.images.first() {
        Some(img) => format!(r#"<img src="{assets}{}" alt="">"#, esc(&img.src)),
        None => plate::plate(&s.p.slug, &s.code),
    }
}

/// The page frame shared by every sheet: head, top bar, title block.
#[allow(clippy::too_many_arguments)]
fn page(site: &Site, here: &Here, title: &str, description: &str, sheet_code: &str, sheet_title: &str, main: &str) -> String {
    let (lang, home, assets) = (here.lang, here.home, here.assets);
    let ui = lang.ui();
    let nav: String = Discipline::ALL
        .iter()
        .map(|d| format!(r##"<a href="{home}#{}"><span class="mono">{}</span><span class="dn">{}</span></a>"##, d.id(), d.letter(), esc(d.name(lang))))
        .collect();
    let langs: String = Lang::ALL
        .iter()
        .map(|l| {
            let current = if *l == lang { r#" aria-current="true""# } else { "" };
            format!(
                r#"<a href="{assets}{}{}" hreflang="{code}" lang="{code}" title="{}"{current}>{}</a>"#,
                l.prefix(),
                here.path,
                l.name(),
                l.code().to_uppercase(),
                code = l.code(),
            )
        })
        .collect();
    let alternates: String = Lang::ALL
        .iter()
        .map(|l| format!(r#"<link rel="alternate" hreflang="{}" href="{assets}{}{}">"#, l.code(), l.prefix(), here.path))
        .collect::<Vec<_>>()
        .join("\n  ");
    let korean = if lang == Lang::Ko { "&family=Noto+Sans+KR:wght@300;400;500" } else { "" };
    format!(
        r##"<!doctype html>
<html lang="{code}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <meta name="description" content="{description}">
  {alternates}
  <script>try {{ if (localStorage.getItem("theme") === "blueprint") document.documentElement.dataset.theme = "blueprint"; }} catch (e) {{}}</script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=Inter+Tight:wght@300;400;500{korean}&display=swap">
  <link rel="stylesheet" href="{assets}style.css">
  <script src="{assets}main.js" defer></script>
</head>
<body>
  <header class="bar">
    <a class="brand" href="{home_link}">{name}</a>
    <nav class="disciplines">{nav}</nav>
    <div class="bar-end">
      <span class="coords mono" aria-hidden="true">X 0000 · Y 0000</span>
      <nav class="langs mono" aria-label="{language}">{langs}</nav>
      <button class="theme-toggle mono" type="button" aria-pressed="false">{blueprint}</button>
    </div>
  </header>

  <main>
{main}
  </main>

  <footer class="titleblock">
    <div class="cell wide"><span class="k">{drawn_by}</span><span class="v">{name}</span></div>
    <div class="cell wide"><span class="k">{title_k}</span><span class="v">{sheet_title}</span></div>
    <div class="cell"><span class="k">{sheet_k}</span><span class="v mono">{sheet_code}</span></div>
    <div class="cell"><span class="k">{scale}</span><span class="v mono">1:1</span></div>
    <div class="cell"><span class="k">{date}</span><span class="v mono">{year}</span></div>
  </footer>
  <div id="preview" aria-hidden="true"></div>
</body>
</html>
"##,
        code = lang.code(),
        title = esc(title),
        description = esc(description),
        home_link = if home.is_empty() { "./" } else { home },
        name = esc(&site.name),
        language = ui.language,
        blueprint = ui.blueprint,
        drawn_by = ui.drawn_by,
        title_k = ui.title,
        sheet_k = ui.sheet_no,
        scale = ui.scale,
        date = ui.date,
        sheet_title = esc(sheet_title),
        year = current_year(),
    )
}

fn render_index(site: &Site, all: &[Sheet], lang: Lang) -> String {
    let ui = lang.ui();
    let here = Here { lang, home: "", assets: if lang == Lang::Es { "" } else { "../" }, path: String::new() };
    let mut main = String::new();

    let legend: String = Discipline::ALL
        .iter()
        .map(|d| {
            let n = all.iter().filter(|s| s.p.discipline == *d).count();
            format!(
                r##"<a href="#{}"><span class="mono">{}</span>{}<span class="mono count">{n:02}</span></a>"##,
                d.id(),
                d.letter(),
                esc(d.name(lang))
            )
        })
        .collect();
    main.push_str(&format!(
        r#"    <section class="cover">
      <p class="label mono">00 — {index} · {role}</p>
      <h1 class="statement">{statement}</h1>
      <nav class="legend" aria-label="{legend_label}">{legend}</nav>
    </section>
"#,
        index = ui.index,
        role = esc(site.role.get(lang)),
        statement = esc(site.statement.get(lang)),
        legend_label = ui.legend,
    ));

    for d in Discipline::ALL {
        let mine: Vec<&Sheet> = all.iter().filter(|s| s.p.discipline == d).collect();
        let rows: String = if mine.is_empty() {
            format!(
                r#"<li><div class="row empty"><span class="code mono">{}-00</span><span class="title">{}</span></div></li>"#,
                d.letter(),
                ui.in_preparation
            )
        } else {
            mine.iter()
                .map(|s| {
                    let draft = if s.p.draft { format!(r#" <span class="draft mono">{}</span>"#, ui.draft) } else { String::new() };
                    format!(
                        r#"<li><a class="row" href="work/{slug}/"><span class="code mono">{code}</span><span class="title">{title}{draft}</span><span class="kind">{kind}</span><span class="year mono">{year}</span></a><template class="preview">{fig}</template></li>"#,
                        slug = s.p.slug,
                        code = s.code,
                        title = esc(s.p.title.get(lang)),
                        kind = esc(s.p.kind.get(lang)),
                        year = esc(&s.p.year),
                        fig = figure(s, here.assets),
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
            name = esc(d.name(lang)),
            n = mine.len(),
            sheets = if mine.len() == 1 { ui.sheet } else { ui.sheets },
        ));
    }

    let mut contact = String::new();
    let location = site.location.get(lang);
    if !location.is_empty() {
        contact.push_str(&format!("<dt>{}</dt><dd>{}</dd>", ui.based_in, esc(location)));
    }
    if !site.email.is_empty() {
        contact.push_str(&format!(r#"<dt>{}</dt><dd><a href="mailto:{1}">{1}</a></dd>"#, ui.write, esc(&site.email)));
    }
    if !site.links.is_empty() {
        let links: String = site
            .links
            .iter()
            .map(|l| format!(r#"<a href="{}">{}</a>"#, esc(&l.url), esc(l.label.get(lang))))
            .collect::<Vec<_>>()
            .join(" ");
        contact.push_str(&format!("<dt>{}</dt><dd>{links}</dd>", ui.elsewhere));
    }
    main.push_str(&format!(
        r#"    <section class="discipline notes" id="notes">
      <header class="discipline-head">
        <span class="letter">N</span>
        <h2>{notes}</h2>
      </header>
      <div class="notes-body">
        <div class="about">{about}</div>
        <dl class="contact">{contact}</dl>
      </div>
    </section>"#,
        notes = ui.notes,
        about = paragraphs(site.about.get(lang)),
    ));

    let title = format!("{} — {}", site.name, site.role.get(lang));
    page(site, &here, &title, site.statement.get(lang), "00", ui.index_of_drawings, &main)
}

fn render_sheet(site: &Site, all: &[Sheet], i: usize, lang: Lang) -> String {
    let ui = lang.ui();
    let s = &all[i];
    let p = s.p;
    let d = p.discipline;
    let here = Here { lang, home: "../../", assets: if lang == Lang::Es { "../../" } else { "../../../" }, path: format!("work/{}/", p.slug) };
    let (home, assets) = (here.home, here.assets);
    let title = p.title.get(lang);
    let summary = p.summary.get(lang);

    let mut facts = format!(
        r#"<dt>{}</dt><dd>{}</dd><dt>{}</dt><dd class="mono">{}</dd><dt>{}</dt><dd>{}</dd>"#,
        ui.discipline,
        esc(d.name(lang)),
        ui.year,
        esc(&p.year),
        ui.kind,
        esc(p.kind.get(lang))
    );
    for f in &p.facts {
        facts.push_str(&format!("<dt>{}</dt><dd>{}</dd>", esc(f.label.get(lang)), esc(f.value.get(lang))));
    }
    if let Some(plan) = &s.plan {
        for (k, v) in &plan.facts {
            facts.push_str(&format!(r#"<dt>{}</dt><dd class="mono">{}</dd>"#, esc(k), esc(v)));
        }
    }
    if !p.links.is_empty() {
        let links: String = p
            .links
            .iter()
            .map(|l| format!(r#"<a href="{}">{} ↗</a>"#, esc(&l.url), esc(l.label.get(lang))))
            .collect::<Vec<_>>()
            .join("<br>");
        facts.push_str(&format!("<dt>{}</dt><dd>{links}</dd>", ui.links));
    }

    let plates: String = if let Some(plan) = &s.plan {
        format!(
            r#"<figure class="fig plan-fig">{}<figcaption class="mono">Fig. 1 — {}</figcaption></figure>{}"#,
            plan.svg, ui.floor_plan, plan.schedule
        )
    } else if p.images.is_empty() {
        format!(r#"<figure class="fig schematic">{}<figcaption class="mono">Fig. 1 — {}</figcaption></figure>"#, figure(s, assets), ui.schematic)
    } else {
        p.images
            .iter()
            .enumerate()
            .map(|(n, img)| {
                let caption = img.caption.get(lang);
                let cap = if caption.is_empty() { String::new() } else { format!(" — {}", esc(caption)) };
                format!(
                    r#"<figure class="fig"><img src="{assets}{}" alt="{}" loading="lazy"><figcaption class="mono">Fig. {}{cap}</figcaption></figure>"#,
                    esc(&img.src),
                    esc(caption),
                    n + 1
                )
            })
            .collect()
    };

    let link_to = |j: usize, rel: &str| {
        let t = &all[j];
        format!(r#"<a class="{rel}" href="../{}/"><span class="mono">{}</span>{}</a>"#, t.p.slug, t.code, esc(t.p.title.get(lang)))
    };
    let prev = if i > 0 { link_to(i - 1, "prev") } else { r#"<span class="prev"></span>"#.into() };
    let next = if i + 1 < all.len() { link_to(i + 1, "next") } else { r#"<span class="next"></span>"#.into() };
    let draft = if p.draft { format!(r#"<span class="draft mono">{}</span>"#, ui.draft) } else { String::new() };

    let main = format!(
        r##"    <article class="sheet">
      <header class="sheet-head">
        <p class="label mono"><a href="{home}#{id}">{letter} — {dname}</a></p>
        <p class="sheet-code mono">{code} {draft}</p>
        <h1>{title}</h1>
        <p class="summary">{summary}</p>
      </header>
      <dl class="facts">{facts}</dl>
      <div class="plates">{plates}</div>
      <div class="body">{body}</div>
      <nav class="pager" aria-label="{other}">{prev}<a class="up mono" href="{home}#{id}">{index}</a>{next}</nav>
    </article>"##,
        id = d.id(),
        letter = d.letter(),
        dname = esc(d.name(lang)),
        code = s.code,
        title = esc(title),
        summary = esc(summary),
        body = paragraphs(p.body.get(lang)),
        other = ui.other_sheets,
        index = ui.index,
    );

    let page_title = format!("{} · {} — {}", s.code, title, site.name);
    page(site, &here, &page_title, summary, &s.code, title, &main)
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

    let dist = Path::new("dist");
    if dist.exists() {
        fs::remove_dir_all(dist)?;
    }
    copy_dir(Path::new("static"), dist)?;
    let mut count = 0;
    for lang in Lang::ALL {
        let mut all = sheets(&projects.project);
        attach_plans(&mut all, lang)?;
        let root = dist.join(lang.prefix());
        fs::create_dir_all(&root)?;
        fs::write(root.join("index.html"), render_index(&site, &all, lang))?;
        for i in 0..all.len() {
            let dir = root.join("work").join(&all[i].p.slug);
            fs::create_dir_all(&dir)?;
            fs::write(dir.join("index.html"), render_sheet(&site, &all, i, lang))?;
        }
        count = all.len();
    }

    let drafts = projects.project.iter().filter(|p| p.draft).count();
    println!(
        "Built dist/ with {count} sheets in {} languages{}.",
        Lang::ALL.len(),
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
            body: Text::default(),
            facts: vec![],
            links: vec![],
            images: vec![],
            plan: None,
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
        for lang in Lang::ALL {
            let mut all = sheets(&projects.project);
            attach_plans(&mut all, lang).unwrap();
            let index = render_index(&site, &all, lang);
            assert!(index.contains(&format!(r#"<html lang="{}">"#, lang.code())));
            for (i, s) in all.iter().enumerate() {
                assert!(index.contains(&format!(r#"href="work/{}/""#, s.p.slug)));
                let page = render_sheet(&site, &all, i, lang);
                assert!(page.contains(&s.code));
            }
        }
    }

    #[test]
    fn public_content_is_translated() {
        let site: Site = read_toml("content/site.toml").unwrap();
        for (field, t) in [("role", &site.role), ("statement", &site.statement), ("about", &site.about)] {
            assert!(t.missing().is_empty(), "site.{field} is missing {:?}", t.missing());
        }
        let projects: Projects = read_toml("content/projects.toml").unwrap();
        for p in projects.project.iter().filter(|p| !p.draft) {
            for (field, t) in [("title", &p.title), ("kind", &p.kind), ("summary", &p.summary), ("body", &p.body)] {
                assert!(t.missing().is_empty(), "{}: {field} is missing {:?}", p.slug, t.missing());
            }
            for f in &p.facts {
                assert!(f.label.missing().is_empty() && f.value.missing().is_empty(), "{}: a fact is missing a language", p.slug);
            }
        }
    }

    #[test]
    fn text_falls_back_to_spanish() {
        let t: Text = toml::from_str::<BTreeMap<String, Text>>(r#"t = { es = "Hola", en = "Hello" }"#).unwrap().remove("t").unwrap();
        assert_eq!(t.get(Lang::En), "Hello");
        assert_eq!(t.get(Lang::Ko), "Hola");
        assert_eq!(t.missing(), ["ko"]);
        assert_eq!(Text::from("Rust").get(Lang::Ko), "Rust");
    }

    #[test]
    fn language_switch_links_the_same_page() {
        let site: Site = read_toml("content/site.toml").unwrap();
        let ps = vec![project("t1", Discipline::Tools, false)];
        let all = sheets(&ps);
        let en = render_sheet(&site, &all, 0, Lang::En);
        assert!(en.contains(r#"href="../../../work/t1/" hreflang="es""#));
        assert!(en.contains(r#"href="../../../ko/work/t1/" hreflang="ko""#));
        assert!(en.contains(r#"href="../../../style.css""#));
        let es = render_index(&site, &all, Lang::Es);
        assert!(es.contains(r#"href="en/" hreflang="en""#));
        assert!(es.contains(r#"href="style.css""#));
    }

    #[test]
    fn empty_discipline_says_in_preparation() {
        let site: Site = read_toml("content/site.toml").unwrap();
        let ps = vec![project("t1", Discipline::Tools, false)];
        let html = render_index(&site, &sheets(&ps), Lang::En);
        assert!(html.contains("A-00"));
        assert!(html.contains("D-00"));
        assert!(html.contains("In preparation"));
        assert!(render_index(&site, &sheets(&ps), Lang::Es).contains("En preparación"));
    }
}
