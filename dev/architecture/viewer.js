// The diagram viewer: each inlined view sits in a frame at a scale a
// reader can read, with zoom, pan and Fit, 1:1, Wide and Fullscreen
// controls. Without this script a view is scaled to the text column (the
// stylesheet's rule) and the controls stay hidden, so a page with no
// JavaScript shows a diagram and no dead buttons.
//
// Scale: a view is shown at the scale that fits the frame's width, held
// between MIN and 1, so a wide view scrolls inside its frame instead of
// shrinking its text to nothing, and a small one is not blown up. Fit
// shows the whole view, width and height, at whatever scale that takes;
// 1:1 shows it at the size LikeC4 laid it out. Zooming (the wheel with
// Ctrl or Cmd held, which is what a trackpad pinch sends in Chrome and
// Firefox, and Safari's own gesture events for a pinch; the + and -
// buttons; + - 0 with the frame focused) sets the scale freely, kept
// about the pointer; a plain wheel scrolls the page as usual. The scale
// is the view's CSS width, and panning is the frame's own scroll, by
// its scrollbars or by dragging, so text stays crisp and the browser
// keeps the position. Wide lets every figure on the site take the
// window's width, remembered like the color scheme; Fullscreen takes one
// figure to the screen.
(function () {
  var WIDE_KEY = "asbuilt-docs-wide";
  var MIN = 0.7;
  var MIN_ZOOM = 0.1;
  var MAX_ZOOM = 4;
  var STEP = 1.25;
  var WHEEL = 0.002; // scale change per pixel of wheel delta, exponential
  var DRAG = 4; // pixels of movement before a press becomes a drag
  var GUTTER = 16;
  var FRAME_SHARE = 0.75; // of the window's height; the stylesheet's 75vh
  var root = document.documentElement;
  // The zoom modifier's name for the hint: Cmd on Apple platforms, Ctrl
  // elsewhere. The script accepts either key everywhere.
  var MAC = /Mac|iPhone|iPad|iPod/.test(navigator.platform || "");
  var wide = false;
  try {
    wide = window.localStorage.getItem(WIDE_KEY) === "1";
  } catch (e) {}
  var figures = [];

  function viewBoxSize(svg) {
    var parts = (svg.getAttribute("viewBox") || "").trim().split(/[\s,]+/);
    var width = parseFloat(parts[2]);
    var height = parseFloat(parts[3]);
    return width > 0 && height > 0 ? { width: width, height: height } : null;
  }

  function layoutAll() {
    figures.forEach(function (f) {
      f.layout();
    });
  }

  function setup(figure) {
    var svg = figure.querySelector("svg.c4");
    var frame = figure.querySelector(".viewer-frame");
    var bar = figure.querySelector(".viewer-bar");
    if (!svg || !frame || !bar) return;
    var natural = viewBoxSize(svg);
    if (!natural) return;
    var mode = "auto"; // auto | fit | one | zoom
    var zoom = 1; // the scale in zoom mode
    var current = 1; // the scale last applied
    var pannable = false; // whether the view overflows its frame
    var readout = bar.querySelector(".viewer-scale");
    var full = bar.querySelector("[data-viewer-full]");
    var hint = bar.querySelector(".viewer-hint");
    // No wheel to hint at on a touch-only device.
    if (hint && window.matchMedia && window.matchMedia("(hover: none)").matches) hint = null;

    function fullscreen() {
      return document.fullscreenElement === figure;
    }
    function widthFit() {
      return frame.clientWidth / natural.width;
    }
    // The height the frame may take: the stylesheet's share of the
    // window, or in fullscreen the screen below the controls.
    function heightBudget() {
      if (fullscreen()) {
        return window.innerHeight - frame.getBoundingClientRect().top - GUTTER;
      }
      return FRAME_SHARE * window.innerHeight;
    }
    function scale() {
      if (mode === "zoom") return zoom;
      if (mode === "one") return 1;
      if (mode === "fit") return Math.min(widthFit(), heightBudget() / natural.height);
      return Math.min(1, Math.max(MIN, widthFit()));
    }
    function layout() {
      if (wide && !fullscreen()) {
        // The window's width, measured from the document (which leaves
        // out the scrollbar), from wherever the text column put us; the
        // root's own left accounts for any horizontal scroll.
        figure.style.marginLeft = "0";
        var left = figure.getBoundingClientRect().left - root.getBoundingClientRect().left;
        figure.style.marginLeft = GUTTER - left + "px";
        figure.style.width = root.clientWidth - 2 * GUTTER + "px";
      } else {
        figure.style.marginLeft = "";
        figure.style.width = "";
      }
      current = scale();
      svg.style.width = Math.round(natural.width * current) + "px";
      // A view that fits has nowhere to pan to: no grab cursor, no drag.
      pannable =
        frame.scrollWidth > frame.clientWidth + 1 || frame.scrollHeight > frame.clientHeight + 1;
      frame.classList.toggle("pannable", pannable);
      if (readout) readout.textContent = Math.round(current * 100) + "%";
      if (hint) {
        hint.textContent =
          (MAC ? "\u2318" : "Ctrl") + " scroll to zoom" + (pannable ? " \u00b7 drag to pan" : "");
      }
      bar.querySelectorAll("[data-viewer-mode]").forEach(function (button) {
        var pressed = button.getAttribute("data-viewer-mode") === mode;
        button.setAttribute("aria-pressed", String(pressed));
      });
      var wideButton = bar.querySelector("[data-viewer-wide]");
      if (wideButton) wideButton.setAttribute("aria-pressed", String(wide));
    }
    // Scale to `next`, keeping the view point under the frame point
    // (`fx`, `fy`, from the frame's top left; its center when absent)
    // where it is.
    function zoomTo(next, fx, fy) {
      next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
      if (fx === undefined) {
        fx = frame.clientWidth / 2;
        fy = frame.clientHeight / 2;
      }
      var before = current;
      var x = frame.scrollLeft + fx;
      var y = frame.scrollTop + fy;
      mode = "zoom";
      zoom = next;
      layout();
      var ratio = current / before;
      frame.scrollLeft = x * ratio - fx;
      frame.scrollTop = y * ratio - fy;
    }

    bar.querySelectorAll("[data-viewer-mode]").forEach(function (button) {
      button.addEventListener("click", function () {
        var next = button.getAttribute("data-viewer-mode");
        mode = mode === next ? "auto" : next;
        layout();
      });
    });
    bar.querySelectorAll("[data-viewer-zoom]").forEach(function (button) {
      button.addEventListener("click", function () {
        var out = button.getAttribute("data-viewer-zoom") === "out";
        zoomTo(out ? current / STEP : current * STEP);
      });
    });
    var wideButton = bar.querySelector("[data-viewer-wide]");
    if (wideButton) {
      wideButton.addEventListener("click", function () {
        wide = !wide;
        try {
          window.localStorage.setItem(WIDE_KEY, wide ? "1" : "0");
        } catch (e) {}
        layoutAll();
      });
    }
    if (full) {
      if (figure.requestFullscreen) {
        full.addEventListener("click", function () {
          if (fullscreen()) document.exitFullscreen();
          else figure.requestFullscreen();
        });
      } else {
        full.hidden = true;
      }
    }

    frame.addEventListener(
      "wheel",
      function (event) {
        if (!(event.ctrlKey || event.metaKey)) return;
        event.preventDefault();
        // Lines and pages to pixels: a line is about 16px, a page the frame.
        var unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? frame.clientHeight : 1;
        var delta = event.deltaY * unit;
        var rect = frame.getBoundingClientRect();
        zoomTo(current * Math.exp(-delta * WHEEL), event.clientX - rect.left, event.clientY - rect.top);
      },
      { passive: false }
    );
    // Safari reports a trackpad pinch as gesture events (its scale is
    // relative to the gesture's start), not as the wheel with Ctrl.
    var pinchBase = null;
    frame.addEventListener("gesturestart", function (event) {
      event.preventDefault();
      pinchBase = current;
    });
    frame.addEventListener("gesturechange", function (event) {
      event.preventDefault();
      if (pinchBase === null) return;
      var rect = frame.getBoundingClientRect();
      zoomTo(pinchBase * event.scale, event.clientX - rect.left, event.clientY - rect.top);
    });
    frame.addEventListener("gestureend", function () {
      pinchBase = null;
    });
    frame.addEventListener("keydown", function (event) {
      if (event.target !== frame || event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.key === "+" || event.key === "=") zoomTo(current * STEP);
      else if (event.key === "-") zoomTo(current / STEP);
      else if (event.key === "0") {
        mode = "auto";
        layout();
      } else return;
      event.preventDefault();
    });

    // Drag to pan, with the mouse: touch keeps the browser's own
    // scrolling. A press that moves becomes a drag and is not a click,
    // so a node's link under the pointer is not followed on release.
    var press = null;
    var suppressClick = false;
    // A node is a link, which the browser would start dragging as one.
    frame.addEventListener("dragstart", function (event) {
      event.preventDefault();
    });
    frame.addEventListener(
      "click",
      function (event) {
        if (!suppressClick) return;
        suppressClick = false;
        event.stopPropagation();
        event.preventDefault();
      },
      true
    );
    frame.addEventListener("pointerdown", function (event) {
      suppressClick = false;
      if (!pannable || event.pointerType !== "mouse" || event.button !== 0) return;
      if (event.target.closest("button")) return;
      press = {
        id: event.pointerId,
        x: event.clientX,
        y: event.clientY,
        left: frame.scrollLeft,
        top: frame.scrollTop,
        dragged: false,
      };
    });
    frame.addEventListener("pointermove", function (event) {
      if (!press || event.pointerId !== press.id) return;
      // A release the frame never saw (outside it, or behind another
      // window) leaves the press armed: the button being up says so.
      if (!(event.buttons & 1)) {
        press = null;
        frame.classList.remove("dragging");
        return;
      }
      var dx = event.clientX - press.x;
      var dy = event.clientY - press.y;
      if (!press.dragged && Math.abs(dx) < DRAG && Math.abs(dy) < DRAG) return;
      if (!press.dragged) {
        press.dragged = true;
        frame.classList.add("dragging");
        frame.setPointerCapture(press.id);
      }
      frame.scrollLeft = press.left - dx;
      frame.scrollTop = press.top - dy;
    });
    function release(event) {
      if (!press || event.pointerId !== press.id) return;
      // The click that follows a drag's release is swallowed above; the
      // flag is cleared by the next press, so a cancelled drag with no
      // click leaves nothing armed.
      suppressClick = press.dragged;
      press = null;
      frame.classList.remove("dragging");
    }
    frame.addEventListener("pointerup", release);
    frame.addEventListener("pointercancel", release);

    // An edge stands for the relations between what it joins, more than
    // its label can say; the page wrote them beside the view. A click on
    // such an edge opens them in a popover, each endpoint a link.
    var edges = null;
    var edgeData = figure.querySelector("script.viewer-edges");
    if (edgeData) {
      try {
        edges = JSON.parse(edgeData.textContent);
      } catch (e) {}
    }
    var popover = null;
    function closePopover(refocus) {
      if (!popover || popover.hidden) return;
      popover.hidden = true;
      if (refocus) frame.focus();
    }
    function endpoint(text, href) {
      var el = document.createElement(href ? "a" : "span");
      if (href) el.href = href;
      el.textContent = text;
      return el;
    }
    function openPopover(key, relations, event) {
      if (!popover) {
        popover = document.createElement("div");
        popover.className = "viewer-popover";
        popover.setAttribute("role", "dialog");
        popover.setAttribute("aria-label", "Relations");
        popover.hidden = true;
        var close = document.createElement("button");
        close.type = "button";
        close.className = "viewer-popover-close";
        close.setAttribute("aria-label", "Close");
        close.textContent = "\u00d7";
        close.addEventListener("click", function () {
          closePopover(true);
        });
        popover.appendChild(close);
        popover.appendChild(document.createElement("h3"));
        popover.appendChild(document.createElement("ul"));
        popover.addEventListener("keydown", function (e) {
          if (e.key === "Escape") {
            e.preventDefault();
            closePopover(true);
          }
        });
        figure.appendChild(popover);
      }
      popover.querySelector("h3").textContent = key.replace("->", " \u2192 ");
      var list = popover.querySelector("ul");
      list.textContent = "";
      relations.forEach(function (r) {
        var li = document.createElement("li");
        li.appendChild(endpoint(r.source, r.source_href));
        li.appendChild(document.createTextNode(" \u2192 "));
        li.appendChild(endpoint(r.target, r.target_href));
        var detail = document.createElement("span");
        detail.className = "rel-detail";
        detail.textContent =
          " " +
          r.kind +
          (r.items.length ? " " + r.items.join(", ") : "") +
          (r.technology ? " (" + r.technology + ")" : "");
        li.appendChild(detail);
        list.appendChild(li);
      });
      popover.hidden = false;
      // Beside the click, inside the figure (which scrolls in fullscreen).
      var rect = figure.getBoundingClientRect();
      var left = event.clientX - rect.left + figure.scrollLeft + 8;
      var top = event.clientY - rect.top + figure.scrollTop + 8;
      left = Math.max(0, Math.min(left, figure.scrollLeft + figure.clientWidth - popover.offsetWidth - 8));
      top = Math.max(0, Math.min(top, figure.scrollTop + figure.clientHeight - popover.offsetHeight - 8));
      popover.style.left = left + "px";
      popover.style.top = top + "px";
      popover.querySelector(".viewer-popover-close").focus();
    }
    if (edges) {
      frame.querySelectorAll(".edge[data-from]").forEach(function (edge) {
        var key = edge.getAttribute("data-from") + "->" + edge.getAttribute("data-to");
        if (edges[key]) edge.classList.add("c4-explained");
      });
      frame.addEventListener("click", function (event) {
        var edge = event.target.closest(".edge.c4-explained");
        if (!edge) return;
        var key = edge.getAttribute("data-from") + "->" + edge.getAttribute("data-to");
        event.preventDefault();
        openPopover(key, edges[key], event);
      });
      document.addEventListener("click", function (event) {
        if (!popover || popover.hidden || popover.contains(event.target)) return;
        // Another edge of this figure is re-opening it; any other click,
        // including an edge of another figure, closes it.
        if (frame.contains(event.target) && event.target.closest(".edge.c4-explained")) return;
        closePopover(false);
      });
      frame.addEventListener("keydown", function (event) {
        if (event.key === "Escape") closePopover(true);
      });
    }

    document.addEventListener("fullscreenchange", layout);
    // The frame's width changes with the column; the window's with Wide
    // (which sizes the figure itself, so the frame alone would not tell).
    if (window.ResizeObserver) new ResizeObserver(layout).observe(frame);

    figure.setAttribute("data-viewer-active", "");
    frame.setAttribute("tabindex", "0");
    bar.hidden = false;
    layout();
    figures.push({ layout: layout });
  }

  function setupAll(root) {
    if (root.matches && root.matches("figure[data-viewer]:not([data-viewer-active])")) setup(root);
    if (root.querySelectorAll) {
      root.querySelectorAll("figure[data-viewer]:not([data-viewer-active])").forEach(setup);
    }
  }

  document.addEventListener("DOMContentLoaded", function () {
    setupAll(document);
    window.addEventListener("resize", layoutAll);
    // A host that builds its page after load (a single-page app embedding
    // a figure) gets its figures set up as they arrive.
    if (window.MutationObserver) {
      new MutationObserver(function (records) {
        records.forEach(function (record) {
          record.addedNodes.forEach(function (node) {
            if (node.nodeType === 1) setupAll(node);
          });
        });
      }).observe(document.body, { childList: true, subtree: true });
    }
  });
})();
