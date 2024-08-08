import { register as register_checklist } from "./components/checklist.js";
import { register as register_clipboard } from "./components/clipboard.js";
import { register as register_date } from "./components/date.js";
import { register as register_modal } from "./components/modal.js";

function add_dropdowns() {
  document.querySelectorAll(".dropdown").forEach((element) => {
    element.addEventListener("click", (event) => {
      element.querySelector("ul").classList.toggle("invisible");
      // element.classList.toggle("open");
      event.stopPropagation();
    });
  });

  document.addEventListener("click", (event) => {
    if ((event.target as HTMLElement).closest(".dropdown")) {
      return;
    }

    document.querySelectorAll(".dropdown > ul").forEach((element) => {
      element.classList.add("invisible");
    });
  });
}

function init() {
  if (window.customElements) {
    console.log("Web components are supported");
    document.body.classList.add("has-web-components");
  } else {
    console.warn("Web components are not supported");
  }

  register_checklist();
  register_clipboard();
  register_date();
  register_modal();
  add_dropdowns();
}

init();
