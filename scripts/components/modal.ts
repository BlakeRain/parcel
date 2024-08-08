export class ParcelModal extends HTMLElement {
  private closing: boolean = false;
  private withHtmx: boolean = false;

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
      this.closeModal();
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
}

export function register() {
  customElements.define("parcel-modal", ParcelModal);
}
