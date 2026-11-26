// The book has one theme. A reader who picked another one before it did keeps their choice in
// local storage, which would put dark code colours on the paper: put it back to the one that works.
(() => {
  const root = document.documentElement;
  try {
    localStorage.removeItem("mdbook-theme");
  } catch {
    // Storage may be blocked; the class below is what matters.
  }
  for (const name of ["rust", "coal", "navy", "ayu"]) root.classList.remove(name);
  root.classList.add("light");
})();
