export class ParcelModal extends HTMLElement {
  private closing: boolean = false;
  private withHtmx: boolean = false;
  private underlayDismiss: boolean = true;

  connectedCallback() {
    this.withHtmx = this.getAttribute("with-htmx") !== null;

    const content = document.createElement("div");
    content.className = "content";

    while (this.firstChild) {
      content.appendChild(this.firstChild);
    }

    const underlay = document.createElement("div");
    underlay.className = "underlay";
    underlay.addEventListener("click", () => {
      if (this.underlayDismiss) {
        this.closeModal();
      }
    });

    this.addEventListener("animationend", () => {
      this.onAnimationEnd();
    });

    this.appendChild(underlay);
    this.appendChild(content);
    this.className = "modal";
  }

  onAnimationEnd() {
    if (this.closing && this.parentNode) {
      this.removeModal();
    }
  }

  removeModal() {
    if (this.withHtmx) {
      htmx.remove(this);
    } else {
      this.parentNode.removeChild(this);
    }
  }

  closeModal() {
    this.closing = true;
    this.classList.add("closing");
  }

  setUnderlayDismiss(value: boolean) {
    this.underlayDismiss = value;
  }
}

export function register() {
  customElements.define("parcel-modal", ParcelModal);
}
