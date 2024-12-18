import { FunctionComponent, VNode } from "preact";
import { useEffect, useRef } from "preact/hooks";
import { html } from "htm/preact";
import register from "preact-custom-element";
import { useState, ProvideState, StateMode, StateAction } from "./upload/state";
import DropZone from "./upload/components/dropzone";
import FilesSummary from "./upload/components/summary";
import FilesList from "./upload/components/list";
import UploadProgress from "./upload/components/progress";
import { ParcelModal } from "./modal";
import { FileInfo } from "./upload/files";

function startUpload(
  modal: ParcelModal,
  csrf_token: string,
  team: string | null,
  files: FileInfo[],
  dispatch: (action: StateAction) => void,
) {
  const form = new FormData();
  form.append("csrf_token", csrf_token);

  if (team) {
    form.append("team", team);
  }

  for (let file of files) {
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
}

const UploadButtons: FunctionComponent<{
  csrf_token: string;
  team?: string;
}> = (props) => {
  const { state, dispatch } = useState();

  const onCancelClick = (event: MouseEvent) => {
    if (state.upload) {
      state.upload.abort();
    } else {
      (event.target as HTMLElement)
        .closest<ParcelModal>("parcel-modal")
        .closeModal();
    }
  };

  const onUploadClick = (event: MouseEvent) => {
    const modal = (event.target as HTMLElement).closest<ParcelModal>(
      "parcel-modal",
    );
    if (!modal) {
      throw new Error("Could not find parent modal");
    }

    startUpload(
      modal,
      props.csrf_token,
      props.team || null,
      state.files,
      dispatch,
    );
  };

  return html`
    <div class="buttons end">
      <button
        type="button"
        class="button hollow ${state.upload && "danger"}"
        onclick=${onCancelClick}
      >
        <span class="icon-x"></span>
        Cancel${state.upload && " upload"}
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
        <span class="icon-rotate-ccw"></span>
        Upload more
      </button>
      <button type="button" class="button success" onclick=${onCloseClick}>
        <span class="icon-check"></span>
        Finish
      </button>
    </div>
  `;
};

const ErrorButtons: FunctionComponent<{ csrf_token: string; team?: string }> = (
  props,
) => {
  const { state, dispatch } = useState();

  const onCancelClick = (event: MouseEvent) => {
    (event.target as HTMLElement)
      .closest<ParcelModal>("parcel-modal")
      .closeModal();
  };

  const onResetClick = () => {
    dispatch({ type: "reset" });
  };

  const onRetryClick = (event: MouseEvent) => {
    const modal = (event.target as HTMLElement).closest<ParcelModal>(
      "parcel-modal",
    );
    if (!modal) {
      throw new Error("Could not find parent modal");
    }

    startUpload(
      modal,
      props.csrf_token,
      props.team || null,
      state.files,
      dispatch,
    );
  };

  return html`
    <div class="buttons end">
      <button
        type="button"
        class="button hollow danger"
        onclick=${onCancelClick}
      >
        <span class="icon-x"></span>
        Cancel
      </button>
      <button type="button" class="button hollow" onclick=${onResetClick}>
        <span class="icon-rotate-ccw"></span>
        Reset
      </button>
      <button type="button" class="button" onclick=${onRetryClick}>
        <span class="icon-upload"></span>
        Try again
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

const UploadFormInner: FunctionComponent<{
  csrf_token: string;
  team?: string;
}> = (props) => {
  const eventRecv = useRef<HTMLElement>(null);
  const { state, dispatch } = useState();

  let buttons: VNode;
  switch (state.mode) {
    case StateMode.Preparing:
    case StateMode.Uploading:
      buttons = html`<${UploadButtons} ...${props} />`;
      break;

    case StateMode.Error:
    case StateMode.Aborted:
      buttons = html`<${ErrorButtons} ...${props} />`;
      break;

    case StateMode.Complete:
      buttons = html`<${CompleteButtons} />`;
      break;
  }

  useEffect(() => {
    if (!eventRecv.current) {
      return;
    }

    const element = eventRecv.current;
    const onParcelDrop = (event: CustomEvent<{ files: File[] }>) => {
      dispatch({
        type: "add",
        files: event.detail.files,
      });
    };

    element.addEventListener("parcelDrop", onParcelDrop);

    return () => {
      element.removeEventListener("parcelDrop", onParcelDrop);
    };
  });

  return html`
    <div ref=${eventRecv} class="invisible event-receiver"></div>
    <div
      class="grid grid-rows-[max-content_1fr_max-content] max-h-[80vh] gap-4"
    >
      <${DropZone} />
      <${UploadBody} />
      ${buttons}
    </div>
  `;
};

const UploadForm: FunctionComponent<{ csrf_token: string; team?: string }> = (
  props,
) => {
  return html`
      <${ProvideState}>
        <${UploadFormInner} ...${props} />
      </${ProvideState}>
  `;
};

register(UploadForm, "parcel-upload-form", ["csrf_token", "team"]);
