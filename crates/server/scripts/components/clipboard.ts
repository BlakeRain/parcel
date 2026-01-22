class ClipboardElement extends HTMLElement {
  private icon: HTMLDivElement | null = null;

  static get observedAttributes() {
    return ["url", "value"];
  }

  getValue(): string | null {
    let value = this.getAttribute("value");
    if (typeof value !== "string") {
      return null;
    }

    const url = this.hasAttribute("url");
    if (url) {
      const base = window.location.protocol + "//" + window.location.host;

      if (!value.startsWith("/")) {
        value = "/" + value;
      }

      value = base + value;
    }

    return value;
  }

  connectedCallback() {
    this.icon = this.attachIcon();
    this.icon.className = "cursor-pointer icon-clipboard-copy";

    this.icon.addEventListener("click", () => {
      const value = this.getValue();
      if (!value) {
        console.warn("Clipboard: no value to copy");
        return;
      }

      const blob = new Blob([value], { type: "text/plain" });
      const data = [new ClipboardItem({ ["text/plain"]: blob })];

      navigator.clipboard.write(data)
        .then(() => {
          this.icon!.className = "cursor-pointer icon-clipboard-check text-success";
          window.setTimeout(() => {
            if (this.icon) {
              this.icon.className = "cursor-pointer icon-clipboard-copy";
            }
          }, 2000);
        })
        .catch((err) => {
          console.error("Failed to copy to clipboard:", err);
          // Show error state briefly
          this.icon!.className = "cursor-pointer icon-clipboard-x text-danger";
          window.setTimeout(() => {
            if (this.icon) {
              this.icon.className = "cursor-pointer icon-clipboard-copy";
            }
          }, 2000);
        });
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
    this.appendChild(div);
    return div;
  }
}

export function register() {
  customElements.define("parcel-clipboard", ClipboardElement);
}
