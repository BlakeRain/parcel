import { FunctionComponent, VNode } from "preact";
import { html } from "htm/preact";
import register from "preact-custom-element";
import { useState, ProvideState, StateMode } from "./upload/state";
import DropZone from "./upload/components/dropzone";
import FilesSummary from "./upload/components/summary";
import FilesList from "./upload/components/list";
import UploadProgress from "./upload/components/progress";
import { ParcelModal } from "./modal";

const UploadButtons: FunctionComponent<{ csrf_token: string }> = (props) => {
  const { state, dispatch } = useState();

  const onCancelClick = (event: MouseEvent) => {
    (event.target as HTMLElement)
      .closest<ParcelModal>("parcel-modal")
      .closeModal();
  };

  const onUploadClick = (event: MouseEvent) => {
    const modal = (event.target as HTMLElement).closest<ParcelModal>(
      "parcel-modal",
    );
    if (!modal) {
      throw new Error("Could not find parent modal");
    }

    const form = new FormData();
    form.append("csrf_token", props.csrf_token);

    for (let file of state.files) {
      form.append("file", file.file);
    }

    const upload = new XMLHttpRequest();

    upload.addEventListener("load", () => {
      modal.setUnderlayDismiss(true);
      htmx.trigger("#upload-list-refresh", "refresh");
      dispatch({ type: "complete" });
    });

    upload.addEventListener("error", (event) => {
      console.error("Failed to upload file", event);
      modal.setUnderlayDismiss(true);
      dispatch({ type: "error", event });
    });

    upload.addEventListener("abort", (event) => {
      console.warn("Upload was aborted", event);
      modal.setUnderlayDismiss(true);
      dispatch({ type: "abort", event });
    });

    upload.upload.addEventListener("progress", (event) => {
      dispatch({ type: "progress", loaded: event.loaded });
    });

    modal.setUnderlayDismiss(false);
    upload.open("POST", "/uploads/new");
    upload.send(form);
    dispatch({ type: "upload", upload });
  };

  return html`
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

const CompleteButtons: FunctionComponent = () => {
  const { dispatch } = useState();

  const onMoreClick = () => {
    dispatch({ type: "reset" });
  };

  const onCloseClick = (event: MouseEvent) => {
    (event.target as HTMLElement)
      .closest<ParcelModal>("parcel-modal")
      .closeModal();
  };

  return html`
    <div class="buttons end">
      <button type="button" class="button hollow" onclick=${onMoreClick}>
        Upload more
      </button>
      <button type="button" class="button success" onclick=${onCloseClick}>
        <span class="icon-check"></span>
        Finish
      </button>
    </div>
  `;
};

const ErrorButtons: FunctionComponent = () => {
  const onCancelClick = (event: MouseEvent) => {
    (event.target as HTMLElement)
      .closest<ParcelModal>("parcel-modal")
      .closeModal();
  };

  return html`
    <div class="buttons end">
      <button type="button" class="button hollow" onclick=${onCancelClick}>
        Cancel
      </button>
    </div>
  `;
};

const UploadBody = () => {
  const { state } = useState();

  if (state.files.length === 0) {
    return html`<div></div>`;
  }

  return html`
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
  `;
};

const UploadFormInner: FunctionComponent<{ csrf_token: string }> = (props) => {
  const { state } = useState();

  let buttons: VNode;
  switch (state.mode) {
    case StateMode.Preparing:
    case StateMode.Uploading:
      buttons = html`<${UploadButtons} ...${props} />`;
      break;

    case StateMode.Error:
    case StateMode.Aborted:
      buttons = html`<${ErrorButtons} />`;
      break;

    case StateMode.Complete:
      buttons = html`<${CompleteButtons} />`;
      break;
  }

  return html`
    <${DropZone} />
    <${UploadBody} />
    ${buttons}
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
