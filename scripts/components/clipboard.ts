class ClipboardElement extends HTMLElement {
  static get observedAttributes() {
    return ["url", "value"];
  }

  connectedCallback() {
    let valueAttribute = this.getAttribute("value");
    if (typeof valueAttribute !== "string") {
      return;
    }

    const urlAttribute = this.getAttribute("url");
    if (urlAttribute === "true") {
      const base = window.location.protocol + "//" + window.location.host;

      if (!valueAttribute.startsWith("/")) {
        valueAttribute = "/" + valueAttribute;
      }

      valueAttribute = base + valueAttribute;
    }

    const div = document.createElement("div");
    div.className = "icon-clipboard-copy";

    div.addEventListener("click", () => {
      const blob = new Blob([valueAttribute], { type: "text/plain" });
      const data = [new ClipboardItem({ ["text/plain"]: blob })];
      navigator.clipboard.write(data);

      div.className = "icon-clipboard-check";
      window.setTimeout(() => {
        div.className = "icon-clipboard-copy";
      }, 1000);
    });

    this.appendChild(div);
  }
}

export function register() {
  customElements.define("parcel-clipboard", ClipboardElement);
}
