import register from "./components/date.js";

function add_dropdowns() {
  document.querySelectorAll(".dropdown").forEach((element) => {
    element.addEventListener("click", (event) => {
      element.querySelector("ul").classList.toggle("invisible");
      // element.classList.toggle("open");
      event.stopPropagation();
    });
  });

  document.addEventListener("click", (event) => {
    if (event.target.closest(".dropdown")) {
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

  register();
  add_dropdowns();
}

init();
