import { html } from "../shared.js";
import register from "preact-custom-element";

const UploadForm = (props) => {
  const onCancelClick = (event) => {
    event.preventDefault();
    event.target.closest("parcel-modal").closeModal();
  };

  return html`
    <h1 class="heading">New Upload</h1>
    <form class="form">
      <input type="hidden" name="csrf_token" value=${props.csrf_token} />
      <div>Form fields go here</div>
      <div class="buttons end mt-2">
        <a href="#" class="mx-2" onclick=${onCancelClick}> Cancel </a>
        <button type="button" class="button">
          <span class="icon-upload"></span>
          Upload file
        </button>
      </div>
    </form>
  `;
};

register(UploadForm, "parcel-upload-form", ["csrf_token"]);

/*
<script>
  function handleUploadFieldChange(event) {
    console.log("Number of selected files changed to", event.target.files.length);
    const form = document.querySelector("#upload-form");

    const submit = form.querySelector("button[type='submit']");
    submit.disabled = event.target.files.length === 0;
    submit.querySelector("span").textContent = event.target.files.length === 1 ? "Upload file" :
      "Upload files";

  }

  function handleUploadSubmit(form) {
    const input = form.querySelector("input[type='file']");
    const submit = form.querySelector("button[type='submit']");
    const progressContainer = form.querySelector(".progress-container");
    const progressElement = progressContainer.querySelector("progress");
    const progressText = progressContainer.querySelector("div");

    function getErrorElement() {
      const existing = form.querySelector("#upload-form > #error");
      if (existing) {
        return existing;
      }

      const element = document.createElement("DIV");
      element.classNames.add("text-danger");
      form.appendChild(element);

      return element;
    }

    function setError(message) {
      progressContainer.classList.add("invisible")
      getErrorElement().innerHTML = message;
    }

    submit.disabled = true;
    progressElement.value = 0;
    progressText.innerHTML = "";

    let total_files = 0, total_size = 0;
    const form_data = new FormData();

    for (let field of form.elements) {
      if (field.tagName !== "INPUT") {
        continue;
      }

      if (field.type === "file") {
        for (let file of field.files) {
          total_files += 1;
          total_size += file.size;
          form_data.append("file", file);
        }
      }
    }

    if (total_files === 0) {
      console.warn("No files selected, not submitting form");
      return;
    }

    let factor = 1, unit, digits = 0;
    if (total_size > 1024 * 1024 * 1024) {
      unit = "gigabyte";
      factor = 1024 * 1024 * 1024;
      digits = 2;
    } else if (total_size > 1024 * 1024) {
      unit = "megabyte";
      factor = 1024 * 1024;
    } else if (total_size > 1024) {
      unit = "kilobyte";
      factor = 1024;
    } else {
      unit = "byte";
      factor = 1;
    }

    let formatter = new Intl.NumberFormat(undefined, {
      style: "unit",
      unit: unit,
      unitDisplay: "short",
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    });

    const request = new XMLHttpRequest();

    request.addEventListener("load", (event) => {
      console.log("Upload finished");
      htmx.trigger("#space-refresh", "refresh-event", {});
      htmx.trigger("#upload-refresh", "refresh-event", {});

      input.value = null;
      progressElement.value = 0;
      progressText.innerHTML = "";
      progressContainer.classList.add("invisible");
      submit.disabled = true;
    });

    request.addEventListener("error", (event) => {
      console.error("Failed to upload file", event);
      setError("Encountered an error uploading the file");
    });

    request.addEventListener("abort", (event) => {
      console.warn("Upload was aborted", event);
      setError("Upload was aborted");
    });

    request.upload.addEventListener("progress", (event) => {
      const amount = Math.round(event.loaded / total_size * 100.0);
      progressElement.value = amount;
      progressText.innerHTML =
        formatter.format(event.loaded / factor) + " / " +
        formatter.format(total_size / factor);
    })

    progressContainer.classList.remove("invisible");

    console.log("Starting upload of", total_files, "files with a total size of", total_size, "bytes");
    request.open("POST", "/uploads");
    request.send(form_data);
  }
</script>
*/
