import { FunctionComponent } from "preact";
import { html } from "htm/preact";
import register from "preact-custom-element";
import { useState, ProvideState } from "./upload/state";
import DropZone from "./upload/components/dropzone";
import FilesSummary from "./upload/components/summary";
import FilesList from "./upload/components/list";
import UploadProgress from "./upload/components/progress";
import { ParcelModal } from "./modal";

const UploadFormInner: FunctionComponent<{ csrf_token: string }> = (props) => {
  const { state, dispatch } = useState();

  const onCancelClick = (event: MouseEvent) => {
    event.preventDefault();
    (event.target as HTMLElement)
      .closest<ParcelModal>("parcel-modal")
      .closeModal();
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
      dispatch({ type: "abort", event });
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

const UploadForm: FunctionComponent<{ csrf_token: string }> = (props) => {
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
