// Theme toggle and project filters. The page works without this file;
// it only adds the dark theme button and the technology filter.

(() => {
  const root = document.documentElement;

  // --- Theme --------------------------------------------------------------
  const toggle = document.querySelector(".theme-toggle");

  function applyTheme(theme) {
    root.dataset.theme = theme;
    const next = theme === "dark" ? "light" : "dark";
    toggle.setAttribute("aria-label", `Switch to ${next} theme`);
    toggle.title = `Switch to ${next} theme`;
  }

  applyTheme(root.dataset.theme === "dark" ? "dark" : "light");

  toggle.addEventListener("click", () => {
    const theme = root.dataset.theme === "dark" ? "light" : "dark";
    applyTheme(theme);
    try { localStorage.setItem("theme", theme); } catch (e) { /* private mode */ }
  });

  // --- Filters ------------------------------------------------------------
  const cards = [...document.querySelectorAll(".card")];
  const bar = document.querySelector(".filters");

  // Count tags and offer only those shared by two or more projects.
  const counts = new Map();
  for (const card of cards) {
    for (const tag of card.dataset.tags.split("|")) {
      counts.set(tag, (counts.get(tag) || 0) + 1);
    }
  }
  const tags = [...counts].filter(([, n]) => n > 1).map(([t]) => t);
  if (tags.length === 0) return;

  const buttons = ["All", ...tags].map((tag) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = tag;
    b.dataset.tag = tag;
    b.setAttribute("aria-pressed", String(tag === "All"));
    b.addEventListener("click", () => select(tag));
    bar.append(b);
    return b;
  });

  function select(tag) {
    for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.tag === tag));
    for (const card of cards) {
      card.hidden = tag !== "All" && !card.dataset.tags.split("|").includes(tag);
    }
  }
})();
