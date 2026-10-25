// Copy buttons on the commands, and the screenshots opened large in place.
// Without JavaScript the commands are still selectable and the screenshots
// are plain links to the images.

for (const block of document.querySelectorAll(".code")) {
  const button = document.createElement("button");
  button.className = "copy";
  button.type = "button";
  button.textContent = document.documentElement.lang === "pt-BR" ? "Copiar" : "Copy";
  const label = button.textContent;
  button.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(block.querySelector("pre").innerText.trim());
      button.textContent = "✓";
    } catch {
      button.textContent = "×";
    }
    setTimeout(() => (button.textContent = label), 1500);
  });
  block.append(button);
}

const lightbox = document.querySelector("dialog.lightbox");
if (lightbox) {
  const image = lightbox.querySelector("img");
  for (const link of document.querySelectorAll(".shot a")) {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      image.src = link.href;
      image.alt = link.querySelector("img").alt;
      lightbox.showModal();
    });
  }
  lightbox.addEventListener("click", () => lightbox.close());
}
