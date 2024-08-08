class ClipboardElement extends HTMLElement {
  private icon: HTMLDivElement = null;

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

    this.icon = this.attachIcon();
    this.icon.className = "icon-clipboard-copy";

    this.icon.addEventListener("click", () => {
      const blob = new Blob([valueAttribute], { type: "text/plain" });
      const data = [new ClipboardItem({ ["text/plain"]: blob })];
      navigator.clipboard.write(data);

      this.icon.className = "icon-clipboard-check";
      window.setTimeout(() => {
        this.icon.className = "icon-clipboard-copy";
      }, 1000);
    });
  }

  disconnectedCallback() {
    this.removeChild(this.icon);
    this.icon = null;
  }

  attachIcon() {
    let icon = null;

    if (this.children.length > 0) {
      icon = this.children[0];
      if (icon.tagName !== "DIV") {
        icon = null;
      }
    }

    if (!icon) {
      icon = this.attachNewIcon();
    }

    return icon;
  }

  attachNewIcon() {
    const div = document.createElement("div");
    div.className = "icon-clipboard-copy";
    this.appendChild(div);
    return div;
  }
}

export function register() {
  customElements.define("parcel-clipboard", ClipboardElement);
}
