import register from "./components/date.js";

function init() {
  if (window.customElements) {
    console.log("Web components are supported");
    document.body.classList.add("has-web-components");
  } else {
    console.warn("Web components are not supported");
  }

  register();
}

init();
