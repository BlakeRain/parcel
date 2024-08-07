import { html } from "../shared.js";
import register from "preact-custom-element";
import { useState, ProvideState } from "./upload/state.js";
import DropZone from "./upload/components/dropzone.js";
import FilesSummary from "./upload/components/summary.js";
import FilesList from "./upload/components/list.js";
import UploadProgress from "./upload/components/progress.js";

const UploadFormInner = (props) => {
  const { state, dispatch } = useState();

  const onCancelClick = (event) => {
    event.preventDefault();
    event.target.closest("parcel-modal").closeModal();
  };

  const onUploadClick = () => {
    const form = new FormData();
    form.append("csrf_token", props.csrf_token);

    for (let file of state.files) {
      form.append("file", file.file);
    }

    const upload = new XMLHttpRequest();

    upload.addEventListener("load", () => {
      console.log("Upload complete");
      dispatch({ type: "complete" });
    });

    upload.addEventListener("error", (event) => {
      console.error("Failed to upload file", event);
      dispatch({ type: "error", event });
    });

    upload.addEventListener("abort", (event) => {
      console.warn("Upload was aborted", event);
      dispatch({ type: "error", event });
    });

    upload.upload.addEventListener("progress", (event) => {
      dispatch({ type: "progress", loaded: event.loaded });
    });

    upload.open("POST", "/uploads/new");
    upload.send(form);
    dispatch({ type: "upload", upload });
  };

  return html`
    <${DropZone} />
    ${state.files.length > 0
      ? html`
          <div
            class="border border-gray-300 dark:border-slate-600 rounded-md flex flex-col gap-2 overflow-y-hidden"
          >
            ${state.upload
              ? html` <${UploadProgress} /> `
              : html` <${FilesSummary} /> `}
            <div class="overflow-y-scroll px-4 mb-4">
              <${FilesList} />
            </div>
          </div>
        `
      : html`<div></div>`}
    <div class="buttons end">
      <button type="button" class="button hollow" onclick=${onCancelClick}>
        Cancel
      </button>
      <button
        type="button"
        class="button"
        disabled=${state.files.length === 0 || state.upload}
        onclick=${onUploadClick}
      >
        <span class="icon-upload"></span>
        Upload file
      </button>
    </div>
  `;
};

const UploadForm = (props) => {
  return html`
    <div
      class="grid grid-rows-[max-content_1fr_max-content] max-h-[80vh] gap-4"
    >
      <${ProvideState}>
        <${UploadFormInner} ...${props} />
      </${ProvideState}>
    </div>
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
