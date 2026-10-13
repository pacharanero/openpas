// SPDX-FileCopyrightText: 2026 Marcus Baw
// SPDX-License-Identifier: AGPL-3.0-or-later

document.addEventListener("DOMContentLoaded", () => {
  const drawer = document.querySelector("#app-drawer");
  const openButton = document.querySelector("#nav-open");
  const closeButton = document.querySelector("#nav-close");

  if (!drawer || !openButton || !closeButton) return;

  const setOpen = (open) => {
    drawer.checked = open;
    openButton.setAttribute("aria-expanded", String(open));
    setTimeout(() => {
      if (open) closeButton.focus();
      else openButton.focus();
    }, 250);
  };

  openButton.addEventListener("click", () => setOpen(true));
  closeButton.addEventListener("click", () => setOpen(false));
  drawer.addEventListener("change", () => {
    openButton.setAttribute("aria-expanded", String(drawer.checked));
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && drawer.checked) setOpen(false);
  });
});
