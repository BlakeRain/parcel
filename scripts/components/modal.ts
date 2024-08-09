export class ParcelModal extends HTMLElement {
  private closing: boolean = false;
  private withHtmx: boolean = false;
  private withImmediate: boolean = false;
  private underlayDismiss: boolean = true;

  connectedCallback() {
    this.withHtmx = this.getAttribute("with-htmx") !== null;
    this.withImmediate = this.getAttribute("with-immediate") !== null;

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

    if (this.withImmediate) {
      this.className = "modal";
    } else {
      this.className = "modal opening";
    }
  }

  onAnimationEnd() {
    if (this.classList.contains("closing")) {
      this.classList.remove("closing");
    } else if (this.classList.contains("opening")) {
      this.classList.remove("opening");
    }

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
