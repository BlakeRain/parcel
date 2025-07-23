class BaseUrlElement extends HTMLElement {
  connectedCallback() {
    var path = this.getAttribute("path");
    const base = window.location.protocol + "//" + window.location.host;

    if (!path.startsWith("/")) {
      path = "/" + path;
    }

    this.textContent = base + path;
  }
}

export function register() {
  customElements.define("parcel-baseurl", BaseUrlElement);
}
