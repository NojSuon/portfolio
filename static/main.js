// Small touches on top of the static pages (all optional; the site works without them):
// the Paper/Blueprint theme, the cursor coordinate readout, and the drawing
// preview that follows the cursor over the index.

(() => {
  const root = document.documentElement;
  const fine = matchMedia("(hover: hover) and (pointer: fine)").matches;

  // ── Theme: Paper ↔ Blueprint ────────────────────────────────────────────
  const toggle = document.querySelector(".theme-toggle");
  const setTheme = (blueprint) => {
    if (blueprint) root.dataset.theme = "blueprint";
    else delete root.dataset.theme;
    toggle.setAttribute("aria-pressed", String(blueprint));
  };
  setTheme(root.dataset.theme === "blueprint");
  toggle.addEventListener("click", () => {
    const blueprint = root.dataset.theme !== "blueprint";
    setTheme(blueprint);
    try { localStorage.setItem("theme", blueprint ? "blueprint" : "paper"); } catch (e) { /* storage blocked */ }
  });

  // ── Cursor coordinates, in millimetres from the sheet's corner ──────────
  const coords = document.querySelector(".coords");
  const mm = (px) => String(Math.max(0, Math.round(px * 0.2646))).padStart(4, "0");
  let px = 0, py = 0, cx = 0, cy = 0, frame = 0;

  // ── Drawing preview over index rows ─────────────────────────────────────
  const preview = document.getElementById("preview");
  const previews = fine && preview && matchMedia("(min-width: 901px)").matches;
  if (previews) {
    for (const li of document.querySelectorAll(".index li")) {
      const link = li.querySelector("a.row");
      const tpl = li.querySelector("template.preview");
      if (!link || !tpl) continue;
      link.addEventListener("pointerenter", () => {
        preview.replaceChildren(tpl.content.cloneNode(true));
        place();
        preview.classList.add("on");
      });
      link.addEventListener("pointerleave", () => preview.classList.remove("on"));
    }
  }

  function place() {
    const w = preview.offsetWidth, h = preview.offsetHeight, m = 24;
    let x = cx + m, y = cy + m;
    if (x + w > innerWidth - 16) x = cx - m - w;
    if (y + h > innerHeight - 16) y = cy - m - h;
    preview.style.transform = `translate(${x}px, ${y}px)`;
  }

  function tick() {
    frame = 0;
    if (coords) coords.textContent = `X ${mm(px)} · Y ${mm(py)}`;
    if (previews) place();
  }

  if (fine) {
    addEventListener("pointermove", (e) => {
      px = e.pageX; py = e.pageY; cx = e.clientX; cy = e.clientY;
      if (!frame) frame = requestAnimationFrame(tick);
    }, { passive: true });
  }
})();
