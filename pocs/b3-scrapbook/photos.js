// B3 · Photos. Tracks each cover image (loading, loaded, missing) and adds a small control so the
// "missing photo" fallback can be previewed without deleting any files.
(function () {
  const BAD = "missing/none.jpg";
  const imgs = [...document.querySelectorAll("img[data-cover]")];
  for (const img of imgs) {
    img.dataset.src = img.getAttribute("src");
    const fig = img.closest(".photo");
    const ok = () => { fig.classList.add("loaded"); fig.classList.remove("no-photo"); };
    const bad = () => { fig.classList.add("no-photo"); fig.classList.remove("loaded"); };
    img.addEventListener("load", ok);
    img.addEventListener("error", bad);
    if (img.complete) (img.naturalWidth ? ok : bad)();
  }

  const bar = document.createElement("div");
  bar.className = "poc-photos";
  bar.setAttribute("role", "group");
  bar.setAttribute("aria-label", "Exploration controls: photos");
  bar.innerHTML = '<span>Photos</span><button type="button" data-p="on" aria-pressed="true">Show</button><button type="button" data-p="off" aria-pressed="false">Missing</button>';
  const style = document.createElement("style");
  style.textContent = `.poc-photos{position:fixed;left:12px;bottom:12px;z-index:99;display:flex;gap:4px;align-items:center;padding:6px 8px;border-radius:10px;background:#111c;color:#fff;font:600 12px/1 system-ui,sans-serif;backdrop-filter:blur(6px)}
  .poc-photos span{padding:0 6px;opacity:.7}.poc-photos button{min-height:32px;padding:0 10px;border:0;border-radius:6px;background:#fff2;color:#fff;font:inherit;cursor:pointer}
  .poc-photos button[aria-pressed="true"]{background:#fff;color:#111}.poc-photos button:focus-visible{outline:2px solid #fff;outline-offset:2px}
  @media (max-width:700px){.poc-photos{bottom:64px}}`;
  document.head.append(style);
  document.body.append(bar);
  bar.addEventListener("click", (e) => {
    const b = e.target.closest("[data-p]");
    if (!b) return;
    const off = b.dataset.p === "off";
    for (const x of bar.querySelectorAll("button")) x.setAttribute("aria-pressed", String(x === b));
    for (const img of imgs) {
      const fig = img.closest(".photo");
      fig.classList.remove("loaded", "no-photo");
      img.src = off ? BAD : img.dataset.src;
    }
  });
})();
