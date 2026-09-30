// The visitor's color scheme for these pages: System, Light or Dark,
// remembered for the whole site. Loaded in <head> so a saved choice
// applies before the first paint. The control ships hidden and this
// reveals it, so without JavaScript a page follows the system setting
// (or the scheme it was generated with) and shows no dead control.
(function () {
  var KEY = "asbuilt-docs-scheme";
  var MODES = ["system", "light", "dark"];
  var root = document.documentElement;
  var fallback = root.getAttribute("data-theme-default") || "system";
  var mode = fallback;
  try {
    var saved = window.localStorage.getItem(KEY);
    if (MODES.indexOf(saved) >= 0) mode = saved;
  } catch (e) {}
  function apply(next) {
    if (next === "light" || next === "dark") root.setAttribute("data-theme", next);
    else root.removeAttribute("data-theme");
  }
  apply(mode);
  document.addEventListener("DOMContentLoaded", function () {
    var select = document.getElementById("scheme");
    if (!select) return;
    select.value = mode;
    select.parentElement.hidden = false;
    select.addEventListener("change", function () {
      mode = MODES.indexOf(select.value) >= 0 ? select.value : fallback;
      apply(mode);
      try {
        window.localStorage.setItem(KEY, mode);
      } catch (e) {}
    });
  });
})();
