import { register as register_baseurl } from "./components/baseurl";
import { register as register_checklist } from "./components/checklist";
import { register as register_clipboard } from "./components/clipboard";
import { register as register_date } from "./components/date";
import { register as register_modal } from "./components/modal";
import { register as register_select } from "./components/select";
import { register as register_dropdown } from "./components/dropdown";

function init() {
  if (window.customElements) {
    console.log("Web components are supported");
    document.body.classList.add("has-web-components");
  } else {
    console.warn("Web components are not supported");
  }

  register_baseurl();
  register_checklist();
  register_clipboard();
  register_date();
  register_modal();
  register_select();
  register_dropdown();

  document.addEventListener("click", (event) => {
    if ((event.target as HTMLElement).closest(".dismiss-skip")) {
      return;
    }

    document.querySelectorAll(".dismiss-visible").forEach((element) => {
      element.classList.add("invisible");
    });
  });
}

init();
